//! The Kafka transport of Puerto projects with domain events (`puerto new --events kafka`,
//! ADR-0018 § 4): the publisher the outbox relay of `puerto-events` hands each event to, and the
//! subscription each consumer reads. The outbox and the dead letters stay in Postgres
//! (`puerto_sqlx::events`).
//!
//! - One topic per aggregate, `<KAFKA_TOPIC_PREFIX>.<aggregate_type>`, keyed by the aggregate id:
//!   the events of one aggregate land in one partition, in the order the outbox holds them.
//! - The payload is the event's JSON payload; the rest of the envelope travels in headers.
//! - A consumer is a consumer group (`<prefix>.<consumer>`) on the topics of the event types it
//!   subscribes to; the records of other types are skipped. Its offset is committed once the
//!   outcome is settled, so what was not settled is delivered again after a restart.
//! - A `Retry` is retried in process with the delivery policy's backoff, then dead-lettered: the
//!   records behind it wait (a partition stalls), they are never handled ahead of it.
//!
//! ```ignore
//! let kafka = KafkaConfig::from_env("shop")?;
//! kafka.ensure_topics(&["product"]).await?;
//! Loops::new(&config)
//!     .relay(PgOutbox::new(pool.clone()), KafkaPublisher::new(&kafka)?)
//!     .consumer(
//!         KafkaSubscription::new(&kafka, "notify_new_product", &["product"], &[PRODUCT_CREATED], dead_letters.clone())?,
//!         NotifyNewProduct,
//!         dead_letters,
//!     )
//! ```

pub mod config;
pub mod publisher;
pub mod subscription;
mod wire;

pub use config::KafkaConfig;
pub use publisher::KafkaPublisher;
pub use rdkafka::error::KafkaError;
pub use subscription::KafkaSubscription;
