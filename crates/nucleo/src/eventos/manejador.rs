use async_trait::async_trait;

use super::SobreEvento;

/// The payload is not the shape the handler reads — retrying cannot fix it.
pub const MOTIVO_PAYLOAD_INVALIDO: &str = "payload_invalido";
/// The handler kept answering `Reintentar` until the delivery cap.
pub const MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS: &str = "entregas_maximas_superadas";
/// The handler judged the failure permanent.
pub const MOTIVO_ERROR_PERMANENTE: &str = "error_permanente";

/// A consumer: what the worker hands every event it subscribed to. Delivery is at-least-once, so
/// a handler must be idempotent — the same `event.id` can arrive twice (ADR-0018 § 6).
#[async_trait]
pub trait ManejadorEvento: Send + Sync {
    async fn manejar(&self, event: &SobreEvento) -> ResultadoManejo;
}

/// What a handler answers. The worker turns it into a [`super::entrega::Entrega`] with
/// [`super::entrega::decidir`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoManejo {
    /// Done: never deliver it again.
    Ack,
    /// Failed for now: deliver it again later, until the cap.
    Reintentar,
    /// Never deliver it again, and keep a dead letter saying why.
    Descartar {
        /// One of the `REASON_*` codes.
        motivo: &'static str,
        detalle: String,
    },
}

impl ResultadoManejo {
    pub fn descartar(motivo: &'static str, detalle: impl Into<String>) -> Self {
        Self::Descartar {
            motivo,
            detalle: detalle.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_reason_codes_stable_when_stored_in_dead_letters() {
        assert_eq!(MOTIVO_PAYLOAD_INVALIDO, "payload_invalido");
        assert_eq!(
            MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS,
            "entregas_maximas_superadas"
        );
        assert_eq!(MOTIVO_ERROR_PERMANENTE, "error_permanente");
    }

    #[test]
    fn should_carry_reason_and_detail_when_dead_lettered() {
        assert_eq!(
            ResultadoManejo::descartar(MOTIVO_ERROR_PERMANENTE, "card declined"),
            ResultadoManejo::Descartar {
                motivo: "error_permanente",
                detalle: "card declined".into()
            }
        );
    }
}
