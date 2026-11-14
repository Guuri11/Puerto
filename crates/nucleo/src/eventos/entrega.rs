//! The delivery policy every transport applies (ADR-0018 § 4): a pure function, so its tests and
//! mutants cover them all.

use std::time::Duration;

use super::manejador::{MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS, ResultadoManejo};

/// How many times an event is delivered to one consumer before it is dead-lettered.
pub const ENTREGAS_MAXIMAS_POR_DEFECTO: u32 = 5;

/// The wait before each redelivery, by attempt; the last one repeats.
const BACKOFF_SECONDS: [u64; 5] = [1, 5, 15, 60, 300];

/// What the transport does with a delivery once its handler answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entrega {
    Ack,
    Reintentar {
        espera: Duration,
    },
    Descartar {
        motivo: &'static str,
        detalle: String,
    },
}

/// `intento` is the 1-based number of the delivery that was just handled. A `Reintentar` at or above
/// `entregas_maximas` is dead-lettered: with the default cap, attempt 4 retries and attempt 5 does
/// not.
pub fn decidir(outcome: ResultadoManejo, intento: u32, entregas_maximas: u32) -> Entrega {
    match outcome {
        ResultadoManejo::Ack => Entrega::Ack,
        ResultadoManejo::Descartar { motivo, detalle } => Entrega::Descartar { motivo, detalle },
        ResultadoManejo::Reintentar if intento >= entregas_maximas => Entrega::Descartar {
            motivo: MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS,
            detalle: format!("still failing after {intento} deliveries"),
        },
        ResultadoManejo::Reintentar => Entrega::Reintentar {
            espera: espera_para(intento),
        },
    }
}

/// The wait after delivery number `intento` failed: 1 s, 5 s, 15 s, 60 s, then 300 s.
pub fn espera_para(intento: u32) -> Duration {
    let index = (intento.max(1) - 1) as usize;
    let seconds = BACKOFF_SECONDS[index.min(BACKOFF_SECONDS.len() - 1)];
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eventos::manejador::MOTIVO_PAYLOAD_INVALIDO;

    #[test]
    fn should_ack_when_the_handler_acks() {
        assert_eq!(
            decidir(ResultadoManejo::Ack, 1, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Ack
        );
    }

    #[test]
    fn should_ack_even_on_the_last_attempt_when_the_handler_acks() {
        assert_eq!(
            decidir(ResultadoManejo::Ack, 5, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Ack
        );
    }

    #[test]
    fn should_dead_letter_with_the_handlers_reason_when_the_handler_gives_up() {
        let outcome = ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, "missing field `nombre`");

        assert_eq!(
            decidir(outcome, 1, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Descartar {
                motivo: "payload_invalido",
                detalle: "missing field `nombre`".into()
            }
        );
    }

    #[test]
    fn should_retry_with_backoff_when_below_the_cap() {
        assert_eq!(
            decidir(ResultadoManejo::Reintentar, 1, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Reintentar {
                espera: Duration::from_secs(1)
            }
        );
        assert_eq!(
            decidir(ResultadoManejo::Reintentar, 4, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Reintentar {
                espera: Duration::from_secs(60)
            }
        );
    }

    #[test]
    fn should_dead_letter_when_the_retry_reaches_the_cap() {
        assert_eq!(
            decidir(ResultadoManejo::Reintentar, 5, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Descartar {
                motivo: "entregas_maximas_superadas",
                detalle: "still failing after 5 deliveries".into()
            }
        );
    }

    #[test]
    fn should_dead_letter_when_the_retry_is_past_the_cap() {
        assert!(matches!(
            decidir(ResultadoManejo::Reintentar, 9, ENTREGAS_MAXIMAS_POR_DEFECTO),
            Entrega::Descartar {
                motivo: MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS,
                ..
            }
        ));
    }

    #[test]
    fn should_grow_the_backoff_with_each_attempt() {
        let delays: Vec<u64> = (1..=5).map(|n| espera_para(n).as_secs()).collect();
        assert_eq!(delays, [1, 5, 15, 60, 300]);
    }

    #[test]
    fn should_cap_the_backoff_when_attempts_go_past_the_table() {
        assert_eq!(espera_para(6), Duration::from_secs(300));
        assert_eq!(espera_para(u32::MAX), Duration::from_secs(300));
    }

    #[test]
    fn should_wait_the_shortest_backoff_when_the_attempt_is_zero() {
        assert_eq!(espera_para(0), Duration::from_secs(1));
    }

    #[test]
    fn should_deliver_five_times_by_default() {
        assert_eq!(ENTREGAS_MAXIMAS_POR_DEFECTO, 5);
    }
}
