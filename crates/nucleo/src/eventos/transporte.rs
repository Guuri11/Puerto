//! The ports the worker drives (ADR-0018 § 4). A transport (`db`, `kafka`) is an adapter crate
//! implementing them; the relay and the consume loop in `puerto-eventos` only see these traits.
//!
//! ```text
//! Outbox ──fetch──▶ relay ──publish──▶ PublicadorEventos      (relay: one process holds the lead)
//! SuscripcionEventos ──next_batch──▶ consumer ──handle──▶ ManejadorEvento
//!                   ◀──settle(decide(..))──  └─dead letter─▶ DestinoDescartados (before settling)
//! ```

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::SobreEvento;
use super::entrega::Entrega;

/// How much of a payload a dead letter keeps, in characters.
pub const LIMITE_PAYLOAD_DESCARTADO: usize = 4000;

/// What a transport port fails with. The adapter logs the driver error with its context; the
/// loop that called it only needs to know to try again later.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErrorTransporte {
    /// The store or the broker failed or could not be reached.
    #[error("transporte.no_disponible")]
    NoDisponible,
    /// What was stored cannot be read back as an envelope.
    #[error("transporte.datos_invalidos")]
    DatosInvalidos,
}

/// The relay's input: the events written by the API's units of work, not yet published.
#[async_trait]
pub trait Outbox: Send + Sync {
    /// Whether this process may relay now. Only one relay runs at a time, so the outbox order
    /// is kept with several worker replicas (a Postgres advisory lock). It is asked before
    /// every batch; once held, it keeps answering `true`.
    async fn intentar_liderar(&self) -> Result<bool, ErrorTransporte>;
    /// Up to `limite` unpublished events, oldest first (the order they were written).
    async fn buscar_sin_publicar(&self, limite: u32) -> Result<Vec<SobreEvento>, ErrorTransporte>;
    /// Marks these events published. Idempotent: an id already marked is left as it is.
    async fn marcar_publicados(&self, ids: &[Uuid]) -> Result<(), ErrorTransporte>;
}

/// The relay's output: hands one event to the transport. `Ok` means the transport has it
/// durably — the relay marks it published only then, and stops at the first `Err` so the next
/// batch starts from that event again (order kept).
#[async_trait]
pub trait PublicadorEventos: Send + Sync {
    async fn publicar(&self, event: &SobreEvento) -> Result<(), ErrorTransporte>;
}

/// One event handed to one consumer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Reservado {
    pub sobre: SobreEvento,
    /// The 1-based number of this delivery to this consumer — what `decidir` reads.
    pub intento: u32,
}

impl Reservado {
    pub fn new(sobre: SobreEvento, intento: u32) -> Self {
        Self { sobre, intento }
    }
}

/// A consumer's input. The consume loop asks for a batch, hands each event to the handler and
/// settles it with what `decidir` answered; an unsettled claim is delivered again later.
#[async_trait]
pub trait SuscripcionEventos: Send {
    /// The consumer it delivers to — the `[[consumidor]] nombre`.
    fn consumidor(&self) -> &str;
    /// The most claims one batch hands out, whatever `siguiente_lote` is asked for. A transport that
    /// keeps an order (Kafka: a partition) answers 1, so a `Reintentar` holds back what comes after
    /// it instead of letting the handler see it first.
    fn lote_maximo(&self) -> u32 {
        u32::MAX
    }
    /// Up to `max` events due for this consumer. Empty when there is nothing to do.
    async fn siguiente_lote(&mut self, max: u32) -> Result<Vec<Reservado>, ErrorTransporte>;
    /// Ack → never again; `Reintentar { espera }` → again after `espera`; `Descartar` → never again
    /// (the dead letter is recorded before this is called).
    async fn resolver(
        &mut self,
        reservado: &Reservado,
        entrega: &Entrega,
    ) -> Result<(), ErrorTransporte>;
}

/// What is kept of an event no consumer will see again: nothing is retired without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistroDescartado {
    /// `None` when what arrived was not an envelope at all.
    pub id_evento: Option<Uuid>,
    pub consumidor: String,
    pub tipo_evento: Option<String>,
    pub intentos: u32,
    pub motivo: String,
    pub detalle: String,
    /// At most [`LIMITE_PAYLOAD_DESCARTADO`] characters.
    pub payload: String,
}

impl RegistroDescartado {
    pub fn para_evento(
        consumidor: impl Into<String>,
        event: &SobreEvento,
        intentos: u32,
        motivo: &str,
        detalle: impl Into<String>,
    ) -> Self {
        Self {
            id_evento: Some(event.id),
            consumidor: consumidor.into(),
            tipo_evento: Some(event.tipo_evento.clone()),
            intentos,
            motivo: motivo.to_string(),
            detalle: detalle.into(),
            payload: trim_payload(&event.payload.to_string()),
        }
    }

    /// A message that could not be read as an envelope (a poisoned Kafka record).
    pub fn ilegible(
        consumidor: impl Into<String>,
        raw: &str,
        intentos: u32,
        motivo: &str,
        detalle: impl Into<String>,
    ) -> Self {
        Self {
            id_evento: None,
            consumidor: consumidor.into(),
            tipo_evento: None,
            intentos,
            motivo: motivo.to_string(),
            detalle: detalle.into(),
            payload: trim_payload(raw),
        }
    }
}

fn trim_payload(payload: &str) -> String {
    payload.chars().take(LIMITE_PAYLOAD_DESCARTADO).collect()
}

/// Where dead letters go — Postgres, whatever the transport. Idempotent on
/// `(id_evento, consumidor)`: recording the same one twice keeps one.
#[async_trait]
pub trait DestinoDescartados: Send + Sync {
    async fn registrar(&self, registrar: &RegistroDescartado) -> Result<(), ErrorTransporte>;
}

#[cfg(test)]
mod tests {
    //! A fake of each port, driven the way the relay and the consume loop will drive them: the
    //! signatures have to fit those loops, not only compile.
    use super::*;
    use crate::eventos::entrega::{ENTREGAS_MAXIMAS_POR_DEFECTO, decidir};
    use crate::eventos::{
        EventoDominio, MOTIVO_PAYLOAD_INVALIDO, ManejadorEvento, ResultadoManejo,
    };
    use serde_json::{Value, json};
    use std::sync::Mutex;
    use std::time::Duration;

    struct Created(Uuid);

    impl EventoDominio for Created {
        fn tipo_evento(&self) -> &'static str {
            "product_created"
        }
        fn tipo_agregado(&self) -> &'static str {
            "product"
        }
        fn id_agregado(&self) -> Uuid {
            self.0
        }
        fn version_esquema(&self) -> u16 {
            1
        }
        fn payload(&self) -> Value {
            json!({ "id": self.0, "name": "Desk lamp" })
        }
    }

    fn sobre() -> SobreEvento {
        SobreEvento::new(&Created(Uuid::now_v7()))
    }

    #[derive(Default)]
    struct FakeOutbox {
        rows: Mutex<Vec<(SobreEvento, bool)>>,
    }

    #[async_trait]
    impl Outbox for FakeOutbox {
        async fn intentar_liderar(&self) -> Result<bool, ErrorTransporte> {
            Ok(true)
        }
        async fn buscar_sin_publicar(
            &self,
            limite: u32,
        ) -> Result<Vec<SobreEvento>, ErrorTransporte> {
            let rows = self
                .rows
                .lock()
                .map_err(|_| ErrorTransporte::NoDisponible)?;
            Ok(rows
                .iter()
                .filter(|(_, published)| !published)
                .take(limite as usize)
                .map(|(event, _)| event.clone())
                .collect())
        }
        async fn marcar_publicados(&self, ids: &[Uuid]) -> Result<(), ErrorTransporte> {
            let mut rows = self
                .rows
                .lock()
                .map_err(|_| ErrorTransporte::NoDisponible)?;
            for (event, published) in rows.iter_mut() {
                *published |= ids.contains(&event.id);
            }
            Ok(())
        }
    }

    /// Accepts `capacity` events, then fails.
    struct FakePublisher {
        capacity: usize,
        published: Mutex<Vec<Uuid>>,
    }

    #[async_trait]
    impl PublicadorEventos for FakePublisher {
        async fn publicar(&self, event: &SobreEvento) -> Result<(), ErrorTransporte> {
            let mut published = self
                .published
                .lock()
                .map_err(|_| ErrorTransporte::NoDisponible)?;
            if published.len() == self.capacity {
                return Err(ErrorTransporte::NoDisponible);
            }
            published.push(event.id);
            Ok(())
        }
    }

    struct FakeSubscription {
        due: Vec<Reservado>,
        settled: Vec<(Uuid, Entrega)>,
    }

    #[async_trait]
    impl SuscripcionEventos for FakeSubscription {
        fn consumidor(&self) -> &str {
            "notify_new_product"
        }
        async fn siguiente_lote(&mut self, max: u32) -> Result<Vec<Reservado>, ErrorTransporte> {
            let take = self.due.len().min(max as usize);
            Ok(self.due.drain(..take).collect())
        }
        async fn resolver(
            &mut self,
            reservado: &Reservado,
            entrega: &Entrega,
        ) -> Result<(), ErrorTransporte> {
            self.settled.push((reservado.sobre.id, entrega.clone()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeSink {
        records: Mutex<Vec<RegistroDescartado>>,
    }

    #[async_trait]
    impl DestinoDescartados for FakeSink {
        async fn registrar(&self, registrar: &RegistroDescartado) -> Result<(), ErrorTransporte> {
            let mut records = self
                .records
                .lock()
                .map_err(|_| ErrorTransporte::NoDisponible)?;
            let seen = records.iter().any(|r| {
                r.id_evento == registrar.id_evento && r.consumidor == registrar.consumidor
            });
            if !seen {
                records.push(registrar.clone());
            }
            Ok(())
        }
    }

    /// Acks a payload with a `nombre`, dead-letters the rest, retries nothing.
    struct NeedsName;

    #[async_trait]
    impl ManejadorEvento for NeedsName {
        async fn manejar(&self, event: &SobreEvento) -> ResultadoManejo {
            match event.payload.get("name") {
                Some(_) => ResultadoManejo::Ack,
                None => ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, "missing `nombre`"),
            }
        }
    }

    #[test]
    fn should_hand_out_whole_batches_unless_a_transport_keeps_an_order() {
        let subscription = FakeSubscription {
            due: vec![],
            settled: vec![],
        };
        assert_eq!(subscription.lote_maximo(), u32::MAX);
    }

    #[tokio::test]
    async fn should_mark_only_the_confirmed_prefix_when_the_publisher_fails_midway() {
        // Arrange
        let eventos: Vec<SobreEvento> = (0..3).map(|_| sobre()).collect();
        let outbox = FakeOutbox {
            rows: Mutex::new(eventos.iter().cloned().map(|e| (e, false)).collect()),
        };
        let publisher = FakePublisher {
            capacity: 2,
            published: Mutex::new(Vec::new()),
        };

        // Act — one relay pass
        assert_eq!(outbox.intentar_liderar().await, Ok(true));
        let mut confirmed = Vec::new();
        for event in outbox.buscar_sin_publicar(10).await.unwrap_or_default() {
            if publisher.publicar(&event).await.is_err() {
                break;
            }
            confirmed.push(event.id);
        }
        let marked = outbox.marcar_publicados(&confirmed).await;

        // Assert
        assert_eq!(marked, Ok(()));
        let left = outbox.buscar_sin_publicar(10).await.unwrap_or_default();
        assert_eq!(left, vec![eventos[2].clone()]);
    }

    #[tokio::test]
    async fn should_leave_marked_events_marked_when_marked_again() {
        let event = sobre();
        let outbox = FakeOutbox {
            rows: Mutex::new(vec![(event.clone(), false)]),
        };

        let first = outbox.marcar_publicados(&[event.id]).await;
        let second = outbox.marcar_publicados(&[event.id]).await;

        assert_eq!((first, second), (Ok(()), Ok(())));
        assert_eq!(outbox.buscar_sin_publicar(10).await, Ok(vec![]));
    }

    #[tokio::test]
    async fn should_settle_every_claim_and_dead_letter_first_when_driven_by_a_consume_pass() {
        // Arrange
        let good = sobre();
        let mut poisoned = sobre();
        poisoned.payload = json!({ "id": poisoned.id_agregado });
        let mut subscription = FakeSubscription {
            due: vec![
                Reservado::new(good.clone(), 1),
                Reservado::new(poisoned.clone(), 2),
            ],
            settled: Vec::new(),
        };
        let sink = FakeSink::default();

        // Act — one consume pass
        for reservado in subscription.siguiente_lote(10).await.unwrap_or_default() {
            let outcome = NeedsName.manejar(&reservado.sobre).await;
            let entrega = decidir(outcome, reservado.intento, ENTREGAS_MAXIMAS_POR_DEFECTO);
            if let Entrega::Descartar { motivo, detalle } = &entrega {
                let registrar = RegistroDescartado::para_evento(
                    subscription.consumidor(),
                    &reservado.sobre,
                    reservado.intento,
                    motivo,
                    detalle.clone(),
                );
                assert_eq!(sink.registrar(&registrar).await, Ok(()));
            }
            assert_eq!(subscription.resolver(&reservado, &entrega).await, Ok(()));
        }

        // Assert
        assert_eq!(
            subscription.settled,
            vec![
                (good.id, Entrega::Ack),
                (
                    poisoned.id,
                    Entrega::Descartar {
                        motivo: "payload_invalido",
                        detalle: "missing `nombre`".into()
                    }
                ),
            ]
        );
        let records = sink.records.lock().map(|r| r.clone()).unwrap_or_default();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id_evento, Some(poisoned.id));
        assert_eq!(records[0].consumidor, "notify_new_product");
        assert_eq!(records[0].intentos, 2);
    }

    #[tokio::test]
    async fn should_hand_out_at_most_the_batch_size_when_more_are_due() {
        let mut subscription = FakeSubscription {
            due: (0..3).map(|_| Reservado::new(sobre(), 1)).collect(),
            settled: Vec::new(),
        };

        let first = subscription.siguiente_lote(2).await.unwrap_or_default();
        let second = subscription.siguiente_lote(2).await.unwrap_or_default();

        assert_eq!((first.len(), second.len()), (2, 1));
    }

    #[tokio::test]
    async fn should_keep_one_dead_letter_when_the_same_one_is_recorded_twice() {
        let sink = FakeSink::default();
        let registrar =
            RegistroDescartado::para_evento("audit", &sobre(), 5, "error_permanente", "no");

        let _ = sink.registrar(&registrar).await;
        let _ = sink.registrar(&registrar).await;

        assert_eq!(sink.records.lock().map(|r| r.len()).unwrap_or(0), 1);
    }

    #[test]
    fn should_describe_the_event_when_dead_lettering_an_envelope() {
        let event = sobre();

        let registrar = RegistroDescartado::para_evento(
            "audit",
            &event,
            5,
            "entregas_maximas_superadas",
            "timeout",
        );

        assert_eq!(
            registrar,
            RegistroDescartado {
                id_evento: Some(event.id),
                consumidor: "audit".into(),
                tipo_evento: Some("product_created".into()),
                intentos: 5,
                motivo: "entregas_maximas_superadas".into(),
                detalle: "timeout".into(),
                payload: event.payload.to_string(),
            }
        );
    }

    #[test]
    fn should_keep_the_raw_message_when_dead_lettering_one_that_is_not_an_envelope() {
        let registrar =
            RegistroDescartado::ilegible("audit", "not json", 1, "payload_invalido", "eof");

        assert_eq!(registrar.id_evento, None);
        assert_eq!(registrar.tipo_evento, None);
        assert_eq!(registrar.payload, "not json");
        assert_eq!(registrar.motivo, "payload_invalido");
    }

    #[test]
    fn should_trim_the_payload_when_it_is_longer_than_the_limit() {
        let at_limit = "é".repeat(LIMITE_PAYLOAD_DESCARTADO);
        let over = format!("{at_limit}tail");

        let kept = RegistroDescartado::ilegible("audit", &at_limit, 1, "payload_invalido", "");
        let trimmed = RegistroDescartado::ilegible("audit", &over, 1, "payload_invalido", "");

        assert_eq!(kept.payload, at_limit);
        assert_eq!(trimmed.payload, at_limit);
    }

    #[test]
    fn should_speak_machine_readable_codes_when_a_transport_fails() {
        assert_eq!(
            ErrorTransporte::NoDisponible.to_string(),
            "transporte.no_disponible"
        );
        assert_eq!(
            ErrorTransporte::DatosInvalidos.to_string(),
            "transporte.datos_invalidos"
        );
    }

    #[test]
    fn should_keep_the_retry_delay_when_a_claim_is_settled_for_later() {
        let entrega = decidir(ResultadoManejo::Reintentar, 2, ENTREGAS_MAXIMAS_POR_DEFECTO);
        assert_eq!(
            entrega,
            Entrega::Reintentar {
                espera: Duration::from_secs(5)
            }
        );
    }
}
