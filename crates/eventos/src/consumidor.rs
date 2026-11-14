//! A consume loop: hands each delivery of one subscription to its handler and settles it with the
//! delivery policy (ADR-0018 § 4, § 6).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use puerto_nucleo::eventos::entrega::{
    ENTREGAS_MAXIMAS_POR_DEFECTO, Entrega, decidir, espera_para,
};
use puerto_nucleo::eventos::transporte::{
    DestinoDescartados, ErrorTransporte, RegistroDescartado, Reservado, SuscripcionEventos,
};
use puerto_nucleo::eventos::{ManejadorEvento, ResultadoManejo};

use crate::conductor::{Pasada, Progreso};

/// How many deliveries one pass claims when nothing else is configured.
pub const TAMANO_LOTE_CONSUMIDOR_POR_DEFECTO: u32 = 10;

/// One `[[consumidor]]`: its subscription, its handler and where its dead letters go.
pub struct Consumidor<S> {
    suscripcion: S,
    tamano_lote: u32,
    policy: Policy,
}

/// Everything but the subscription, which is only `Send`: what a delivery needs while it waits
/// on the handler and the sink.
struct Policy {
    handler: Arc<dyn ManejadorEvento>,
    dead_letters: Arc<dyn DestinoDescartados>,
    max_deliveries: u32,
    espera_maxima_reintento: Option<Duration>,
}

impl<S: SuscripcionEventos> Consumidor<S> {
    pub fn new(
        suscripcion: S,
        handler: impl ManejadorEvento + 'static,
        dead_letters: Arc<dyn DestinoDescartados>,
    ) -> Self {
        Self {
            tamano_lote: TAMANO_LOTE_CONSUMIDOR_POR_DEFECTO
                .min(suscripcion.lote_maximo())
                .max(1),
            suscripcion,
            policy: Policy {
                handler: Arc::new(handler),
                dead_letters,
                max_deliveries: ENTREGAS_MAXIMAS_POR_DEFECTO,
                espera_maxima_reintento: None,
            },
        }
    }

    /// Never more than the subscription hands out per batch (`max_batch`).
    pub fn con_tamano_lote(mut self, tamano_lote: u32) -> Self {
        self.tamano_lote = tamano_lote.max(1).min(self.suscripcion.lote_maximo());
        self
    }

    /// How many deliveries an event gets before it is dead-lettered.
    pub fn con_entregas_maximas(mut self, max_deliveries: u32) -> Self {
        self.policy.max_deliveries = max_deliveries.max(1);
        self
    }

    /// Caps the policy's backoff between redeliveries — `Some(Duration::ZERO)` retries at the
    /// next pass. For tests and development; production keeps the policy's.
    pub fn con_espera_maxima_reintento(
        mut self,
        espera_maxima_reintento: Option<Duration>,
    ) -> Self {
        self.policy.espera_maxima_reintento = espera_maxima_reintento;
        self
    }

    /// One batch. A claim left unsettled (the pass failed before it) is delivered again once its
    /// lease runs out: at-least-once, never lost.
    pub async fn consumir_una_vez(&mut self) -> Result<Progreso, ErrorTransporte> {
        let batch = self.suscripcion.siguiente_lote(self.tamano_lote).await?;
        let full = batch.len() as u64 >= u64::from(self.tamano_lote);
        let consumidor = self.suscripcion.consumidor().to_string();
        for claimed in &batch {
            let delivery = self.policy.deliver(&consumidor, claimed).await;
            self.suscripcion.resolver(claimed, &delivery).await?;
        }
        Ok(if full { Progreso::More } else { Progreso::Idle })
    }
}

impl Policy {
    /// What to settle `claimed` with. A dead letter is recorded before it is answered; when it
    /// cannot be, the delivery is retried instead, so nothing is retired without one.
    async fn deliver(&self, consumidor: &str, claimed: &Reservado) -> Entrega {
        let event = &claimed.sobre;
        let outcome = self.handle(consumidor, claimed).await;
        match decidir(outcome, claimed.intento, self.max_deliveries) {
            Entrega::Ack => Entrega::Ack,
            Entrega::Reintentar { espera } => {
                tracing::warn!(consumidor, event_id = %event.id, attempt = claimed.intento, "consumidor: reintento");
                Entrega::Reintentar {
                    espera: self.cap(espera),
                }
            }
            Entrega::Descartar { motivo, detalle } => {
                let record = RegistroDescartado::para_evento(
                    consumidor,
                    event,
                    claimed.intento,
                    motivo,
                    detalle.clone(),
                );
                match self.dead_letters.registrar(&record).await {
                    Ok(()) => Entrega::Descartar { motivo, detalle },
                    Err(err) => {
                        tracing::error!(consumidor, event_id = %event.id, error = %err, "consumidor: el descartado no se registró, se reintenta");
                        Entrega::Reintentar {
                            espera: self.cap(espera_para(claimed.intento)),
                        }
                    }
                }
            }
        }
    }

    /// The handler's answer. It runs on its own task, so a panic is caught: it is a failure that
    /// may pass (`Retry`), and the loop goes on.
    async fn handle(&self, consumidor: &str, claimed: &Reservado) -> ResultadoManejo {
        let handler = Arc::clone(&self.handler);
        let event = claimed.sobre.clone();
        match tokio::spawn(async move { handler.manejar(&event).await }).await {
            Ok(outcome) => outcome,
            Err(err) => {
                tracing::error!(
                    consumidor,
                    event_id = %claimed.sobre.id,
                    error = %err,
                    "consumidor: el manejador entró en pánico"
                );
                ResultadoManejo::Reintentar
            }
        }
    }

    fn cap(&self, delay: Duration) -> Duration {
        self.espera_maxima_reintento
            .map_or(delay, |max| delay.min(max))
    }
}

#[async_trait]
impl<S: SuscripcionEventos + 'static> Pasada for Consumidor<S> {
    fn name(&self) -> String {
        format!("consumidor {}", self.suscripcion.consumidor())
    }

    async fn pass(&mut self) -> Result<Progreso, ErrorTransporte> {
        self.consumir_una_vez().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pruebas::{DestinoFalso, Guion, SuscripcionFalsa, envelope};
    use puerto_nucleo::eventos::{MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS, MOTIVO_PAYLOAD_INVALIDO};

    fn consumidor(
        due: Vec<Reservado>,
        handler: Guion,
        sink: &DestinoFalso,
    ) -> (Consumidor<SuscripcionFalsa>, SuscripcionFalsa) {
        let suscripcion = SuscripcionFalsa::with(due);
        let consumidor = Consumidor::new(suscripcion.clone(), handler, Arc::new(sink.clone()));
        (consumidor, suscripcion)
    }

    #[tokio::test]
    async fn should_settle_ack_when_the_handler_acks() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 1);
        let sink = DestinoFalso::default();
        let (mut consumidor, suscripcion) = consumidor(vec![claimed.clone()], Guion::Ack, &sink);

        // Act
        let result = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        assert_eq!(
            suscripcion.resueltos(),
            vec![(claimed.sobre.id, Entrega::Ack)]
        );
        assert!(sink.registros().is_empty());
    }

    #[tokio::test]
    async fn should_settle_a_retry_with_the_backoff_when_below_the_cap() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 2);
        let sink = DestinoFalso::default();
        let (mut consumidor, suscripcion) = consumidor(vec![claimed.clone()], Guion::Retry, &sink);

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(
            suscripcion.resueltos(),
            vec![(
                claimed.sobre.id,
                Entrega::Reintentar {
                    espera: Duration::from_secs(5)
                }
            )]
        );
        assert!(sink.registros().is_empty());
    }

    #[tokio::test]
    async fn should_dead_letter_when_the_retry_reaches_the_cap() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 5);
        let sink = DestinoFalso::default();
        let (mut consumidor, suscripcion) = consumidor(vec![claimed.clone()], Guion::Retry, &sink);

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert
        let registros = sink.registros();
        assert_eq!(registros.len(), 1);
        assert_eq!(registros[0].id_evento, Some(claimed.sobre.id));
        assert_eq!(registros[0].consumidor, "notify_new_product");
        assert_eq!(registros[0].intentos, 5);
        assert_eq!(registros[0].motivo, MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS);
        assert!(matches!(
            suscripcion.resueltos()[0].1,
            Entrega::Descartar {
                motivo: MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn should_record_the_dead_letter_before_settling_when_the_handler_gives_up() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 1);
        let sink = DestinoFalso::default();
        let suscripcion = SuscripcionFalsa::with(vec![claimed.clone()]).observando(&sink);
        let mut consumidor =
            Consumidor::new(suscripcion.clone(), Guion::DecodeAs, Arc::new(sink.clone()));

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert — the payload is not a number: the handler dead-letters it as invalid
        let registros = sink.registros();
        assert_eq!(registros.len(), 1);
        assert_eq!(registros[0].motivo, MOTIVO_PAYLOAD_INVALIDO);
        assert_eq!(
            suscripcion.descartados_vistos_al_resolver(),
            vec![1],
            "the dead letter was not recorded before the claim was settled"
        );
        assert!(matches!(
            suscripcion.resueltos()[0].1,
            Entrega::Descartar {
                motivo: MOTIVO_PAYLOAD_INVALIDO,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn should_retry_instead_of_retiring_when_the_dead_letter_cannot_be_recorded() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 5);
        let sink = DestinoFalso::fallando();
        let (mut consumidor, suscripcion) = consumidor(vec![claimed.clone()], Guion::Retry, &sink);

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(
            suscripcion.resueltos(),
            vec![(
                claimed.sobre.id,
                Entrega::Reintentar {
                    espera: Duration::from_secs(300)
                }
            )]
        );
    }

    #[tokio::test]
    async fn should_treat_a_panic_as_a_retry_and_go_on_when_the_handler_panics() {
        // Arrange
        let first = Reservado::new(envelope("product_created"), 1);
        let second = Reservado::new(envelope("product_created"), 1);
        let sink = DestinoFalso::default();
        let (mut consumidor, suscripcion) = consumidor(
            vec![first.clone(), second.clone()],
            Guion::PanicOnce(Default::default()),
            &sink,
        );

        // Act
        let result = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        assert_eq!(
            suscripcion.resueltos(),
            vec![
                (
                    first.sobre.id,
                    Entrega::Reintentar {
                        espera: Duration::from_secs(1)
                    }
                ),
                (second.sobre.id, Entrega::Ack),
            ]
        );
    }

    #[tokio::test]
    async fn should_cap_the_retry_delay_when_a_maximum_is_configured() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 4);
        let sink = DestinoFalso::default();
        let (consumidor, suscripcion) = consumidor(vec![claimed.clone()], Guion::Retry, &sink);
        let mut consumidor =
            consumidor.con_espera_maxima_reintento(Some(Duration::from_millis(10)));

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(
            suscripcion.resueltos()[0].1,
            Entrega::Reintentar {
                espera: Duration::from_millis(10)
            }
        );
    }

    #[tokio::test]
    async fn should_dead_letter_earlier_when_fewer_deliveries_are_configured() {
        // Arrange
        let claimed = Reservado::new(envelope("product_created"), 2);
        let sink = DestinoFalso::default();
        let (consumidor, _) = consumidor(vec![claimed], Guion::Retry, &sink);
        let mut consumidor = consumidor.con_entregas_maximas(2);

        // Act
        let _ = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(sink.registros().len(), 1);
    }

    #[tokio::test]
    async fn should_ask_for_more_at_once_when_the_batch_was_full() {
        // Arrange
        let due = (0..3)
            .map(|_| Reservado::new(envelope("product_created"), 1))
            .collect();
        let sink = DestinoFalso::default();
        let (consumidor, suscripcion) = consumidor(due, Guion::Ack, &sink);
        let mut consumidor = consumidor.con_tamano_lote(2);

        // Act
        let first = consumidor.consumir_una_vez().await;
        let second = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!((first, second), (Ok(Progreso::More), Ok(Progreso::Idle)));
        assert_eq!(suscripcion.resueltos().len(), 3);
    }

    #[tokio::test]
    async fn should_ask_for_more_at_once_when_the_subscription_hands_out_one_claim_at_a_time() {
        // Arrange
        let due = (0..3)
            .map(|_| Reservado::new(envelope("product_created"), 1))
            .collect();
        let sink = DestinoFalso::default();
        let suscripcion = SuscripcionFalsa::with(due).de_uno_en_uno();
        let mut consumidor =
            Consumidor::new(suscripcion.clone(), Guion::Ack, Arc::new(sink.clone()));

        // Act
        let first = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(first, Ok(Progreso::More));
        assert_eq!(suscripcion.resueltos().len(), 1);
    }

    #[test]
    fn should_never_claim_more_than_the_subscription_hands_out() {
        let consumidor = Consumidor::new(
            SuscripcionFalsa::with(vec![]).de_uno_en_uno(),
            Guion::Ack,
            Arc::new(DestinoFalso::default()),
        )
        .con_tamano_lote(50);
        assert_eq!(consumidor.tamano_lote, 1);
    }

    #[tokio::test]
    async fn should_stop_the_pass_when_a_claim_cannot_be_settled() {
        // Arrange
        let due = (0..2)
            .map(|_| Reservado::new(envelope("product_created"), 1))
            .collect();
        let sink = DestinoFalso::default();
        let suscripcion = SuscripcionFalsa::with(due).fallando_al_resolver();
        let mut consumidor =
            Consumidor::new(suscripcion.clone(), Guion::Ack, Arc::new(sink.clone()));

        // Act
        let result = consumidor.consumir_una_vez().await;

        // Assert
        assert_eq!(result, Err(ErrorTransporte::NoDisponible));
        assert_eq!(suscripcion.llamadas_resolver(), 1);
    }

    #[test]
    fn should_deliver_at_least_once_when_the_cap_is_zero() {
        let consumidor = Consumidor::new(
            SuscripcionFalsa::with(vec![]),
            Guion::Ack,
            Arc::new(DestinoFalso::default()),
        )
        .con_entregas_maximas(0)
        .con_tamano_lote(0);
        assert_eq!(
            (consumidor.policy.max_deliveries, consumidor.tamano_lote),
            (1, 1)
        );
    }
}
