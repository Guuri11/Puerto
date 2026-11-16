//! Puerto's auth runtime (ADR-0021): the adapters of `puerto_nucleo::auth::Autenticador`.
//!
//! - feature `oidc` — [`oidc::AutenticadorOidc`]: verifies the access tokens of an OpenID Connect
//!   provider (Auth0, Keycloak, …) and reads the caller's roles from a configurable claim. Which
//!   provider is configuration, not code.
//! - feature `local` — [`local::HasherArgon2`]: the `HasherContrasena` of a project that keeps its own
//!   accounts. Its sessions, and the `Autenticador` that reads them, are `puerto-sqlx`'s.
//!
//! A route never sees which one checked its caller: `puerto_web::auth::BearerAuth` asks the
//! `Arc<dyn Autenticador>` that `bootstrap.rs` registers.

#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "oidc")]
pub mod oidc;

#[cfg(feature = "oidc")]
pub use oidc::{AutenticadorOidc, ClaimRoles, ConfigOidc, ErrorConfigOidc, ErrorOidc};
pub use puerto_nucleo::auth::{Autenticador, ErrorAuth, Principal};
