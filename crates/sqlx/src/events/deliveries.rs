//! The `db` transport: no broker, one `event_deliveries` row per event and subscribed consumer.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use puerto_core::EventEnvelope;
use puerto_core::events::delivery::Delivery;
use puerto_core::events::transport::{Claimed, EventPublisher, EventSubscription, TransportError};
use sqlx::PgPool;

use super::{ENVELOPE_COLUMNS, envelope_from_pg, unavailable};

/// How long a claimed delivery stays invisible to other claims. A worker that dies while
/// handling one leaves it due again after this — at-least-once, never lost.
pub const DEFAULT_CLAIM_LEASE: Duration = Duration::from_secs(60);

/// The relay's publisher on the `db` transport: fans each event out to the consumers subscribed
/// to its type and marks it published, in one transaction.
pub struct PgDeliveries {
    pool: PgPool,
    /// event type → the consumers subscribed to it, in the order they subscribed.
    routes: BTreeMap<String, Vec<String>>,
}

impl PgDeliveries {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            routes: BTreeMap::new(),
        }
    }

    /// Subscribes `consumer` to `event_types` — one `[[consumer]]` of puerto.toml.
    pub fn route(mut self, consumer: &str, event_types: &[&str]) -> Self {
        for event_type in event_types {
            let consumers = self.routes.entry((*event_type).to_string()).or_default();
            if !consumers.iter().any(|c| c == consumer) {
                consumers.push(consumer.to_string());
            }
        }
        self
    }

    /// Who receives an event of this type. None is fine: the event is marked published anyway.
    pub fn consumers_for(&self, event_type: &str) -> &[String] {
        self.routes.get(event_type).map_or(&[], Vec::as_slice)
    }
}

#[async_trait]
impl EventPublisher for PgDeliveries {
    async fn publish(&self, event: &EventEnvelope) -> Result<(), TransportError> {
        let context = "deliveries: publish";
        let mut tx = self.pool.begin().await.map_err(unavailable(context))?;
        sqlx::query(
            "INSERT INTO event_deliveries (event_id, consumer) \
             SELECT $1, consumer FROM unnest($2::text[]) AS consumer \
             ON CONFLICT DO NOTHING",
        )
        .bind(event.id)
        .bind(self.consumers_for(&event.event_type))
        .execute(&mut *tx)
        .await
        .map_err(unavailable(context))?;
        sqlx::query(
            "UPDATE outbox SET published_at = now() WHERE id = $1 AND published_at IS NULL",
        )
        .bind(event.id)
        .execute(&mut *tx)
        .await
        .map_err(unavailable(context))?;
        tx.commit().await.map_err(unavailable(context))
    }
}

/// One consumer's input on the `db` transport.
pub struct PgSubscription {
    pool: PgPool,
    consumer: String,
    lease: Duration,
}

impl PgSubscription {
    pub fn new(pool: PgPool, consumer: impl Into<String>) -> Self {
        Self {
            pool,
            consumer: consumer.into(),
            lease: DEFAULT_CLAIM_LEASE,
        }
    }

    /// How long a claim is kept from other workers; longer than the slowest handler.
    pub fn with_lease(mut self, lease: Duration) -> Self {
        self.lease = lease;
        self
    }
}

#[async_trait]
impl EventSubscription for PgSubscription {
    fn consumer(&self) -> &str {
        &self.consumer
    }

    /// Claims the due deliveries, oldest event first. A claim counts as a delivery (`attempts`
    /// goes up now, not when it is settled), so a worker that crashes mid-handle still uses one
    /// of the event's deliveries, and the claim is hidden for the lease.
    async fn next_batch(&mut self, max: u32) -> Result<Vec<Claimed>, TransportError> {
        let context = "deliveries: next_batch";
        let rows = sqlx::query(&format!(
            "WITH due AS ( \
                 SELECT d.event_id FROM event_deliveries d \
                 JOIN outbox o ON o.id = d.event_id \
                 WHERE d.consumer = $1 AND d.status = 'pending' AND d.next_attempt_at <= now() \
                 ORDER BY o.seq LIMIT $2 \
                 FOR UPDATE OF d SKIP LOCKED \
             ), claimed AS ( \
                 UPDATE event_deliveries d \
                 SET attempts = d.attempts + 1, \
                     next_attempt_at = now() + make_interval(secs => $3) \
                 FROM due WHERE d.event_id = due.event_id AND d.consumer = $1 \
                 RETURNING d.event_id, d.attempts \
             ) \
             SELECT {ENVELOPE_COLUMNS}, claimed.attempts FROM claimed \
             JOIN outbox o ON o.id = claimed.event_id \
             ORDER BY o.seq"
        ))
        .bind(&self.consumer)
        .bind(i64::from(max))
        .bind(self.lease.as_secs_f64())
        .fetch_all(&self.pool)
        .await
        .map_err(unavailable(context))?;
        rows.iter()
            .map(|row| {
                let envelope = envelope_from_pg(row, context)?;
                let attempts: i32 =
                    sqlx::Row::try_get(row, "attempts").map_err(|_| TransportError::InvalidData)?;
                let attempt = u32::try_from(attempts).map_err(|_| TransportError::InvalidData)?;
                Ok(Claimed::new(envelope, attempt))
            })
            .collect()
    }

    async fn settle(
        &mut self,
        claimed: &Claimed,
        delivery: &Delivery,
    ) -> Result<(), TransportError> {
        let (id, consumer) = (claimed.envelope.id, &self.consumer);
        let result = match delivery {
            Delivery::Ack => {
                sqlx::query(
                    "UPDATE event_deliveries SET status = 'done', last_error = NULL \
                     WHERE event_id = $1 AND consumer = $2",
                )
                .bind(id)
                .bind(consumer)
                .execute(&self.pool)
                .await
            }
            Delivery::Retry { delay } => {
                sqlx::query(
                    "UPDATE event_deliveries \
                     SET next_attempt_at = now() + make_interval(secs => $3) \
                     WHERE event_id = $1 AND consumer = $2",
                )
                .bind(id)
                .bind(consumer)
                .bind(delay.as_secs_f64())
                .execute(&self.pool)
                .await
            }
            Delivery::DeadLetter { reason, detail } => {
                sqlx::query(
                    "UPDATE event_deliveries SET status = 'dead', last_error = $3 \
                     WHERE event_id = $1 AND consumer = $2",
                )
                .bind(id)
                .bind(consumer)
                .bind(format!("{reason}: {detail}"))
                .execute(&self.pool)
                .await
            }
        };
        result.map_err(unavailable("deliveries: settle"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://puerto@127.0.0.1:1/puerto")
            .unwrap()
    }

    #[tokio::test]
    async fn should_route_an_event_to_every_consumer_subscribed_to_its_type() {
        let deliveries = PgDeliveries::new(lazy_pool())
            .route(
                "notify_new_product",
                &["product_created", "product_published"],
            )
            .route("audit", &["product_created"]);

        assert_eq!(
            deliveries.consumers_for("product_created"),
            ["notify_new_product", "audit"]
        );
        assert_eq!(
            deliveries.consumers_for("product_published"),
            ["notify_new_product"]
        );
    }

    #[tokio::test]
    async fn should_route_to_nobody_when_no_consumer_subscribed() {
        let deliveries = PgDeliveries::new(lazy_pool()).route("audit", &["product_created"]);

        assert!(deliveries.consumers_for("product_deleted").is_empty());
    }

    #[tokio::test]
    async fn should_route_once_when_a_consumer_subscribes_twice() {
        let deliveries = PgDeliveries::new(lazy_pool())
            .route("audit", &["product_created", "product_created"])
            .route("audit", &["product_created"]);

        assert_eq!(deliveries.consumers_for("product_created"), ["audit"]);
    }

    #[tokio::test]
    async fn should_name_its_consumer_and_default_lease_when_subscribed() {
        let subscription = PgSubscription::new(lazy_pool(), "audit");

        assert_eq!(subscription.consumer(), "audit");
        assert_eq!(subscription.lease, DEFAULT_CLAIM_LEASE);
        assert_eq!(
            subscription.with_lease(Duration::from_secs(5)).lease,
            Duration::from_secs(5)
        );
    }
}
