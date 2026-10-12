//! A consume loop: hands each delivery of one subscription to its handler and settles it with the
//! delivery policy (ADR-0018 § 4, § 6).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use puerto_core::events::delivery::{DEFAULT_MAX_DELIVERIES, Delivery, backoff_for, decide};
use puerto_core::events::transport::{
    Claimed, DeadLetterRecord, DeadLetterSink, EventSubscription, TransportError,
};
use puerto_core::events::{EventHandler, HandlerOutcome};

use crate::driver::{Pass, Progress};

/// How many deliveries one pass claims when nothing else is configured.
pub const DEFAULT_CONSUMER_BATCH_SIZE: u32 = 10;

/// One `[[consumer]]`: its subscription, its handler and where its dead letters go.
pub struct Consumer<S> {
    subscription: S,
    batch_size: u32,
    policy: Policy,
}

/// Everything but the subscription, which is only `Send`: what a delivery needs while it waits
/// on the handler and the sink.
struct Policy {
    handler: Arc<dyn EventHandler>,
    dead_letters: Arc<dyn DeadLetterSink>,
    max_deliveries: u32,
    max_retry_delay: Option<Duration>,
}

impl<S: EventSubscription> Consumer<S> {
    pub fn new(
        subscription: S,
        handler: impl EventHandler + 'static,
        dead_letters: Arc<dyn DeadLetterSink>,
    ) -> Self {
        Self {
            subscription,
            batch_size: DEFAULT_CONSUMER_BATCH_SIZE,
            policy: Policy {
                handler: Arc::new(handler),
                dead_letters,
                max_deliveries: DEFAULT_MAX_DELIVERIES,
                max_retry_delay: None,
            },
        }
    }

    pub fn with_batch_size(mut self, batch_size: u32) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// How many deliveries an event gets before it is dead-lettered.
    pub fn with_max_deliveries(mut self, max_deliveries: u32) -> Self {
        self.policy.max_deliveries = max_deliveries.max(1);
        self
    }

    /// Caps the policy's backoff between redeliveries — `Some(Duration::ZERO)` retries at the
    /// next pass. For tests and development; production keeps the policy's.
    pub fn with_max_retry_delay(mut self, max_retry_delay: Option<Duration>) -> Self {
        self.policy.max_retry_delay = max_retry_delay;
        self
    }

    /// One batch. A claim left unsettled (the pass failed before it) is delivered again once its
    /// lease runs out: at-least-once, never lost.
    pub async fn consume_once(&mut self) -> Result<Progress, TransportError> {
        let batch = self.subscription.next_batch(self.batch_size).await?;
        let full = batch.len() as u64 >= u64::from(self.batch_size);
        let consumer = self.subscription.consumer().to_string();
        for claimed in &batch {
            let delivery = self.policy.deliver(&consumer, claimed).await;
            self.subscription.settle(claimed, &delivery).await?;
        }
        Ok(if full { Progress::More } else { Progress::Idle })
    }
}

impl Policy {
    /// What to settle `claimed` with. A dead letter is recorded before it is answered; when it
    /// cannot be, the delivery is retried instead, so nothing is retired without one.
    async fn deliver(&self, consumer: &str, claimed: &Claimed) -> Delivery {
        let event = &claimed.envelope;
        let outcome = self.handle(consumer, claimed).await;
        match decide(outcome, claimed.attempt, self.max_deliveries) {
            Delivery::Ack => Delivery::Ack,
            Delivery::Retry { delay } => {
                tracing::warn!(consumer, event_id = %event.id, attempt = claimed.attempt, "consumer: retry");
                Delivery::Retry {
                    delay: self.cap(delay),
                }
            }
            Delivery::DeadLetter { reason, detail } => {
                let record = DeadLetterRecord::for_event(
                    consumer,
                    event,
                    claimed.attempt,
                    reason,
                    detail.clone(),
                );
                match self.dead_letters.record(&record).await {
                    Ok(()) => Delivery::DeadLetter { reason, detail },
                    Err(err) => {
                        tracing::error!(consumer, event_id = %event.id, error = %err, "consumer: dead letter not recorded, retrying");
                        Delivery::Retry {
                            delay: self.cap(backoff_for(claimed.attempt)),
                        }
                    }
                }
            }
        }
    }

    /// The handler's answer. It runs on its own task, so a panic is caught: it is a failure that
    /// may pass (`Retry`), and the loop goes on.
    async fn handle(&self, consumer: &str, claimed: &Claimed) -> HandlerOutcome {
        let handler = Arc::clone(&self.handler);
        let event = claimed.envelope.clone();
        match tokio::spawn(async move { handler.handle(&event).await }).await {
            Ok(outcome) => outcome,
            Err(err) => {
                tracing::error!(
                    consumer,
                    event_id = %claimed.envelope.id,
                    error = %err,
                    "consumer: handler panicked"
                );
                HandlerOutcome::Retry
            }
        }
    }

    fn cap(&self, delay: Duration) -> Duration {
        self.max_retry_delay.map_or(delay, |max| delay.min(max))
    }
}

#[async_trait]
impl<S: EventSubscription + 'static> Pass for Consumer<S> {
    fn name(&self) -> String {
        format!("consumer {}", self.subscription.consumer())
    }

    async fn pass(&mut self) -> Result<Progress, TransportError> {
        self.consume_once().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeSink, FakeSubscription, Scripted, envelope};
    use puerto_core::events::{REASON_MAX_DELIVERIES_EXCEEDED, REASON_PAYLOAD_INVALID};

    fn consumer(
        due: Vec<Claimed>,
        handler: Scripted,
        sink: &FakeSink,
    ) -> (Consumer<FakeSubscription>, FakeSubscription) {
        let subscription = FakeSubscription::with(due);
        let consumer = Consumer::new(subscription.clone(), handler, Arc::new(sink.clone()));
        (consumer, subscription)
    }

    #[tokio::test]
    async fn should_settle_ack_when_the_handler_acks() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 1);
        let sink = FakeSink::default();
        let (mut consumer, subscription) = consumer(vec![claimed.clone()], Scripted::Ack, &sink);

        // Act
        let result = consumer.consume_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        assert_eq!(
            subscription.settled(),
            vec![(claimed.envelope.id, Delivery::Ack)]
        );
        assert!(sink.records().is_empty());
    }

    #[tokio::test]
    async fn should_settle_a_retry_with_the_backoff_when_below_the_cap() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 2);
        let sink = FakeSink::default();
        let (mut consumer, subscription) = consumer(vec![claimed.clone()], Scripted::Retry, &sink);

        // Act
        let _ = consumer.consume_once().await;

        // Assert
        assert_eq!(
            subscription.settled(),
            vec![(
                claimed.envelope.id,
                Delivery::Retry {
                    delay: Duration::from_secs(5)
                }
            )]
        );
        assert!(sink.records().is_empty());
    }

    #[tokio::test]
    async fn should_dead_letter_when_the_retry_reaches_the_cap() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 5);
        let sink = FakeSink::default();
        let (mut consumer, subscription) = consumer(vec![claimed.clone()], Scripted::Retry, &sink);

        // Act
        let _ = consumer.consume_once().await;

        // Assert
        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event_id, Some(claimed.envelope.id));
        assert_eq!(records[0].consumer, "notify_new_product");
        assert_eq!(records[0].attempts, 5);
        assert_eq!(records[0].reason, REASON_MAX_DELIVERIES_EXCEEDED);
        assert!(matches!(
            subscription.settled()[0].1,
            Delivery::DeadLetter {
                reason: REASON_MAX_DELIVERIES_EXCEEDED,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn should_record_the_dead_letter_before_settling_when_the_handler_gives_up() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 1);
        let sink = FakeSink::default();
        let subscription = FakeSubscription::with(vec![claimed.clone()]).watching(&sink);
        let mut consumer = Consumer::new(
            subscription.clone(),
            Scripted::DecodeAs,
            Arc::new(sink.clone()),
        );

        // Act
        let _ = consumer.consume_once().await;

        // Assert — the payload is not a number: the handler dead-letters it as invalid
        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].reason, REASON_PAYLOAD_INVALID);
        assert_eq!(
            subscription.dead_letters_seen_at_settle(),
            vec![1],
            "the dead letter was not recorded before the claim was settled"
        );
        assert!(matches!(
            subscription.settled()[0].1,
            Delivery::DeadLetter {
                reason: REASON_PAYLOAD_INVALID,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn should_retry_instead_of_retiring_when_the_dead_letter_cannot_be_recorded() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 5);
        let sink = FakeSink::failing();
        let (mut consumer, subscription) = consumer(vec![claimed.clone()], Scripted::Retry, &sink);

        // Act
        let _ = consumer.consume_once().await;

        // Assert
        assert_eq!(
            subscription.settled(),
            vec![(
                claimed.envelope.id,
                Delivery::Retry {
                    delay: Duration::from_secs(300)
                }
            )]
        );
    }

    #[tokio::test]
    async fn should_treat_a_panic_as_a_retry_and_go_on_when_the_handler_panics() {
        // Arrange
        let first = Claimed::new(envelope("product_created"), 1);
        let second = Claimed::new(envelope("product_created"), 1);
        let sink = FakeSink::default();
        let (mut consumer, subscription) = consumer(
            vec![first.clone(), second.clone()],
            Scripted::PanicOnce(Default::default()),
            &sink,
        );

        // Act
        let result = consumer.consume_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        assert_eq!(
            subscription.settled(),
            vec![
                (
                    first.envelope.id,
                    Delivery::Retry {
                        delay: Duration::from_secs(1)
                    }
                ),
                (second.envelope.id, Delivery::Ack),
            ]
        );
    }

    #[tokio::test]
    async fn should_cap_the_retry_delay_when_a_maximum_is_configured() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 4);
        let sink = FakeSink::default();
        let (consumer, subscription) = consumer(vec![claimed.clone()], Scripted::Retry, &sink);
        let mut consumer = consumer.with_max_retry_delay(Some(Duration::from_millis(10)));

        // Act
        let _ = consumer.consume_once().await;

        // Assert
        assert_eq!(
            subscription.settled()[0].1,
            Delivery::Retry {
                delay: Duration::from_millis(10)
            }
        );
    }

    #[tokio::test]
    async fn should_dead_letter_earlier_when_fewer_deliveries_are_configured() {
        // Arrange
        let claimed = Claimed::new(envelope("product_created"), 2);
        let sink = FakeSink::default();
        let (consumer, _) = consumer(vec![claimed], Scripted::Retry, &sink);
        let mut consumer = consumer.with_max_deliveries(2);

        // Act
        let _ = consumer.consume_once().await;

        // Assert
        assert_eq!(sink.records().len(), 1);
    }

    #[tokio::test]
    async fn should_ask_for_more_at_once_when_the_batch_was_full() {
        // Arrange
        let due = (0..3)
            .map(|_| Claimed::new(envelope("product_created"), 1))
            .collect();
        let sink = FakeSink::default();
        let (consumer, subscription) = consumer(due, Scripted::Ack, &sink);
        let mut consumer = consumer.with_batch_size(2);

        // Act
        let first = consumer.consume_once().await;
        let second = consumer.consume_once().await;

        // Assert
        assert_eq!((first, second), (Ok(Progress::More), Ok(Progress::Idle)));
        assert_eq!(subscription.settled().len(), 3);
    }

    #[tokio::test]
    async fn should_stop_the_pass_when_a_claim_cannot_be_settled() {
        // Arrange
        let due = (0..2)
            .map(|_| Claimed::new(envelope("product_created"), 1))
            .collect();
        let sink = FakeSink::default();
        let subscription = FakeSubscription::with(due).failing_settle();
        let mut consumer =
            Consumer::new(subscription.clone(), Scripted::Ack, Arc::new(sink.clone()));

        // Act
        let result = consumer.consume_once().await;

        // Assert
        assert_eq!(result, Err(TransportError::Unavailable));
        assert_eq!(subscription.settle_calls(), 1);
    }

    #[test]
    fn should_deliver_at_least_once_when_the_cap_is_zero() {
        let consumer = Consumer::new(
            FakeSubscription::with(vec![]),
            Scripted::Ack,
            Arc::new(FakeSink::default()),
        )
        .with_max_deliveries(0)
        .with_batch_size(0);
        assert_eq!(
            (consumer.policy.max_deliveries, consumer.batch_size),
            (1, 1)
        );
    }
}
