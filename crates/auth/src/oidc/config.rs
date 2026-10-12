//! The OIDC configuration, read once at startup from the environment (and `.env`).
//!
//! | Variable | | |
//! |---|---|---|
//! | `AUTH_ISSUER` | required | `http://localhost:8180/realms/shop` (Keycloak), `https://shop.eu.auth0.com/` (Auth0) |
//! | `AUTH_AUDIENCE` | required | the API's audience: `api` (the realm's audience mapper), Auth0's API identifier |
//! | `AUTH_ROLES_CLAIM` | `realm_access.roles` | where the roles are — see [`RolesClaim`] |

use super::RolesClaim;

/// An auth variable that is missing or set to a value the authenticator cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OidcConfigError {
    #[error("{key} is not set — set it in .env (.env.example has one)")]
    Missing { key: &'static str },
    #[error("{key} is not valid: expected {expected}")]
    Invalid {
        key: &'static str,
        expected: &'static str,
    },
}

/// Built by [`OidcConfig::from_env`], or [`OidcConfig::from_vars`] (tests).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub roles_claim: RolesClaim,
}

impl OidcConfig {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            issuer: issuer.into(),
            audience: audience.into(),
            roles_claim: RolesClaim::default(),
        }
    }

    pub fn with_roles_claim(mut self, roles_claim: RolesClaim) -> Self {
        self.roles_claim = roles_claim;
        self
    }

    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, OidcConfigError> {
        dotenvy::dotenv().ok();
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, OidcConfigError> {
        let read = |key: &'static str| {
            var(key)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let issuer = read("AUTH_ISSUER").ok_or(OidcConfigError::Missing { key: "AUTH_ISSUER" })?;
        if !(issuer.starts_with("https://") || issuer.starts_with("http://")) {
            return Err(OidcConfigError::Invalid {
                key: "AUTH_ISSUER",
                expected: "the provider's issuer URL, like https://shop.eu.auth0.com/",
            });
        }
        let audience = read("AUTH_AUDIENCE").ok_or(OidcConfigError::Missing {
            key: "AUTH_AUDIENCE",
        })?;
        let roles_claim = read("AUTH_ROLES_CLAIM")
            .map(RolesClaim::new)
            .unwrap_or_default();
        Ok(Self::new(issuer, audience).with_roles_claim(roles_claim))
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
        let config = OidcConfig::from_vars(env);

        // Assert
        assert_eq!(
            config,
            Ok(OidcConfig::new("http://localhost:8180/realms/shop", "api"))
        );
        assert_eq!(
            config.map(|c| c.roles_claim),
            Ok(RolesClaim::new("realm_access.roles"))
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
        let config = OidcConfig::from_vars(env);

        // Assert
        assert_eq!(
            config.map(|c| c.roles_claim),
            Ok(RolesClaim::new("https://shop.example.com/roles"))
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
                OidcConfig::from_vars(vars(env)),
                Err(OidcConfigError::Missing { key })
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
        let config = OidcConfig::from_vars(env);

        // Assert
        assert!(matches!(
            config,
            Err(OidcConfigError::Invalid {
                key: "AUTH_ISSUER",
                ..
            })
        ));
    }
}
