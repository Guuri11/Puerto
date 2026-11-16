//! The worker every Puerto project with domain events runs (ADR-0018 § 5): the relay that moves
//! the outbox onto the transport, one consume loop per consumer, the health probes and graceful
//! shutdown.
//!
//! It only sees the transport ports of `puerto_nucleo::eventos::transporte`: the project's
//! `presentacion/worker/src/generado/registro.rs` hands it the adapters (`puerto_sqlx::eventos`
//! for the `db` transport) and its consumers.
//!
//! ```ignore
//! pub async fn construir_worker(config: ConfigWorker) -> Result<Bucles, String> {
//!     Ok(Bucles::new(&config)
//!         .relay(OutboxPostgres::new(pool.clone()), entregas)
//!         .consumidor(
//!             SuscripcionPostgres::new(pool.clone(), "avisar_producto_nuevo"),
//!             AvisarProductoNuevo,
//!             descartados,
//!         ))
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
