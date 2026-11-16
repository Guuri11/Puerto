//! Puerto's mail runtime (ADR-0020): what every generated mail shares. A project declares one
//! port per mail in its domain (`EnviadorCorreoBienvenida`); the implementation renders that mail's
//! [`Plantillas`] and hands the [`puerto_nucleo::correo::Correo`] to a [`EnviadorCorreoSmtp`]. A worker consumer
//! sends it through [`enviar_una_vez()`], so a redelivered event does not send it twice.

pub mod config;
pub mod enviar_una_vez;
pub mod plantillas;
#[cfg(any(test, feature = "pruebas"))]
pub mod pruebas;
pub mod smtp;

pub use config::{ConfigCorreo, ErrorConfigCorreo};
pub use enviar_una_vez::enviar_una_vez;
pub use plantillas::Plantillas;
pub use puerto_nucleo::correo::{Correo, CorreosEnviados, EnviadorCorreo, ErrorCorreo};
pub use smtp::EnviadorCorreoSmtp;
