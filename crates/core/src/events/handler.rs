use async_trait::async_trait;

use super::EventEnvelope;

/// The payload is not the shape the handler reads — retrying cannot fix it.
pub const REASON_PAYLOAD_INVALID: &str = "payload_invalid";
/// The handler kept answering `Retry` until the delivery cap.
pub const REASON_MAX_DELIVERIES_EXCEEDED: &str = "max_deliveries_exceeded";
/// The handler judged the failure permanent.
pub const REASON_PERMANENT_ERROR: &str = "permanent_error";

/// A consumer: what the worker hands every event it subscribed to. Delivery is at-least-once, so
/// a handler must be idempotent — the same `event.id` can arrive twice (ADR-0018 § 6).
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &EventEnvelope) -> HandlerOutcome;
}

/// What a handler answers. The worker turns it into a [`super::delivery::Delivery`] with
/// [`super::delivery::decide`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerOutcome {
    /// Done: never deliver it again.
    Ack,
    /// Failed for now: deliver it again later, until the cap.
    Retry,
    /// Never deliver it again, and keep a dead letter saying why.
    DeadLetter {
        /// One of the `REASON_*` codes.
        reason: &'static str,
        detail: String,
    },
}

impl HandlerOutcome {
    pub fn dead_letter(reason: &'static str, detail: impl Into<String>) -> Self {
        Self::DeadLetter {
            reason,
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_keep_reason_codes_stable_when_stored_in_dead_letters() {
        assert_eq!(REASON_PAYLOAD_INVALID, "payload_invalid");
        assert_eq!(REASON_MAX_DELIVERIES_EXCEEDED, "max_deliveries_exceeded");
        assert_eq!(REASON_PERMANENT_ERROR, "permanent_error");
    }

    #[test]
    fn should_carry_reason_and_detail_when_dead_lettered() {
        assert_eq!(
            HandlerOutcome::dead_letter(REASON_PERMANENT_ERROR, "card declined"),
            HandlerOutcome::DeadLetter {
                reason: "permanent_error",
                detail: "card declined".into()
            }
        );
    }
}
