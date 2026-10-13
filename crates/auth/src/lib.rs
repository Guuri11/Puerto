//! Puerto's auth runtime (ADR-0021): the adapters of `puerto_core::auth::Authenticator`.
//!
//! - feature `oidc` — [`oidc::OidcAuthenticator`]: verifies the access tokens of an OpenID Connect
//!   provider (Auth0, Keycloak, …) and reads the caller's roles from a configurable claim. Which
//!   provider is configuration, not code.
//! - feature `local` — [`local::Argon2Hasher`]: the `PasswordHasher` of a project that keeps its own
//!   accounts. Its sessions, and the `Authenticator` that reads them, are `puerto-sqlx`'s.
//!
//! A route never sees which one checked its caller: `puerto_web::auth::BearerAuth` asks the
//! `Arc<dyn Authenticator>` that `bootstrap.rs` registers.

#[cfg(feature = "local")]
pub mod local;
#[cfg(feature = "oidc")]
pub mod oidc;

#[cfg(feature = "oidc")]
pub use oidc::{OidcAuthenticator, OidcConfig, OidcConfigError, OidcError, RolesClaim};
pub use puerto_core::auth::{AuthError, Authenticator, Principal};
