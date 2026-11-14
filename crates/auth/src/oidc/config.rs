//! The OIDC configuration, read once at startup from the environment (and `.env`).
//!
//! | Variable | | |
//! |---|---|---|
//! | `AUTH_ISSUER` | required | `http://localhost:8180/realms/shop` (Keycloak), `https://shop.eu.auth0.com/` (Auth0) |
//! | `AUTH_AUDIENCE` | required | the API's audience: `api` (the realm's audience mapper), Auth0's API identifier |
//! | `AUTH_ROLES_CLAIM` | `realm_access.roles` | where the roles are — see [`ClaimRoles`] |

use super::ClaimRoles;

/// An auth variable that is missing or set to a value the authenticator cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorConfigOidc {
    #[error("{clave} no está definida: defínela en .env (.env.example tiene una)")]
    Falta { clave: &'static str },
    #[error("{clave} no es válida: se esperaba {esperado}")]
    Invalido {
        clave: &'static str,
        esperado: &'static str,
    },
}

/// Built by [`ConfigOidc::from_env`], or [`ConfigOidc::from_variables`] (tests).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConfigOidc {
    pub emisor: String,
    pub audiencia: String,
    pub claim_roles: ClaimRoles,
}

impl ConfigOidc {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            emisor: issuer.into(),
            audiencia: audience.into(),
            claim_roles: ClaimRoles::default(),
        }
    }

    pub fn con_claim_roles(mut self, claim_roles: ClaimRoles) -> Self {
        self.claim_roles = claim_roles;
        self
    }

    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ErrorConfigOidc> {
        dotenvy::dotenv().ok();
        Self::from_variables(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_variables(var: impl Fn(&str) -> Option<String>) -> Result<Self, ErrorConfigOidc> {
        let read = |key: &'static str| {
            var(key)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let issuer = read("AUTH_ISSUER").ok_or(ErrorConfigOidc::Falta {
            clave: "AUTH_ISSUER",
        })?;
        if !(issuer.starts_with("https://") || issuer.starts_with("http://")) {
            return Err(ErrorConfigOidc::Invalido {
                clave: "AUTH_ISSUER",
                esperado: "la URL del emisor del proveedor, como https://shop.eu.auth0.com/",
            });
        }
        let audience = read("AUTH_AUDIENCE").ok_or(ErrorConfigOidc::Falta {
            clave: "AUTH_AUDIENCE",
        })?;
        let claim_roles = read("AUTH_ROLES_CLAIM")
            .map(ClaimRoles::new)
            .unwrap_or_default();
        Ok(Self::new(issuer, audience).con_claim_roles(claim_roles))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn should_default_to_keycloak_realm_roles_when_no_roles_claim_is_set() {
        // Arrange
        let env = vars(&[
            ("AUTH_ISSUER", "http://localhost:8180/realms/shop"),
            ("AUTH_AUDIENCE", "api"),
        ]);

        // Act
        let config = ConfigOidc::from_variables(env);

        // Assert
        assert_eq!(
            config,
            Ok(ConfigOidc::new("http://localhost:8180/realms/shop", "api"))
        );
        assert_eq!(
            config.map(|c| c.claim_roles),
            Ok(ClaimRoles::new("realm_access.roles"))
        );
    }

    #[test]
    fn should_read_the_roles_claim_when_it_is_set() {
        // Arrange
        let env = vars(&[
            ("AUTH_ISSUER", "https://shop.eu.auth0.com/"),
            ("AUTH_AUDIENCE", "https://api.shop.example.com"),
            ("AUTH_ROLES_CLAIM", "https://shop.example.com/roles"),
        ]);

        // Act
        let config = ConfigOidc::from_variables(env);

        // Assert
        assert_eq!(
            config.map(|c| c.claim_roles),
            Ok(ClaimRoles::new("https://shop.example.com/roles"))
        );
    }

    #[test]
    fn should_reject_the_config_when_a_required_variable_is_missing_or_empty() {
        let cases: [(&[(&str, &str)], &str); 3] = [
            (&[("AUTH_AUDIENCE", "api")], "AUTH_ISSUER"),
            (
                &[("AUTH_ISSUER", "  "), ("AUTH_AUDIENCE", "api")],
                "AUTH_ISSUER",
            ),
            (
                &[("AUTH_ISSUER", "https://id.example.com")],
                "AUTH_AUDIENCE",
            ),
        ];
        for (env, key) in cases {
            assert_eq!(
                ConfigOidc::from_variables(vars(env)),
                Err(ErrorConfigOidc::Falta { clave: key })
            );
        }
    }

    #[test]
    fn should_reject_the_config_when_the_issuer_is_not_a_url() {
        // Arrange
        let env = vars(&[
            ("AUTH_ISSUER", "shop.eu.auth0.com"),
            ("AUTH_AUDIENCE", "api"),
        ]);

        // Act
        let config = ConfigOidc::from_variables(env);

        // Assert
        assert!(matches!(
            config,
            Err(ErrorConfigOidc::Invalido {
                clave: "AUTH_ISSUER",
                ..
            })
        ));
    }
}
