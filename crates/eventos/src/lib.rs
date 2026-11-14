//! The worker every Puerto project with domain events runs (ADR-0018 § 5): the relay that moves
//! the outbox onto the transport, one consume loop per consumer, the health probes and graceful
//! shutdown.
//!
//! It only sees the transport ports of `puerto_nucleo::events::transport`: the project's
//! `worker/src/generated/registry.rs` hands it the adapters (`puerto_sqlx::events` for the `db`
//! transport) and its consumers.
//!
//! ```ignore
//! pub async fn build_worker(config: ConfigWorker) -> Loops {
//!     Loops::new(&config)
//!         .relay(PgOutbox::new(pool.clone()), deliveries)
//!         .consumer(PgSubscription::new(pool.clone(), "notify_new_product"), NotifyNewProduct, dead_letters)
//! }
//! ```

pub mod conductor;
pub mod config;
pub mod consumidor;
pub mod relay;
pub mod worker;

#[cfg(test)]
mod pruebas;

pub use conductor::Progreso;
pub use config::ConfigWorker;
pub use consumidor::Consumidor;
pub use puerto_web::{Error, ErrorConfig};
pub use relay::Relay;
pub use worker::{Bucles, Worker};
