//! The application's configuration, read once at startup from the environment (and `.env`).
//!
//! | Variable | Default | |
//! |---|---|---|
//! | `SERVICE_PORT` | `8080` | the port the server listens on |
//! | `CORS_ALLOWED_ORIGINS` | none: any origin | comma-separated origins a browser may call from |
//! | `PUERTO_BANNER` | `true` | `false` hides the startup banner |
//!
//! A value that is set but invalid is an error at startup, never a silent default: a typo in
//! `SERVICE_PORT` would otherwise start the server on a port nobody routes to.

use poem::middleware::Cors;

/// The port the server listens on when `SERVICE_PORT` is unset.
pub const DEFAULT_PORT: u16 = 8080;

/// A configuration variable that is set to a value the application cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("{key}={value:?} is not valid: expected {expected}")]
    Invalid {
        key: &'static str,
        value: String,
        expected: &'static str,
    },
}

/// Built by [`Config::from_env`], or from [`Config::default`] with the `with_*` methods (tests).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    port: u16,
    cors_allowed_origins: Vec<String>,
    banner: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            cors_allowed_origins: vec![],
            banner: true,
        }
    }
}

impl Config {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let var = |key: &str| var(key).filter(|value| !value.trim().is_empty());
        let mut config = Self::default();
        if let Some(value) = var("SERVICE_PORT") {
            config.port = value.trim().parse().map_err(|_| ConfigError::Invalid {
                key: "SERVICE_PORT",
                value,
                expected: "a port number (0-65535)",
            })?;
        }
        if let Some(value) = var("CORS_ALLOWED_ORIGINS") {
            config.cors_allowed_origins = value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(String::from)
                .collect();
        }
        if let Some(value) = var("PUERTO_BANNER") {
            config.banner = match value.trim() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err(ConfigError::Invalid {
                        key: "PUERTO_BANNER",
                        value,
                        expected: "true or false",
                    });
                }
            };
        }
        Ok(config)
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_cors_allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.cors_allowed_origins = origins;
        self
    }

    pub fn with_banner(mut self, banner: bool) -> Self {
        self.banner = banner;
        self
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn cors_allowed_origins(&self) -> &[String] {
        &self.cors_allowed_origins
    }

    pub fn banner(&self) -> bool {
        self.banner
    }

    /// The CORS middleware: the configured origins, or any origin when none is configured.
    pub fn cors(&self) -> Cors {
        self.cors_allowed_origins
            .iter()
            .fold(Cors::new(), |cors, origin| {
                cors.allow_origin(origin.as_str())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn should_use_the_defaults_when_nothing_is_set() {
        // Act
        let config = Config::from_vars(vars(&[])).unwrap();

        // Assert
        assert_eq!(config.port(), 8080);
        assert!(config.cors_allowed_origins().is_empty());
        assert!(config.banner());
    }

    #[test]
    fn should_read_every_variable_when_set() {
        // Act
        let config = Config::from_vars(vars(&[
            ("SERVICE_PORT", "3000"),
            (
                "CORS_ALLOWED_ORIGINS",
                "http://localhost:5173, https://app.example.com,",
            ),
            ("PUERTO_BANNER", "false"),
        ]))
        .unwrap();

        // Assert
        assert_eq!(config.port(), 3000);
        assert_eq!(
            config.cors_allowed_origins(),
            ["http://localhost:5173", "https://app.example.com"]
        );
        assert!(!config.banner());
    }

    #[test]
    fn should_reject_the_port_when_it_is_not_a_number() {
        // Act
        let result = Config::from_vars(vars(&[("SERVICE_PORT", "abc")]));

        // Assert
        assert_eq!(
            result.unwrap_err(),
            ConfigError::Invalid {
                key: "SERVICE_PORT",
                value: "abc".into(),
                expected: "a port number (0-65535)",
            }
        );
    }

    #[test]
    fn should_reject_the_port_when_it_is_out_of_range() {
        // Act
        let result = Config::from_vars(vars(&[("SERVICE_PORT", "70000")]));

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_the_banner_when_it_is_not_a_boolean() {
        // Act
        let result = Config::from_vars(vars(&[("PUERTO_BANNER", "no")]));

        // Assert
        assert!(matches!(
            result,
            Err(ConfigError::Invalid {
                key: "PUERTO_BANNER",
                ..
            })
        ));
    }

    #[test]
    fn should_show_the_banner_when_it_is_switched_on() {
        for on in ["true", "1"] {
            // Act
            let config = Config::from_vars(vars(&[("PUERTO_BANNER", on)])).unwrap();

            // Assert
            assert!(config.banner(), "{on}");
        }
    }

    #[test]
    fn should_override_the_defaults_when_built_by_hand() {
        // Act
        let config = Config::default()
            .with_port(3000)
            .with_banner(false)
            .with_cors_allowed_origins(vec!["http://localhost:5173".into()]);

        // Assert
        assert_eq!(config.port(), 3000);
        assert!(!config.banner());
        assert_eq!(config.cors_allowed_origins(), ["http://localhost:5173"]);
    }

    #[test]
    fn should_treat_an_empty_value_as_unset() {
        // Act
        let config =
            Config::from_vars(vars(&[("SERVICE_PORT", ""), ("PUERTO_BANNER", "")])).unwrap();

        // Assert
        assert_eq!(config.port(), DEFAULT_PORT);
        assert!(config.banner());
    }

    #[test]
    fn should_say_which_variable_is_wrong() {
        // Act
        let message = Config::from_vars(vars(&[("SERVICE_PORT", "abc")]))
            .unwrap_err()
            .to_string();

        // Assert
        assert_eq!(
            message,
            "SERVICE_PORT=\"abc\" is not valid: expected a port number (0-65535)"
        );
    }
}
