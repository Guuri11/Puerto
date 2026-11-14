//! Puerto's mail runtime (ADR-0020): what every generated mail shares. A project declares one
//! port per mail in its domain (`WelcomeMailer`); the implementation renders that mail's
//! [`Templates`] and hands the [`puerto_nucleo::mail::Email`] to a [`SmtpMailer`]. A worker consumer
//! sends it through [`send_once`], so a redelivered event does not send it twice.

pub mod config;
pub mod send_once;
pub mod smtp;
pub mod templates;
#[cfg(any(test, feature = "test-utils"))]
pub mod testing;

pub use config::{MailConfig, MailConfigError};
pub use puerto_nucleo::correo::{Correo, CorreosEnviados, EnviadorCorreo, ErrorCorreo};
pub use send_once::send_once;
pub use smtp::SmtpMailer;
pub use templates::Templates;
