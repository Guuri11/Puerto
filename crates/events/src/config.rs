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

use puerto_core::events::delivery::DEFAULT_MAX_DELIVERIES;
use puerto_web::ConfigError;

use crate::relay::DEFAULT_RELAY_BATCH_SIZE;

/// The port the worker's health probes listen on when `WORKER_PORT` is unset — the API's is 8080.
pub const DEFAULT_WORKER_PORT: u16 = 8081;
/// The default pause of the relay and of each consumer, in milliseconds.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 500;

/// Built by [`WorkerConfig::from_env`], or from [`WorkerConfig::default`] (tests).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WorkerConfig {
    pub port: u16,
    pub relay_batch_size: u32,
    pub relay_poll_interval: Duration,
    pub consumer_poll_interval: Duration,
    pub max_deliveries: u32,
    pub max_retry_delay: Option<Duration>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_WORKER_PORT,
            relay_batch_size: DEFAULT_RELAY_BATCH_SIZE,
            relay_poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            consumer_poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
            max_deliveries: DEFAULT_MAX_DELIVERIES,
            max_retry_delay: None,
        }
    }
}

impl WorkerConfig {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        puerto_web::config::load_dotenv();
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let var = |key: &str| var(key).filter(|value| !value.trim().is_empty());
        let mut config = Self::default();
        if let Some(value) = var("WORKER_PORT") {
            config.port = parse(value, "WORKER_PORT", "a port number (0-65535)")?;
        }
        if let Some(value) = var("RELAY_BATCH_SIZE") {
            config.relay_batch_size = positive(value, "RELAY_BATCH_SIZE")?;
        }
        if let Some(value) = var("RELAY_POLL_INTERVAL_MS") {
            config.relay_poll_interval = millis(value, "RELAY_POLL_INTERVAL_MS")?;
        }
        if let Some(value) = var("CONSUMER_POLL_INTERVAL_MS") {
            config.consumer_poll_interval = millis(value, "CONSUMER_POLL_INTERVAL_MS")?;
        }
        if let Some(value) = var("MAX_DELIVERIES") {
            config.max_deliveries = positive(value, "MAX_DELIVERIES")?;
        }
        if let Some(value) = var("RETRY_MAX_DELAY_MS") {
            config.max_retry_delay = Some(millis(value, "RETRY_MAX_DELAY_MS")?);
        }
        Ok(config)
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

fn parse<T: std::str::FromStr>(
    value: String,
    key: &'static str,
    expected: &'static str,
) -> Result<T, ConfigError> {
    value.trim().parse().map_err(|_| ConfigError::Invalid {
        key,
        value,
        expected,
    })
}

fn positive(value: String, key: &'static str) -> Result<u32, ConfigError> {
    let expected = "a whole number of at least 1";
    match parse::<u32>(value.clone(), key, expected)? {
        0 => Err(ConfigError::Invalid {
            key,
            value,
            expected,
        }),
        n => Ok(n),
    }
}

fn millis(value: String, key: &'static str) -> Result<Duration, ConfigError> {
    parse(value, key, "a number of milliseconds").map(Duration::from_millis)
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
        let config = WorkerConfig::from_vars(vars(&[]));

        // Assert
        assert_eq!(
            config,
            Ok(WorkerConfig {
                port: 8081,
                relay_batch_size: 100,
                relay_poll_interval: Duration::from_millis(500),
                consumer_poll_interval: Duration::from_millis(500),
                max_deliveries: 5,
                max_retry_delay: None,
            })
        );
    }

    #[test]
    fn should_read_every_variable_when_set() {
        // Act
        let config = WorkerConfig::from_vars(vars(&[
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
            Ok(WorkerConfig {
                port: 9090,
                relay_batch_size: 10,
                relay_poll_interval: Duration::from_millis(50),
                consumer_poll_interval: Duration::from_millis(75),
                max_deliveries: 3,
                max_retry_delay: Some(Duration::ZERO),
            })
        );
    }

    #[test]
    fn should_reject_the_port_when_it_is_not_a_number() {
        // Act
        let result = WorkerConfig::from_vars(vars(&[("WORKER_PORT", "abc")]));

        // Assert
        assert_eq!(
            result,
            Err(ConfigError::Invalid {
                key: "WORKER_PORT",
                value: "abc".into(),
                expected: "a port number (0-65535)",
            })
        );
    }

    #[test]
    fn should_reject_a_count_when_it_is_zero() {
        for key in ["RELAY_BATCH_SIZE", "MAX_DELIVERIES"] {
            // Act
            let result = WorkerConfig::from_vars(vars(&[(key, "0")]));

            // Assert
            assert!(
                matches!(result, Err(ConfigError::Invalid { key: k, .. }) if k == key),
                "{key}: {result:?}"
            );
        }
    }

    #[test]
    fn should_reject_an_interval_when_it_is_negative() {
        // Act
        let result = WorkerConfig::from_vars(vars(&[("RELAY_POLL_INTERVAL_MS", "-1")]));

        // Assert
        assert!(matches!(
            result,
            Err(ConfigError::Invalid {
                key: "RELAY_POLL_INTERVAL_MS",
                expected: "a number of milliseconds",
                ..
            })
        ));
    }

    #[test]
    fn should_override_the_port_when_built_by_hand() {
        assert_eq!(WorkerConfig::default().with_port(9191).port, 9191);
    }

    #[test]
    fn should_treat_an_empty_value_as_unset() {
        // Act
        let config = WorkerConfig::from_vars(vars(&[("WORKER_PORT", " "), ("MAX_DELIVERIES", "")]));

        // Assert
        assert_eq!(config, Ok(WorkerConfig::default()));
    }
}
