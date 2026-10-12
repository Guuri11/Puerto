//! A consumer's input on the Kafka transport.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use puerto_core::events::REASON_PAYLOAD_INVALID;
use puerto_core::events::delivery::Delivery;
use puerto_core::events::transport::{
    Claimed, DeadLetterRecord, DeadLetterSink, EventSubscription, TransportError,
};
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::error::KafkaError;
use rdkafka::message::{Headers, Message, OwnedMessage};
use rdkafka::{Offset, TopicPartitionList};
use tokio::time::Instant;
use uuid::Uuid;

use crate::KafkaConfig;
use crate::wire;

/// How long one batch waits for a record before it answers that nothing is due.
const RECEIVE_WAIT: Duration = Duration::from_millis(100);
/// Longer than the policy's longest backoff (300 s) plus a handler's run: while a retry waits,
/// nothing is read, and the broker must not take that for a dead consumer.
const MAX_POLL_INTERVAL_MS: &str = "600000";

/// Where a record sits: what is committed once its outcome is settled.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Position {
    topic: String,
    partition: i32,
    offset: i64,
}

impl Position {
    fn of(message: &OwnedMessage) -> Self {
        Self {
            topic: message.topic().to_string(),
            partition: message.partition(),
            offset: message.offset(),
        }
    }
}

/// A delivery that asked to be retried: handed out again once `due`.
#[derive(Debug)]
struct Retry {
    claimed: Claimed,
    position: Position,
    due: Instant,
}

/// What the subscription keeps between batches — the claim handed out and the retry waiting.
/// Pure, so its rules are tested without a broker.
#[derive(Debug, Default)]
struct Ledger {
    in_flight: Option<(Uuid, Position)>,
    retry: Option<Retry>,
}

impl Ledger {
    /// `None` when no retry is waiting: read the next record. Otherwise what to hand out now —
    /// the retry once it is due, nothing before (the records behind it wait).
    fn retry_batch(&mut self, now: Instant) -> Option<Vec<Claimed>> {
        let retry = self.retry.take_if(|retry| retry.due <= now);
        match retry {
            Some(retry) => {
                self.in_flight = Some((retry.claimed.envelope.id, retry.position));
                Some(vec![retry.claimed])
            }
            None if self.retry.is_some() => Some(vec![]),
            None => None,
        }
    }

    fn hand_out(&mut self, claimed: &Claimed, position: Position) {
        self.in_flight = Some((claimed.envelope.id, position));
    }

    /// The position to commit, or `None` for a retry, which keeps the record (and every one
    /// behind it in the partition) for later. A claim this subscription did not hand out is an
    /// error.
    fn settle(
        &mut self,
        claimed: &Claimed,
        delivery: &Delivery,
        now: Instant,
    ) -> Result<Option<Position>, TransportError> {
        let position = match self.in_flight.take() {
            Some((id, position)) if id == claimed.envelope.id => position,
            other => {
                self.in_flight = other;
                return Err(TransportError::InvalidData);
            }
        };
        Ok(match delivery {
            Delivery::Retry { delay } => {
                self.retry = Some(Retry {
                    claimed: Claimed::new(claimed.envelope.clone(), claimed.attempt + 1),
                    position,
                    due: now + *delay,
                });
                None
            }
            Delivery::Ack | Delivery::DeadLetter { .. } => Some(position),
        })
    }
}

/// A record that is not an envelope, whose dead letter is still to be written.
struct Poisoned {
    record: DeadLetterRecord,
    position: Position,
}

/// One `[[consumer]]` on Kafka: a consumer group (`<prefix>.<consumer>`) on the topics of its
/// aggregates, handing out the records of its event types one at a time, in partition order.
///
/// - The offset is committed after the outcome (`Ack`, or `DeadLetter` once recorded): a worker
///   that stops before that sees the record again — at-least-once.
/// - `Retry` keeps the record and hands it out again after the policy's backoff, as the next
///   attempt; nothing is read meanwhile. A restart starts its attempts over.
/// - A record of another event type is skipped (committed); one that is not an envelope is
///   dead-lettered as `payload_invalid` and skipped.
pub struct KafkaSubscription {
    name: String,
    consumer: Arc<StreamConsumer>,
    event_types: Vec<String>,
    dead_letters: Arc<dyn DeadLetterSink>,
    ledger: Ledger,
    poisoned: Option<Poisoned>,
}

impl KafkaSubscription {
    /// Joins the consumer's group and subscribes to the topics of `aggregate_types` (create them
    /// first: [`KafkaConfig::ensure_topics`]). A new group starts from the earliest record.
    pub fn new(
        config: &KafkaConfig,
        consumer: &str,
        aggregate_types: &[&str],
        event_types: &[&str],
        dead_letters: Arc<dyn DeadLetterSink>,
    ) -> Result<Self, KafkaError> {
        let client: StreamConsumer = config
            .client()
            .set("group.id", config.group(consumer))
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("max.poll.interval.ms", MAX_POLL_INTERVAL_MS)
            .create()?;
        let topics: Vec<String> = aggregate_types.iter().map(|a| config.topic(a)).collect();
        let topics: Vec<&str> = topics.iter().map(String::as_str).collect();
        client.subscribe(&topics)?;
        Ok(Self {
            name: consumer.to_string(),
            consumer: Arc::new(client),
            event_types: event_types.iter().map(|t| t.to_string()).collect(),
            dead_letters,
            ledger: Ledger::default(),
            poisoned: None,
        })
    }

    /// The next record, or `None` when none arrives within [`RECEIVE_WAIT`].
    async fn receive(&self) -> Result<Option<OwnedMessage>, TransportError> {
        match tokio::time::timeout(RECEIVE_WAIT, self.consumer.recv()).await {
            Err(_) => Ok(None),
            Ok(Ok(message)) => Ok(Some(message.detach())),
            Ok(Err(err)) => {
                tracing::error!(consumer = self.name, error = %err, "kafka: receive failed");
                Err(TransportError::Unavailable)
            }
        }
    }

    /// Commits past `position`. librdkafka's commit blocks, so it runs off the async threads.
    async fn commit(&self, position: Position) -> Result<(), TransportError> {
        let mut offsets = TopicPartitionList::new();
        let next = Offset::Offset(position.offset + 1);
        let added = offsets.add_partition_offset(&position.topic, position.partition, next);
        let consumer = Arc::clone(&self.consumer);
        let committed = match added {
            Ok(()) => {
                tokio::task::spawn_blocking(move || consumer.commit(&offsets, CommitMode::Sync))
                    .await
                    .map_err(|err| err.to_string())
                    .and_then(|result| result.map_err(|err| err.to_string()))
            }
            Err(err) => Err(err.to_string()),
        };
        committed.map_err(|err| {
            tracing::error!(
                consumer = self.name,
                topic = position.topic,
                error = err,
                "kafka: commit failed"
            );
            TransportError::Unavailable
        })
    }

    /// Records the dead letter of a record that is not an envelope, then skips it. When the sink
    /// fails it is kept, and the next batch tries again before reading on.
    async fn retire_unreadable(&mut self, poisoned: Poisoned) -> Result<(), TransportError> {
        if let Err(err) = self.dead_letters.record(&poisoned.record).await {
            tracing::error!(consumer = self.name, error = %err, "kafka: dead letter not recorded, keeping the record");
            self.poisoned = Some(poisoned);
            return Err(err);
        }
        self.commit(poisoned.position).await
    }
}

#[async_trait]
impl EventSubscription for KafkaSubscription {
    fn consumer(&self) -> &str {
        &self.name
    }

    /// One record per batch: a `Retry` must hold back the records behind it.
    fn max_batch(&self) -> u32 {
        1
    }

    async fn next_batch(&mut self, max: u32) -> Result<Vec<Claimed>, TransportError> {
        if max == 0 {
            return Ok(vec![]);
        }
        if let Some(poisoned) = self.poisoned.take() {
            self.retire_unreadable(poisoned).await?;
        }
        if let Some(batch) = self.ledger.retry_batch(Instant::now()) {
            return Ok(batch);
        }
        while let Some(message) = self.receive().await? {
            let position = Position::of(&message);
            let headers: wire::RawHeaders = message
                .headers()
                .map(|headers| {
                    headers
                        .iter()
                        .map(|h| (h.key.to_string(), h.value.map(<[u8]>::to_vec)))
                        .collect()
                })
                .unwrap_or_default();
            match wire::envelope_from(&headers, message.payload()) {
                Ok(envelope) if self.event_types.contains(&envelope.event_type) => {
                    let claimed = Claimed::new(envelope, 1);
                    self.ledger.hand_out(&claimed, position);
                    return Ok(vec![claimed]);
                }
                Ok(_) => self.commit(position).await?,
                Err(detail) => {
                    tracing::warn!(
                        consumer = self.name,
                        topic = position.topic,
                        offset = position.offset,
                        detail,
                        "kafka: record is not an event"
                    );
                    let raw = String::from_utf8_lossy(message.payload().unwrap_or_default());
                    let record = DeadLetterRecord::unreadable(
                        self.name.as_str(),
                        &raw,
                        1,
                        REASON_PAYLOAD_INVALID,
                        detail,
                    );
                    self.retire_unreadable(Poisoned { record, position })
                        .await?;
                }
            }
        }
        Ok(vec![])
    }

    async fn settle(
        &mut self,
        claimed: &Claimed,
        delivery: &Delivery,
    ) -> Result<(), TransportError> {
        match self.ledger.settle(claimed, delivery, Instant::now()) {
            Ok(Some(position)) => self.commit(position).await,
            Ok(None) => Ok(()),
            Err(err) => {
                tracing::error!(consumer = self.name, event_id = %claimed.envelope.id, "kafka: settled a claim that was not handed out");
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puerto_core::events::{DomainEvent, EventEnvelope};
    use serde_json::{Value, json};

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
            json!({ "id": self.0 })
        }
    }

    fn claimed() -> Claimed {
        Claimed::new(EventEnvelope::new(&Created(Uuid::now_v7())), 1)
    }

    fn position(offset: i64) -> Position {
        Position {
            topic: "shop.product".into(),
            partition: 0,
            offset,
        }
    }

    fn handed_out(claimed: &Claimed) -> Ledger {
        let mut ledger = Ledger::default();
        ledger.hand_out(claimed, position(7));
        ledger
    }

    #[test]
    fn should_commit_the_record_when_it_is_acked_or_dead_lettered() {
        let now = Instant::now();
        for delivery in [
            Delivery::Ack,
            Delivery::DeadLetter {
                reason: REASON_PAYLOAD_INVALID,
                detail: "bad".into(),
            },
        ] {
            let claimed = claimed();
            let mut ledger = handed_out(&claimed);
            assert_eq!(
                ledger.settle(&claimed, &delivery, now),
                Ok(Some(position(7)))
            );
            assert_eq!(ledger.retry_batch(now), None);
        }
    }

    #[test]
    fn should_hold_a_retry_until_its_delay_has_passed_then_hand_it_out_as_the_next_attempt() {
        // Arrange
        let now = Instant::now();
        let claimed = claimed();
        let mut ledger = handed_out(&claimed);
        let retry = Delivery::Retry {
            delay: Duration::from_secs(5),
        };

        // Act
        let settled = ledger.settle(&claimed, &retry, now);
        let before = ledger.retry_batch(now + Duration::from_millis(4999));
        let at = ledger.retry_batch(now + Duration::from_secs(5));

        // Assert
        assert_eq!(settled, Ok(None));
        assert_eq!(before, Some(vec![]));
        assert_eq!(at, Some(vec![Claimed::new(claimed.envelope.clone(), 2)]));
        assert_eq!(ledger.retry_batch(now + Duration::from_secs(6)), None);
    }

    #[test]
    fn should_commit_the_retried_record_once_its_next_attempt_is_acked() {
        let now = Instant::now();
        let claimed = claimed();
        let mut ledger = handed_out(&claimed);
        let retry = Delivery::Retry {
            delay: Duration::ZERO,
        };
        let _ = ledger.settle(&claimed, &retry, now);
        let again = ledger.retry_batch(now).unwrap().remove(0);

        assert_eq!(
            ledger.settle(&again, &Delivery::Ack, now),
            Ok(Some(position(7)))
        );
    }

    #[test]
    fn should_refuse_to_settle_a_claim_it_did_not_hand_out() {
        let now = Instant::now();
        let claimed = claimed();
        let mut ledger = handed_out(&claimed);

        assert_eq!(
            ledger.settle(&self::claimed(), &Delivery::Ack, now),
            Err(TransportError::InvalidData)
        );
        assert_eq!(
            ledger.settle(&claimed, &Delivery::Ack, now),
            Ok(Some(position(7)))
        );
        assert_eq!(
            ledger.settle(&claimed, &Delivery::Ack, now),
            Err(TransportError::InvalidData)
        );
    }
}
