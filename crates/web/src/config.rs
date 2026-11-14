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
pub const PUERTO_POR_DEFECTO: u16 = 8080;

/// A configuration variable that is set to a value the application cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorConfig {
    #[error("{clave}={valor:?} is not valid: expected {esperado}")]
    Invalido {
        clave: &'static str,
        valor: String,
        esperado: &'static str,
    },
}

/// Loads `.env` into the process environment when there is one; a variable already set wins.
pub fn cargar_dotenv() {
    dotenvy::dotenv().ok();
}

/// Built by [`Config::from_env`], or from [`Config::default`] with the `with_*` methods (tests).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    puerto: u16,
    origenes_cors_permitidos: Vec<String>,
    banner: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            puerto: PUERTO_POR_DEFECTO,
            origenes_cors_permitidos: vec![],
            banner: true,
        }
    }
}

impl Config {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ErrorConfig> {
        cargar_dotenv();
        Self::from_variables(|clave| std::env::var(clave).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_variables(var: impl Fn(&str) -> Option<String>) -> Result<Self, ErrorConfig> {
        let var = |clave: &str| var(clave).filter(|valor| !valor.trim().is_empty());
        let mut config = Self::default();
        if let Some(valor) = var("SERVICE_PORT") {
            config.puerto = valor.trim().parse().map_err(|_| ErrorConfig::Invalido {
                clave: "SERVICE_PORT",
                valor,
                esperado: "a port number (0-65535)",
            })?;
        }
        if let Some(valor) = var("CORS_ALLOWED_ORIGINS") {
            config.origenes_cors_permitidos = valor
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(String::from)
                .collect();
        }
        if let Some(valor) = var("PUERTO_BANNER") {
            config.banner = match valor.trim() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err(ErrorConfig::Invalido {
                        clave: "PUERTO_BANNER",
                        valor,
                        esperado: "true or false",
                    });
                }
            };
        }
        Ok(config)
    }

    pub fn con_puerto(mut self, puerto: u16) -> Self {
        self.puerto = puerto;
        self
    }

    pub fn con_origenes_cors_permitidos(mut self, origins: Vec<String>) -> Self {
        self.origenes_cors_permitidos = origins;
        self
    }

    pub fn con_banner(mut self, banner: bool) -> Self {
        self.banner = banner;
        self
    }

    pub fn puerto(&self) -> u16 {
        self.puerto
    }

    pub fn origenes_cors_permitidos(&self) -> &[String] {
        &self.origenes_cors_permitidos
    }

    pub fn banner(&self) -> bool {
        self.banner
    }

    /// The CORS middleware: the configured origins, or any origin when none is configured.
    pub fn cors(&self) -> Cors {
        self.origenes_cors_permitidos
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
        move |clave| {
            pairs
                .iter()
                .find(|(k, _)| k == clave)
                .map(|(_, v)| v.clone())
        }
    }

    #[test]
    fn should_use_the_defaults_when_nothing_is_set() {
        // Act
        let config = Config::from_variables(vars(&[])).unwrap();

        // Assert
        assert_eq!(config.puerto(), 8080);
        assert!(config.origenes_cors_permitidos().is_empty());
        assert!(config.banner());
    }

    #[test]
    fn should_read_every_variable_when_set() {
        // Act
        let config = Config::from_variables(vars(&[
            ("SERVICE_PORT", "3000"),
            (
                "CORS_ALLOWED_ORIGINS",
                "http://localhost:5173, https://app.example.com,",
            ),
            ("PUERTO_BANNER", "false"),
        ]))
        .unwrap();

        // Assert
        assert_eq!(config.puerto(), 3000);
        assert_eq!(
            config.origenes_cors_permitidos(),
            ["http://localhost:5173", "https://app.example.com"]
        );
        assert!(!config.banner());
    }

    #[test]
    fn should_reject_the_port_when_it_is_not_a_number() {
        // Act
        let result = Config::from_variables(vars(&[("SERVICE_PORT", "abc")]));

        // Assert
        assert_eq!(
            result.unwrap_err(),
            ErrorConfig::Invalido {
                clave: "SERVICE_PORT",
                valor: "abc".into(),
                esperado: "a port number (0-65535)",
            }
        );
    }

    #[test]
    fn should_reject_the_port_when_it_is_out_of_range() {
        // Act
        let result = Config::from_variables(vars(&[("SERVICE_PORT", "70000")]));

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn should_reject_the_banner_when_it_is_not_a_boolean() {
        // Act
        let result = Config::from_variables(vars(&[("PUERTO_BANNER", "no")]));

        // Assert
        assert!(matches!(
            result,
            Err(ErrorConfig::Invalido {
                clave: "PUERTO_BANNER",
                ..
            })
        ));
    }

    #[test]
    fn should_show_the_banner_when_it_is_switched_on() {
        for on in ["true", "1"] {
            // Act
            let config = Config::from_variables(vars(&[("PUERTO_BANNER", on)])).unwrap();

            // Assert
            assert!(config.banner(), "{on}");
        }
    }

    #[test]
    fn should_override_the_defaults_when_built_by_hand() {
        // Act
        let config = Config::default()
            .con_puerto(3000)
            .con_banner(false)
            .con_origenes_cors_permitidos(vec!["http://localhost:5173".into()]);

        // Assert
        assert_eq!(config.puerto(), 3000);
        assert!(!config.banner());
        assert_eq!(config.origenes_cors_permitidos(), ["http://localhost:5173"]);
    }

    #[test]
    fn should_treat_an_empty_value_as_unset() {
        // Act
        let config =
            Config::from_variables(vars(&[("SERVICE_PORT", ""), ("PUERTO_BANNER", "")])).unwrap();

        // Assert
        assert_eq!(config.puerto(), PUERTO_POR_DEFECTO);
        assert!(config.banner());
    }

    #[test]
    fn should_say_which_variable_is_wrong() {
        // Act
        let mensaje = Config::from_variables(vars(&[("SERVICE_PORT", "abc")]))
            .unwrap_err()
            .to_string();

        // Assert
        assert_eq!(
            mensaje,
            "SERVICE_PORT=\"abc\" is not valid: expected a port number (0-65535)"
        );
    }
}
