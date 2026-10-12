//! The HTTP shell every Puerto project shares (ADR-0017): the server with graceful shutdown, the
//! health probes, the configuration read from the environment, CORS, the error body and bearer
//! auth.
//!
//! A generated project calls it; it never calls the project. `main.rs` is
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() -> Result<(), puerto_web::Error> {
//!     let config = puerto_web::Config::from_env()?;
//!     puerto_web::App::new(config).try_serve(generated::bootstrap::build_app).await
//! }
//! ```
//!
//! and `generated/bootstrap.rs` — the composition root — builds the routes with [`routes`].

pub mod app;
pub mod auth;
pub mod config;
pub mod error_response;
pub mod health;

pub use app::{App, routes};
pub use config::{Config, ConfigError};
pub use error_response::{ErrorResponse, IntoErrorResponse};
pub use health::ReadinessProbe;
/// `#[require_role("admin")]` on a handler that takes `auth: BearerAuth` — see [`auth`].
pub use puerto_macros::require_role;

/// Why the application did not start or stopped serving.
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("server: {0}")]
    Server(#[from] std::io::Error),
    /// What `build` answered instead of the routes (`App::try_serve`): a database that is down, a
    /// variable that is missing. Its message is the whole report.
    #[error("{0}")]
    Startup(String),
}

/// `main` returns this error, and Rust prints what `main` returns with `Debug`: the message is what
/// the person starting the server needs to read.
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_print_the_readable_message_when_main_returns_the_error() {
        // Arrange
        let error = Error::from(Config::from_vars(|_| Some("abc".into())).unwrap_err());

        // Act
        let printed = format!("{error:?}");

        // Assert
        assert_eq!(
            printed,
            "SERVICE_PORT=\"abc\" is not valid: expected a port number (0-65535)"
        );
    }
}
