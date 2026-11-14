//! The OIDC adapter (ADR-0021 § 1): an OAuth2 resource server. Auth0, Keycloak and any other
//! OpenID Connect provider differ only in configuration — the issuer, the audience and where the
//! roles are ([`ClaimRoles`]).

mod authenticator;
mod claims;
mod config;

pub use authenticator::{AutenticadorOidc, ErrorOidc};
pub use claims::ClaimRoles;
pub use config::{ConfigOidc, ErrorConfigOidc};
