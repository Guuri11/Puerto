//! The delivery policy every transport applies (ADR-0018 § 4): a pure function, so its tests and
//! mutants cover them all.

use std::time::Duration;

use super::handler::{HandlerOutcome, REASON_MAX_DELIVERIES_EXCEEDED};

/// How many times an event is delivered to one consumer before it is dead-lettered.
pub const DEFAULT_MAX_DELIVERIES: u32 = 5;

/// The wait before each redelivery, by attempt; the last one repeats.
const BACKOFF_SECONDS: [u64; 5] = [1, 5, 15, 60, 300];

/// What the transport does with a delivery once its handler answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    Ack,
    Retry {
        delay: Duration,
    },
    DeadLetter {
        reason: &'static str,
        detail: String,
    },
}

/// `attempt` is the 1-based number of the delivery that was just handled. A `Retry` at or above
/// `max_deliveries` is dead-lettered: with the default cap, attempt 4 retries and attempt 5 does
/// not.
pub fn decide(outcome: HandlerOutcome, attempt: u32, max_deliveries: u32) -> Delivery {
    match outcome {
        HandlerOutcome::Ack => Delivery::Ack,
        HandlerOutcome::DeadLetter { reason, detail } => Delivery::DeadLetter { reason, detail },
        HandlerOutcome::Retry if attempt >= max_deliveries => Delivery::DeadLetter {
            reason: REASON_MAX_DELIVERIES_EXCEEDED,
            detail: format!("still failing after {attempt} deliveries"),
        },
        HandlerOutcome::Retry => Delivery::Retry {
            delay: backoff_for(attempt),
        },
    }
}

/// The wait after delivery number `attempt` failed: 1 s, 5 s, 15 s, 60 s, then 300 s.
pub fn backoff_for(attempt: u32) -> Duration {
    let index = (attempt.max(1) - 1) as usize;
    let seconds = BACKOFF_SECONDS[index.min(BACKOFF_SECONDS.len() - 1)];
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::handler::REASON_PAYLOAD_INVALID;

    #[test]
    fn should_ack_when_the_handler_acks() {
        assert_eq!(
            decide(HandlerOutcome::Ack, 1, DEFAULT_MAX_DELIVERIES),
            Delivery::Ack
        );
    }

    #[test]
    fn should_ack_even_on_the_last_attempt_when_the_handler_acks() {
        assert_eq!(
            decide(HandlerOutcome::Ack, 5, DEFAULT_MAX_DELIVERIES),
            Delivery::Ack
        );
    }

    #[test]
    fn should_dead_letter_with_the_handlers_reason_when_the_handler_gives_up() {
        let outcome = HandlerOutcome::dead_letter(REASON_PAYLOAD_INVALID, "missing field `name`");

        assert_eq!(
            decide(outcome, 1, DEFAULT_MAX_DELIVERIES),
            Delivery::DeadLetter {
                reason: "payload_invalid",
                detail: "missing field `name`".into()
            }
        );
    }

    #[test]
    fn should_retry_with_backoff_when_below_the_cap() {
        assert_eq!(
            decide(HandlerOutcome::Retry, 1, DEFAULT_MAX_DELIVERIES),
            Delivery::Retry {
                delay: Duration::from_secs(1)
            }
        );
        assert_eq!(
            decide(HandlerOutcome::Retry, 4, DEFAULT_MAX_DELIVERIES),
            Delivery::Retry {
                delay: Duration::from_secs(60)
            }
        );
    }

    #[test]
    fn should_dead_letter_when_the_retry_reaches_the_cap() {
        assert_eq!(
            decide(HandlerOutcome::Retry, 5, DEFAULT_MAX_DELIVERIES),
            Delivery::DeadLetter {
                reason: "max_deliveries_exceeded",
                detail: "still failing after 5 deliveries".into()
            }
        );
    }

    #[test]
    fn should_dead_letter_when_the_retry_is_past_the_cap() {
        assert!(matches!(
            decide(HandlerOutcome::Retry, 9, DEFAULT_MAX_DELIVERIES),
            Delivery::DeadLetter {
                reason: REASON_MAX_DELIVERIES_EXCEEDED,
                ..
            }
        ));
    }

    #[test]
    fn should_grow_the_backoff_with_each_attempt() {
        let delays: Vec<u64> = (1..=5).map(|n| backoff_for(n).as_secs()).collect();
        assert_eq!(delays, [1, 5, 15, 60, 300]);
    }

    #[test]
    fn should_cap_the_backoff_when_attempts_go_past_the_table() {
        assert_eq!(backoff_for(6), Duration::from_secs(300));
        assert_eq!(backoff_for(u32::MAX), Duration::from_secs(300));
    }

    #[test]
    fn should_wait_the_shortest_backoff_when_the_attempt_is_zero() {
        assert_eq!(backoff_for(0), Duration::from_secs(1));
    }

    #[test]
    fn should_deliver_five_times_by_default() {
        assert_eq!(DEFAULT_MAX_DELIVERIES, 5);
    }
}
