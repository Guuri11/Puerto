//! The relay: moves what the API's units of work wrote to the outbox onto the transport, in the
//! order it was written (ADR-0018 § 5).

use async_trait::async_trait;
use puerto_core::events::transport::{EventPublisher, Outbox, TransportError};

use crate::driver::{Pass, Progress};

/// How many outbox rows one pass reads when nothing else is configured.
pub const DEFAULT_RELAY_BATCH_SIZE: u32 = 100;

/// Reads unpublished events from `O` and hands them to `P`. Only the replica holding the lead
/// relays (`Outbox::try_lead`); the others keep asking.
pub struct Relay<O, P> {
    outbox: O,
    publisher: P,
    batch_size: u32,
}

impl<O: Outbox, P: EventPublisher> Relay<O, P> {
    pub fn new(outbox: O, publisher: P) -> Self {
        Self {
            outbox,
            publisher,
            batch_size: DEFAULT_RELAY_BATCH_SIZE,
        }
    }

    pub fn with_batch_size(mut self, batch_size: u32) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    /// One batch: publishes in outbox order and stops at the first failure, so the next pass
    /// starts from that event again. Only what the publisher confirmed is marked published.
    pub async fn relay_once(&self) -> Result<Progress, TransportError> {
        if !self.outbox.try_lead().await? {
            return Ok(Progress::Idle);
        }
        let events = self.outbox.fetch_unpublished(self.batch_size).await?;
        let mut confirmed = Vec::with_capacity(events.len());
        let mut failure = None;
        for event in &events {
            match self.publisher.publish(event).await {
                Ok(()) => confirmed.push(event.id),
                Err(err) => {
                    tracing::warn!(error = %err, event_id = %event.id, "relay: publish failed");
                    failure = Some(err);
                    break;
                }
            }
        }
        if !confirmed.is_empty() {
            self.outbox.mark_published(&confirmed).await?;
            tracing::debug!(count = confirmed.len(), "relay: published");
        }
        match failure {
            Some(err) => Err(err),
            // A full batch: more may be waiting, so the next pass runs at once.
            None if events.len() as u64 >= u64::from(self.batch_size) => Ok(Progress::More),
            None => Ok(Progress::Idle),
        }
    }
}

#[async_trait]
impl<O: Outbox + 'static, P: EventPublisher + 'static> Pass for Relay<O, P> {
    fn name(&self) -> String {
        "relay".to_string()
    }

    async fn pass(&mut self) -> Result<Progress, TransportError> {
        self.relay_once().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeOutbox, FakePublisher, envelope};

    #[tokio::test]
    async fn should_publish_in_order_and_mark_everything_when_the_publisher_accepts_all() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = FakeOutbox::with(&events);
        let publisher = FakePublisher::accepting(usize::MAX);
        let relay = Relay::new(outbox.clone(), publisher.clone());

        // Act
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        let ids: Vec<_> = events.iter().map(|e| e.id).collect();
        assert_eq!(publisher.published(), ids);
        assert_eq!(outbox.marked(), ids);
        assert!(outbox.unpublished().is_empty());
    }

    #[tokio::test]
    async fn should_mark_only_the_confirmed_prefix_when_the_publisher_fails_midway() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = FakeOutbox::with(&events);
        let publisher = FakePublisher::accepting(1);
        let relay = Relay::new(outbox.clone(), publisher.clone());

        // Act
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Err(TransportError::Unavailable));
        assert_eq!(publisher.published(), vec![events[0].id]);
        assert_eq!(outbox.marked(), vec![events[0].id]);
        assert_eq!(outbox.unpublished(), vec![events[1].id, events[2].id]);
    }

    #[tokio::test]
    async fn should_resume_from_the_failed_event_when_the_next_pass_runs() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = FakeOutbox::with(&events);
        let publisher = FakePublisher::accepting(1);
        let relay = Relay::new(outbox.clone(), publisher.clone());
        let _ = relay.relay_once().await;

        // Act
        publisher.accept(usize::MAX);
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        let ids: Vec<_> = events.iter().map(|e| e.id).collect();
        assert_eq!(publisher.published(), ids);
    }

    #[tokio::test]
    async fn should_do_nothing_when_the_outbox_is_empty() {
        // Arrange
        let outbox = FakeOutbox::with(&[]);
        let publisher = FakePublisher::accepting(usize::MAX);
        let relay = Relay::new(outbox.clone(), publisher.clone());

        // Act
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        assert!(publisher.published().is_empty());
        assert_eq!(outbox.mark_calls(), 0);
    }

    #[tokio::test]
    async fn should_publish_nothing_when_another_replica_leads() {
        // Arrange
        let outbox = FakeOutbox::with(&[envelope("product_created")]).not_leading();
        let publisher = FakePublisher::accepting(usize::MAX);
        let relay = Relay::new(outbox.clone(), publisher.clone());

        // Act
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Ok(Progress::Idle));
        assert!(publisher.published().is_empty());
        assert_eq!(outbox.unpublished().len(), 1);
    }

    #[tokio::test]
    async fn should_ask_for_more_at_once_when_the_batch_was_full() {
        // Arrange
        let events: Vec<_> = (0..3).map(|_| envelope("product_created")).collect();
        let outbox = FakeOutbox::with(&events);
        let relay =
            Relay::new(outbox.clone(), FakePublisher::accepting(usize::MAX)).with_batch_size(2);

        // Act
        let first = relay.relay_once().await;
        let second = relay.relay_once().await;

        // Assert
        assert_eq!((first, second), (Ok(Progress::More), Ok(Progress::Idle)));
        assert!(outbox.unpublished().is_empty());
    }

    #[tokio::test]
    async fn should_fail_the_pass_when_the_outbox_cannot_be_read() {
        // Arrange
        let outbox = FakeOutbox::with(&[envelope("product_created")]).unreadable();
        let publisher = FakePublisher::accepting(usize::MAX);
        let relay = Relay::new(outbox, publisher.clone());

        // Act
        let result = relay.relay_once().await;

        // Assert
        assert_eq!(result, Err(TransportError::Unavailable));
        assert!(publisher.published().is_empty());
    }

    #[test]
    fn should_read_at_least_one_row_when_the_batch_size_is_zero() {
        let relay =
            Relay::new(FakeOutbox::with(&[]), FakePublisher::accepting(0)).with_batch_size(0);
        assert_eq!(relay.batch_size, 1);
    }
}
