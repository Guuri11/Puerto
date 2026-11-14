//! The worker's configuration, read once at startup from the environment (and `.env`).
//!
//! | Variable | Default | |
//! |---|---|---|
//! | `WORKER_PORT` | `8081` | the port `/health` and `/health/ready` are served on |
//! | `RELAY_BATCH_SIZE` | `100` | outbox rows one relay pass reads |
//! | `RELAY_POLL_INTERVAL_MS` | `500` | the relay's pause when the outbox is drained |
//! | `CONSUMER_POLL_INTERVAL_MS` | `500` | a consumer's pause when nothing is due |
//! | `MAX_DELIVERIES` | `5` | deliveries an event gets before it is dead-lettered |
//! | `RETRY_MAX_DELAY_MS` | none: 1 s, 5 s, 15 s, 60 s, 300 s | caps the wait between redeliveries |
//!
//! As with the API's `puerto_web::Config`, a value that is set but invalid is an error at startup.

use std::time::Duration;

use puerto_nucleo::eventos::entrega::ENTREGAS_MAXIMAS_POR_DEFECTO;
use puerto_web::ErrorConfig;

use crate::relay::TAMANO_LOTE_RELAY_POR_DEFECTO;

/// The port the worker's health probes listen on when `WORKER_PORT` is unset — the API's is 8080.
pub const PUERTO_WORKER_POR_DEFECTO: u16 = 8081;
/// The default pause of the relay and of each consumer, in milliseconds.
pub const INTERVALO_SONDEO_MS_POR_DEFECTO: u64 = 500;

/// Built by [`ConfigWorker::from_env`], or from [`ConfigWorker::default`] (tests).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConfigWorker {
    pub port: u16,
    pub tamano_lote_relay: u32,
    pub intervalo_sondeo_relay: Duration,
    pub intervalo_sondeo_consumidor: Duration,
    pub max_deliveries: u32,
    pub espera_maxima_reintento: Option<Duration>,
}

impl Default for ConfigWorker {
    fn default() -> Self {
        Self {
            port: PUERTO_WORKER_POR_DEFECTO,
            tamano_lote_relay: TAMANO_LOTE_RELAY_POR_DEFECTO,
            intervalo_sondeo_relay: Duration::from_millis(INTERVALO_SONDEO_MS_POR_DEFECTO),
            intervalo_sondeo_consumidor: Duration::from_millis(INTERVALO_SONDEO_MS_POR_DEFECTO),
            max_deliveries: ENTREGAS_MAXIMAS_POR_DEFECTO,
            espera_maxima_reintento: None,
        }
    }
}

impl ConfigWorker {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ErrorConfig> {
        puerto_web::config::cargar_dotenv();
        Self::from_variables(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_variables(var: impl Fn(&str) -> Option<String>) -> Result<Self, ErrorConfig> {
        let var = |key: &str| var(key).filter(|value| !value.trim().is_empty());
        let mut config = Self::default();
        if let Some(value) = var("WORKER_PORT") {
            config.port = parse(value, "WORKER_PORT", "un número de puerto (0-65535)")?;
        }
        if let Some(value) = var("RELAY_BATCH_SIZE") {
            config.tamano_lote_relay = positive(value, "RELAY_BATCH_SIZE")?;
        }
        if let Some(value) = var("RELAY_POLL_INTERVAL_MS") {
            config.intervalo_sondeo_relay = millis(value, "RELAY_POLL_INTERVAL_MS")?;
        }
        if let Some(value) = var("CONSUMER_POLL_INTERVAL_MS") {
            config.intervalo_sondeo_consumidor = millis(value, "CONSUMER_POLL_INTERVAL_MS")?;
        }
        if let Some(value) = var("MAX_DELIVERIES") {
            config.max_deliveries = positive(value, "MAX_DELIVERIES")?;
        }
        if let Some(value) = var("RETRY_MAX_DELAY_MS") {
            config.espera_maxima_reintento = Some(millis(value, "RETRY_MAX_DELAY_MS")?);
        }
        Ok(config)
    }

    pub fn con_puerto(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

fn parse<T: std::str::FromStr>(
    value: String,
    key: &'static str,
    expected: &'static str,
) -> Result<T, ErrorConfig> {
    value.trim().parse().map_err(|_| ErrorConfig::Invalido {
        clave: key,
        valor: value,
        esperado: expected,
    })
}

fn positive(value: String, key: &'static str) -> Result<u32, ErrorConfig> {
    let expected = "un número entero de al menos 1";
    match parse::<u32>(value.clone(), key, expected)? {
        0 => Err(ErrorConfig::Invalido {
            clave: key,
            valor: value,
            esperado: expected,
        }),
        n => Ok(n),
    }
}

fn millis(value: String, key: &'static str) -> Result<Duration, ErrorConfig> {
    parse(value, key, "un número de milisegundos").map(Duration::from_millis)
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
        let config = ConfigWorker::from_variables(vars(&[]));

        // Assert
        assert_eq!(
            config,
            Ok(ConfigWorker {
                port: 8081,
                tamano_lote_relay: 100,
                intervalo_sondeo_relay: Duration::from_millis(500),
                intervalo_sondeo_consumidor: Duration::from_millis(500),
                max_deliveries: 5,
                espera_maxima_reintento: None,
            })
        );
    }

    #[test]
    fn should_read_every_variable_when_set() {
        // Act
        let config = ConfigWorker::from_variables(vars(&[
            ("WORKER_PORT", "9090"),
            ("RELAY_BATCH_SIZE", "10"),
            ("RELAY_POLL_INTERVAL_MS", "50"),
            ("CONSUMER_POLL_INTERVAL_MS", " 75 "),
            ("MAX_DELIVERIES", "3"),
            ("RETRY_MAX_DELAY_MS", "0"),
        ]));

        // Assert
        assert_eq!(
            config,
            Ok(ConfigWorker {
                port: 9090,
                tamano_lote_relay: 10,
                intervalo_sondeo_relay: Duration::from_millis(50),
                intervalo_sondeo_consumidor: Duration::from_millis(75),
                max_deliveries: 3,
                espera_maxima_reintento: Some(Duration::ZERO),
            })
        );
    }

    #[test]
    fn should_reject_the_port_when_it_is_not_a_number() {
        // Act
        let result = ConfigWorker::from_variables(vars(&[("WORKER_PORT", "abc")]));

        // Assert
        assert_eq!(
            result,
            Err(ErrorConfig::Invalido {
                clave: "WORKER_PORT",
                valor: "abc".into(),
                esperado: "un número de puerto (0-65535)",
            })
        );
    }

    #[test]
    fn should_reject_a_count_when_it_is_zero() {
        for key in ["RELAY_BATCH_SIZE", "MAX_DELIVERIES"] {
            // Act
            let result = ConfigWorker::from_variables(vars(&[(key, "0")]));

            // Assert
            assert!(
                matches!(result, Err(ErrorConfig::Invalido { clave: k, .. }) if k == key),
                "{key}: {result:?}"
            );
        }
    }

    #[test]
    fn should_reject_an_interval_when_it_is_negative() {
        // Act
        let result = ConfigWorker::from_variables(vars(&[("RELAY_POLL_INTERVAL_MS", "-1")]));

        // Assert
        assert!(matches!(
            result,
            Err(ErrorConfig::Invalido {
                clave: "RELAY_POLL_INTERVAL_MS",
                esperado: "un número de milisegundos",
                ..
            })
        ));
    }

    #[test]
    fn should_override_the_port_when_built_by_hand() {
        assert_eq!(ConfigWorker::default().con_puerto(9191).port, 9191);
    }

    #[test]
    fn should_treat_an_empty_value_as_unset() {
        // Act
        let config =
            ConfigWorker::from_variables(vars(&[("WORKER_PORT", " "), ("MAX_DELIVERIES", "")]));

        // Assert
        assert_eq!(config, Ok(ConfigWorker::default()));
    }
}
