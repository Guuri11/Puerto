//! The ports the worker drives (ADR-0018 § 4). A transport (`db`, `kafka`) is an adapter crate
//! implementing them; the relay and the consume loop in `puerto-events` only see these traits.
//!
//! ```text
//! Outbox ──fetch──▶ relay ──publish──▶ EventPublisher      (relay: one process holds the lead)
//! EventSubscription ──next_batch──▶ consumer ──handle──▶ EventHandler
//!                   ◀──settle(decide(..))──  └─dead letter─▶ DeadLetterSink (before settling)
//! ```

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use super::EventEnvelope;
use super::delivery::Delivery;

/// How much of a payload a dead letter keeps, in characters.
pub const DEAD_LETTER_PAYLOAD_LIMIT: usize = 4000;

/// What a transport port fails with. The adapter logs the driver error with its context; the
/// loop that called it only needs to know to try again later.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TransportError {
    /// The store or the broker failed or could not be reached.
    #[error("transport.unavailable")]
    Unavailable,
    /// What was stored cannot be read back as an envelope.
    #[error("transport.invalid_data")]
    InvalidData,
}

/// The relay's input: the events written by the API's units of work, not yet published.
#[async_trait]
pub trait Outbox: Send + Sync {
    /// Whether this process may relay now. Only one relay runs at a time, so the outbox order
    /// is kept with several worker replicas (a Postgres advisory lock). It is asked before
    /// every batch; once held, it keeps answering `true`.
    async fn try_lead(&self) -> Result<bool, TransportError>;
    /// Up to `limit` unpublished events, oldest first (the order they were written).
    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<EventEnvelope>, TransportError>;
    /// Marks these events published. Idempotent: an id already marked is left as it is.
    async fn mark_published(&self, ids: &[Uuid]) -> Result<(), TransportError>;
}

/// The relay's output: hands one event to the transport. `Ok` means the transport has it
/// durably — the relay marks it published only then, and stops at the first `Err` so the next
/// batch starts from that event again (order kept).
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: &EventEnvelope) -> Result<(), TransportError>;
}

/// One event handed to one consumer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Claimed {
    pub envelope: EventEnvelope,
    /// The 1-based number of this delivery to this consumer — what `decide` reads.
    pub attempt: u32,
}

impl Claimed {
    pub fn new(envelope: EventEnvelope, attempt: u32) -> Self {
        Self { envelope, attempt }
    }
}

/// A consumer's input. The consume loop asks for a batch, hands each event to the handler and
/// settles it with what `decide` answered; an unsettled claim is delivered again later.
#[async_trait]
pub trait EventSubscription: Send {
    /// The consumer it delivers to — the `[[consumer]] name`.
    fn consumer(&self) -> &str;
    /// The most claims one batch hands out, whatever `next_batch` is asked for. A transport that
    /// keeps an order (Kafka: a partition) answers 1, so a `Retry` holds back what comes after
    /// it instead of letting the handler see it first.
    fn max_batch(&self) -> u32 {
        u32::MAX
    }
    /// Up to `max` events due for this consumer. Empty when there is nothing to do.
    async fn next_batch(&mut self, max: u32) -> Result<Vec<Claimed>, TransportError>;
    /// Ack → never again; `Retry { delay }` → again after `delay`; `DeadLetter` → never again
    /// (the dead letter is recorded before this is called).
    async fn settle(
        &mut self,
        claimed: &Claimed,
        delivery: &Delivery,
    ) -> Result<(), TransportError>;
}

/// What is kept of an event no consumer will see again: nothing is retired without one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetterRecord {
    /// `None` when what arrived was not an envelope at all.
    pub event_id: Option<Uuid>,
    pub consumer: String,
    pub event_type: Option<String>,
    pub attempts: u32,
    pub reason: String,
    pub detail: String,
    /// At most [`DEAD_LETTER_PAYLOAD_LIMIT`] characters.
    pub payload: String,
}

impl DeadLetterRecord {
    pub fn for_event(
        consumer: impl Into<String>,
        event: &EventEnvelope,
        attempts: u32,
        reason: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Some(event.id),
            consumer: consumer.into(),
            event_type: Some(event.event_type.clone()),
            attempts,
            reason: reason.to_string(),
            detail: detail.into(),
            payload: trim_payload(&event.payload.to_string()),
        }
    }

    /// A message that could not be read as an envelope (a poisoned Kafka record).
    pub fn unreadable(
        consumer: impl Into<String>,
        raw: &str,
        attempts: u32,
        reason: &str,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            event_id: None,
            consumer: consumer.into(),
            event_type: None,
            attempts,
            reason: reason.to_string(),
            detail: detail.into(),
            payload: trim_payload(raw),
        }
    }
}

fn trim_payload(payload: &str) -> String {
    payload.chars().take(DEAD_LETTER_PAYLOAD_LIMIT).collect()
}

/// Where dead letters go — Postgres, whatever the transport. Idempotent on
/// `(event_id, consumer)`: recording the same one twice keeps one.
#[async_trait]
pub trait DeadLetterSink: Send + Sync {
    async fn record(&self, record: &DeadLetterRecord) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    //! A fake of each port, driven the way the relay and the consume loop will drive them: the
    //! signatures have to fit those loops, not only compile.
    use super::*;
    use crate::events::delivery::{DEFAULT_MAX_DELIVERIES, decide};
    use crate::events::{DomainEvent, EventHandler, HandlerOutcome, REASON_PAYLOAD_INVALID};
    use serde_json::{Value, json};
    use std::sync::Mutex;
    use std::time::Duration;

    struct Created(Uuid);

    impl DomainEvent for Created {
        fn event_type(&self) -> &'static str {
            "product_created"
        }
        fn aggregate_type(&self) -> &'static str {
            "product"
        }
        fn aggregate_id(&self) -> Uuid {
            self.0
        }
        fn schema_version(&self) -> u16 {
            1
        }
        fn payload(&self) -> Value {
            json!({ "id": self.0, "name": "Desk lamp" })
        }
    }

    fn envelope() -> EventEnvelope {
        EventEnvelope::new(&Created(Uuid::now_v7()))
    }

    #[derive(Default)]
    struct FakeOutbox {
        rows: Mutex<Vec<(EventEnvelope, bool)>>,
    }

    #[async_trait]
    impl Outbox for FakeOutbox {
        async fn try_lead(&self) -> Result<bool, TransportError> {
            Ok(true)
        }
        async fn fetch_unpublished(
            &self,
            limit: u32,
        ) -> Result<Vec<EventEnvelope>, TransportError> {
            let rows = self.rows.lock().map_err(|_| TransportError::Unavailable)?;
            Ok(rows
                .iter()
                .filter(|(_, published)| !published)
                .take(limit as usize)
                .map(|(event, _)| event.clone())
                .collect())
        }
        async fn mark_published(&self, ids: &[Uuid]) -> Result<(), TransportError> {
            let mut rows = self.rows.lock().map_err(|_| TransportError::Unavailable)?;
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
    impl EventPublisher for FakePublisher {
        async fn publish(&self, event: &EventEnvelope) -> Result<(), TransportError> {
            let mut published = self
                .published
                .lock()
                .map_err(|_| TransportError::Unavailable)?;
            if published.len() == self.capacity {
                return Err(TransportError::Unavailable);
            }
            published.push(event.id);
            Ok(())
        }
    }

    struct FakeSubscription {
        due: Vec<Claimed>,
        settled: Vec<(Uuid, Delivery)>,
    }

    #[async_trait]
    impl EventSubscription for FakeSubscription {
        fn consumer(&self) -> &str {
            "notify_new_product"
        }
        async fn next_batch(&mut self, max: u32) -> Result<Vec<Claimed>, TransportError> {
            let take = self.due.len().min(max as usize);
            Ok(self.due.drain(..take).collect())
        }
        async fn settle(
            &mut self,
            claimed: &Claimed,
            delivery: &Delivery,
        ) -> Result<(), TransportError> {
            self.settled.push((claimed.envelope.id, delivery.clone()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeSink {
        records: Mutex<Vec<DeadLetterRecord>>,
    }

    #[async_trait]
    impl DeadLetterSink for FakeSink {
        async fn record(&self, record: &DeadLetterRecord) -> Result<(), TransportError> {
            let mut records = self
                .records
                .lock()
                .map_err(|_| TransportError::Unavailable)?;
            let seen = records
                .iter()
                .any(|r| r.event_id == record.event_id && r.consumer == record.consumer);
            if !seen {
                records.push(record.clone());
            }
            Ok(())
        }
    }

    /// Acks a payload with a `name`, dead-letters the rest, retries nothing.
    struct NeedsName;

    #[async_trait]
    impl EventHandler for NeedsName {
        async fn handle(&self, event: &EventEnvelope) -> HandlerOutcome {
            match event.payload.get("name") {
                Some(_) => HandlerOutcome::Ack,
                None => HandlerOutcome::dead_letter(REASON_PAYLOAD_INVALID, "missing `name`"),
            }
        }
    }

    #[tokio::test]
    async fn should_mark_only_the_confirmed_prefix_when_the_publisher_fails_midway() {
        // Arrange
        let events: Vec<EventEnvelope> = (0..3).map(|_| envelope()).collect();
        let outbox = FakeOutbox {
            rows: Mutex::new(events.iter().cloned().map(|e| (e, false)).collect()),
        };
        let publisher = FakePublisher {
            capacity: 2,
            published: Mutex::new(Vec::new()),
        };

        // Act — one relay pass
        assert_eq!(outbox.try_lead().await, Ok(true));
        let mut confirmed = Vec::new();
        for event in outbox.fetch_unpublished(10).await.unwrap_or_default() {
            if publisher.publish(&event).await.is_err() {
                break;
            }
            confirmed.push(event.id);
        }
        let marked = outbox.mark_published(&confirmed).await;

        // Assert
        assert_eq!(marked, Ok(()));
        let left = outbox.fetch_unpublished(10).await.unwrap_or_default();
        assert_eq!(left, vec![events[2].clone()]);
    }

    #[tokio::test]
    async fn should_leave_marked_events_marked_when_marked_again() {
        let event = envelope();
        let outbox = FakeOutbox {
            rows: Mutex::new(vec![(event.clone(), false)]),
        };

        let first = outbox.mark_published(&[event.id]).await;
        let second = outbox.mark_published(&[event.id]).await;

        assert_eq!((first, second), (Ok(()), Ok(())));
        assert_eq!(outbox.fetch_unpublished(10).await, Ok(vec![]));
    }

    #[tokio::test]
    async fn should_settle_every_claim_and_dead_letter_first_when_driven_by_a_consume_pass() {
        // Arrange
        let good = envelope();
        let mut poisoned = envelope();
        poisoned.payload = json!({ "id": poisoned.aggregate_id });
        let mut subscription = FakeSubscription {
            due: vec![
                Claimed::new(good.clone(), 1),
                Claimed::new(poisoned.clone(), 2),
            ],
            settled: Vec::new(),
        };
        let sink = FakeSink::default();

        // Act — one consume pass
        for claimed in subscription.next_batch(10).await.unwrap_or_default() {
            let outcome = NeedsName.handle(&claimed.envelope).await;
            let delivery = decide(outcome, claimed.attempt, DEFAULT_MAX_DELIVERIES);
            if let Delivery::DeadLetter { reason, detail } = &delivery {
                let record = DeadLetterRecord::for_event(
                    subscription.consumer(),
                    &claimed.envelope,
                    claimed.attempt,
                    reason,
                    detail.clone(),
                );
                assert_eq!(sink.record(&record).await, Ok(()));
            }
            assert_eq!(subscription.settle(&claimed, &delivery).await, Ok(()));
        }

        // Assert
        assert_eq!(
            subscription.settled,
            vec![
                (good.id, Delivery::Ack),
                (
                    poisoned.id,
                    Delivery::DeadLetter {
                        reason: "payload_invalid",
                        detail: "missing `name`".into()
                    }
                ),
            ]
        );
        let records = sink.records.lock().map(|r| r.clone()).unwrap_or_default();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event_id, Some(poisoned.id));
        assert_eq!(records[0].consumer, "notify_new_product");
        assert_eq!(records[0].attempts, 2);
    }

    #[tokio::test]
    async fn should_hand_out_at_most_the_batch_size_when_more_are_due() {
        let mut subscription = FakeSubscription {
            due: (0..3).map(|_| Claimed::new(envelope(), 1)).collect(),
            settled: Vec::new(),
        };

        let first = subscription.next_batch(2).await.unwrap_or_default();
        let second = subscription.next_batch(2).await.unwrap_or_default();

        assert_eq!((first.len(), second.len()), (2, 1));
    }

    #[tokio::test]
    async fn should_keep_one_dead_letter_when_the_same_one_is_recorded_twice() {
        let sink = FakeSink::default();
        let record = DeadLetterRecord::for_event("audit", &envelope(), 5, "permanent_error", "no");

        let _ = sink.record(&record).await;
        let _ = sink.record(&record).await;

        assert_eq!(sink.records.lock().map(|r| r.len()).unwrap_or(0), 1);
    }

    #[test]
    fn should_describe_the_event_when_dead_lettering_an_envelope() {
        let event = envelope();

        let record =
            DeadLetterRecord::for_event("audit", &event, 5, "max_deliveries_exceeded", "timeout");

        assert_eq!(
            record,
            DeadLetterRecord {
                event_id: Some(event.id),
                consumer: "audit".into(),
                event_type: Some("product_created".into()),
                attempts: 5,
                reason: "max_deliveries_exceeded".into(),
                detail: "timeout".into(),
                payload: event.payload.to_string(),
            }
        );
    }

    #[test]
    fn should_keep_the_raw_message_when_dead_lettering_one_that_is_not_an_envelope() {
        let record = DeadLetterRecord::unreadable("audit", "not json", 1, "payload_invalid", "eof");

        assert_eq!(record.event_id, None);
        assert_eq!(record.event_type, None);
        assert_eq!(record.payload, "not json");
        assert_eq!(record.reason, "payload_invalid");
    }

    #[test]
    fn should_trim_the_payload_when_it_is_longer_than_the_limit() {
        let at_limit = "é".repeat(DEAD_LETTER_PAYLOAD_LIMIT);
        let over = format!("{at_limit}tail");

        let kept = DeadLetterRecord::unreadable("audit", &at_limit, 1, "payload_invalid", "");
        let trimmed = DeadLetterRecord::unreadable("audit", &over, 1, "payload_invalid", "");

        assert_eq!(kept.payload, at_limit);
        assert_eq!(trimmed.payload, at_limit);
    }

    #[test]
    fn should_speak_machine_readable_codes_when_a_transport_fails() {
        assert_eq!(
            TransportError::Unavailable.to_string(),
            "transport.unavailable"
        );
        assert_eq!(
            TransportError::InvalidData.to_string(),
            "transport.invalid_data"
        );
    }

    #[test]
    fn should_keep_the_retry_delay_when_a_claim_is_settled_for_later() {
        let delivery = decide(HandlerOutcome::Retry, 2, DEFAULT_MAX_DELIVERIES);
        assert_eq!(
            delivery,
            Delivery::Retry {
                delay: Duration::from_secs(5)
            }
        );
    }
}
