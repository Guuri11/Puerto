//! The relay: moves what the API's units of work wrote to the outbox onto the transport, in the
//! order it was written (ADR-0018 § 5).

use async_trait::async_trait;
use puerto_nucleo::eventos::transporte::{ErrorTransporte, Outbox, PublicadorEventos};

use crate::conductor::{Pasada, Progreso};

/// How many outbox rows one pass reads when nothing else is configured.
pub const TAMANO_LOTE_RELAY_POR_DEFECTO: u32 = 100;

/// Reads unpublished events from `O` and hands them to `P`. Only the replica holding the lead
/// relays (`Outbox::try_lead`); the others keep asking.
pub struct Relay<O, P> {
    outbox: O,
    publicador: P,
    tamano_lote: u32,
}

impl<O: Outbox, P: PublicadorEventos> Relay<O, P> {
    pub fn new(outbox: O, publicador: P) -> Self {
        Self {
            outbox,
            publicador,
            tamano_lote: TAMANO_LOTE_RELAY_POR_DEFECTO,
        }
    }

    pub fn con_tamano_lote(mut self, tamano_lote: u32) -> Self {
        self.tamano_lote = tamano_lote.max(1);
        self
    }

    /// One batch: publishes in outbox order and stops at the first failure, so the next pass
    /// starts from that event again. Only what the publisher confirmed is marked published.
    pub async fn relay_una_vez(&self) -> Result<Progreso, ErrorTransporte> {
        if !self.outbox.intentar_liderar().await? {
            return Ok(Progreso::Idle);
        }
        let events = self.outbox.buscar_sin_publicar(self.tamano_lote).await?;
        let mut confirmed = Vec::with_capacity(events.len());
        let mut failure = None;
        for event in &events {
            match self.publicador.publicar(event).await {
                Ok(()) => confirmed.push(event.id),
                Err(err) => {
                    tracing::warn!(error = %err, event_id = %event.id, "relay: falló la publicación");
                    failure = Some(err);
                    break;
                }
            }
        }
        if !confirmed.is_empty() {
            self.outbox.marcar_publicados(&confirmed).await?;
            tracing::debug!(count = confirmed.len(), "relay: publicado");
        }
        match failure {
            Some(err) => Err(err),
            // A full batch: more may be waiting, so the next pass runs at once.
            None if events.len() as u64 >= u64::from(self.tamano_lote) => Ok(Progreso::More),
            None => Ok(Progreso::Idle),
        }
    }
}

#[async_trait]
impl<O: Outbox + 'static, P: PublicadorEventos + 'static> Pasada for Relay<O, P> {
    fn name(&self) -> String {
        "relay".to_string()
    }

    async fn pass(&mut self) -> Result<Progreso, ErrorTransporte> {
        self.relay_una_vez().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pruebas::{OutboxFalso, PublicadorFalso, envelope};

    #[tokio::test]
    async fn should_publish_in_order_and_mark_everything_when_the_publisher_accepts_all() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = OutboxFalso::with(&events);
        let publicador = PublicadorFalso::aceptando(usize::MAX);
        let relay = Relay::new(outbox.clone(), publicador.clone());

        // Act
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        let ids: Vec<_> = events.iter().map(|e| e.id).collect();
        assert_eq!(publicador.publicados(), ids);
        assert_eq!(outbox.marcados(), ids);
        assert!(outbox.sin_publicar().is_empty());
    }

    #[tokio::test]
    async fn should_mark_only_the_confirmed_prefix_when_the_publisher_fails_midway() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = OutboxFalso::with(&events);
        let publicador = PublicadorFalso::aceptando(1);
        let relay = Relay::new(outbox.clone(), publicador.clone());

        // Act
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Err(ErrorTransporte::NoDisponible));
        assert_eq!(publicador.publicados(), vec![events[0].id]);
        assert_eq!(outbox.marcados(), vec![events[0].id]);
        assert_eq!(outbox.sin_publicar(), vec![events[1].id, events[2].id]);
    }

    #[tokio::test]
    async fn should_resume_from_the_failed_event_when_the_next_pass_runs() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = OutboxFalso::with(&events);
        let publicador = PublicadorFalso::aceptando(1);
        let relay = Relay::new(outbox.clone(), publicador.clone());
        let _ = relay.relay_una_vez().await;

        // Act
        publicador.aceptar(usize::MAX);
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        let ids: Vec<_> = events.iter().map(|e| e.id).collect();
        assert_eq!(publicador.publicados(), ids);
    }

    #[tokio::test]
    async fn should_do_nothing_when_the_outbox_is_empty() {
        // Arrange
        let outbox = OutboxFalso::with(&[]);
        let publicador = PublicadorFalso::aceptando(usize::MAX);
        let relay = Relay::new(outbox.clone(), publicador.clone());

        // Act
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        assert!(publicador.publicados().is_empty());
        assert_eq!(outbox.llamadas_marcar(), 0);
    }

    #[tokio::test]
    async fn should_publish_nothing_when_another_replica_leads() {
        // Arrange
        let outbox = OutboxFalso::with(&[envelope("product_created")]).sin_liderar();
        let publicador = PublicadorFalso::aceptando(usize::MAX);
        let relay = Relay::new(outbox.clone(), publicador.clone());

        // Act
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Ok(Progreso::Idle));
        assert!(publicador.publicados().is_empty());
        assert_eq!(outbox.sin_publicar().len(), 1);
    }

    #[tokio::test]
    async fn should_ask_for_more_at_once_when_the_batch_was_full() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = OutboxFalso::with(&events);
        let relay =
            Relay::new(outbox.clone(), PublicadorFalso::aceptando(usize::MAX)).con_tamano_lote(2);

        // Act
        let first = relay.relay_una_vez().await;
        let second = relay.relay_una_vez().await;

        // Assert
        assert_eq!((first, second), (Ok(Progreso::More), Ok(Progreso::Idle)));
        assert!(outbox.sin_publicar().is_empty());
    }

    #[tokio::test]
    async fn should_fail_the_pass_when_the_outbox_cannot_be_read() {
        // Arrange
        let outbox = OutboxFalso::with(&[envelope("product_created")]).unreadable();
        let publicador = PublicadorFalso::aceptando(usize::MAX);
        let relay = Relay::new(outbox, publicador.clone());

        // Act
        let result = relay.relay_una_vez().await;

        // Assert
        assert_eq!(result, Err(ErrorTransporte::NoDisponible));
        assert!(publicador.publicados().is_empty());
    }

    #[test]
    fn should_read_at_least_one_row_when_the_batch_size_is_zero() {
        let relay =
            Relay::new(OutboxFalso::with(&[]), PublicadorFalso::aceptando(0)).con_tamano_lote(0);
        assert_eq!(relay.tamano_lote, 1);
    }
}
