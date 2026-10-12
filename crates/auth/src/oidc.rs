//! The OIDC adapter (ADR-0021 § 1): an OAuth2 resource server. Auth0, Keycloak and any other
//! OpenID Connect provider differ only in configuration — the issuer, the audience and where the
//! roles are ([`RolesClaim`]).

mod authenticator;
mod claims;
mod config;

pub use authenticator::{OidcAuthenticator, OidcError};
pub use claims::RolesClaim;
pub use config::{OidcConfig, OidcConfigError};
