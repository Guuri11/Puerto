//! The worker every Puerto project with domain events runs (ADR-0018 § 5): the relay that moves
//! the outbox onto the transport, one consume loop per consumer, the health probes and graceful
//! shutdown.
//!
//! It only sees the transport ports of `puerto_core::events::transport`: the project's
//! `worker/src/generated/registry.rs` hands it the adapters (`puerto_sqlx::events` for the `db`
//! transport) and its consumers.
//!
//! ```ignore
//! pub async fn build_worker(config: WorkerConfig) -> Loops {
//!     Loops::new(&config)
//!         .relay(PgOutbox::new(pool.clone()), deliveries)
//!         .consumer(PgSubscription::new(pool.clone(), "notify_new_product"), NotifyNewProduct, dead_letters)
//! }
//! ```

pub mod config;
pub mod consumer;
pub mod driver;
pub mod relay;
pub mod worker;

#[cfg(test)]
mod test_support;

pub use config::WorkerConfig;
pub use consumer::Consumer;
pub use driver::Progress;
pub use puerto_web::{ConfigError, Error};
pub use relay::Relay;
pub use worker::{Loops, Worker};
