//! Where the brokers are and how topics are named, read once at startup from the environment
//! (and `.env`).
//!
//! | Variable | Default | |
//! |---|---|---|
//! | `KAFKA_BROKERS` | `localhost:9092` | `bootstrap.servers`: `host:port`, comma-separated |
//! | `KAFKA_TOPIC_PREFIX` | the project name | topics are `<prefix>.<aggregate>`, groups `<prefix>.<consumer>` |

use std::time::Duration;

use puerto_web::ConfigError;
use rdkafka::ClientConfig;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::error::KafkaError;
use rdkafka::types::RDKafkaErrorCode;

/// The brokers when `KAFKA_BROKERS` is unset: the Redpanda of the project's `compose.yaml`.
pub const DEFAULT_BROKERS: &str = "localhost:9092";

/// How long creating the topics at startup may take.
const ADMIN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic_prefix: String,
}

impl KafkaConfig {
    /// Loads `.env` when there is one, then reads the process environment. `default_prefix` is
    /// the project name, which the generated registry passes.
    pub fn from_env(default_prefix: &str) -> Result<Self, ConfigError> {
        puerto_web::config::load_dotenv();
        Self::from_vars(default_prefix, |key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_vars(
        default_prefix: &str,
        var: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let var = |key: &str| var(key).filter(|value| !value.trim().is_empty());
        let brokers = var("KAFKA_BROKERS").map_or_else(
            || DEFAULT_BROKERS.to_string(),
            |value| value.trim().to_string(),
        );
        let topic_prefix = match var("KAFKA_TOPIC_PREFIX") {
            Some(value) => value.trim().to_string(),
            None => default_prefix.to_string(),
        };
        if !is_topic_name(&topic_prefix) {
            return Err(ConfigError::Invalid {
                key: "KAFKA_TOPIC_PREFIX",
                value: topic_prefix,
                expected: "letters, digits, '.', '_' or '-'",
            });
        }
        Ok(Self {
            brokers,
            topic_prefix,
        })
    }

    /// The topic an aggregate's events are published to: `shop.product`.
    pub fn topic(&self, aggregate_type: &str) -> String {
        format!("{}.{aggregate_type}", self.topic_prefix)
    }

    /// A consumer's group: `shop.notify_new_product`. Two projects on one cluster never share one.
    pub fn group(&self, consumer: &str) -> String {
        format!("{}.{consumer}", self.topic_prefix)
    }

    pub(crate) fn client(&self) -> ClientConfig {
        let mut client = ClientConfig::new();
        client.set("bootstrap.servers", &self.brokers);
        client
    }

    /// Creates the topic of each aggregate that does not exist yet, with the broker's default
    /// partitions and replication. Run once at startup, before the relay publishes.
    pub async fn ensure_topics(&self, aggregate_types: &[&str]) -> Result<(), KafkaError> {
        let admin: AdminClient<DefaultClientContext> = self.client().create()?;
        let names: Vec<String> = aggregate_types.iter().map(|a| self.topic(a)).collect();
        let topics: Vec<NewTopic<'_>> = names
            .iter()
            .map(|name| NewTopic::new(name, -1, TopicReplication::Fixed(-1)))
            .collect();
        let options = AdminOptions::new().operation_timeout(Some(ADMIN_TIMEOUT));
        for result in admin.create_topics(&topics, &options).await? {
            match result {
                Ok(_) | Err((_, RDKafkaErrorCode::TopicAlreadyExists)) => {}
                Err((topic, code)) => {
                    tracing::error!(topic, error = %code, "kafka: topic not created");
                    return Err(KafkaError::AdminOp(code));
                }
            }
        }
        Ok(())
    }
}

/// What Kafka accepts in a topic name (and so in a prefix of one).
fn is_topic_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<KafkaConfig, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        KafkaConfig::from_vars("shop", |key| vars.get(key).cloned())
    }

    #[test]
    fn should_use_the_local_broker_and_the_project_name_when_nothing_is_set() {
        let config = config(&[]).unwrap();
        assert_eq!(
            (config.brokers.as_str(), config.topic_prefix.as_str()),
            ("localhost:9092", "shop")
        );
    }

    #[test]
    fn should_read_the_brokers_and_the_prefix_when_they_are_set() {
        let config = config(&[
            ("KAFKA_BROKERS", " kafka-1:9092,kafka-2:9092 "),
            ("KAFKA_TOPIC_PREFIX", "shop-staging"),
        ])
        .unwrap();
        assert_eq!(
            (config.brokers.as_str(), config.topic_prefix.as_str()),
            ("kafka-1:9092,kafka-2:9092", "shop-staging")
        );
    }

    #[test]
    fn should_treat_an_empty_value_as_unset() {
        let config = config(&[("KAFKA_BROKERS", " "), ("KAFKA_TOPIC_PREFIX", "")]).unwrap();
        assert_eq!(config, self::config(&[]).unwrap());
    }

    #[test]
    fn should_reject_a_prefix_kafka_does_not_accept_in_a_topic_name() {
        let error = config(&[("KAFKA_TOPIC_PREFIX", "my shop")]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "KAFKA_TOPIC_PREFIX=\"my shop\" is not valid: expected letters, digits, '.', '_' or '-'"
        );
    }

    #[test]
    fn should_accept_every_character_a_topic_name_may_hold() {
        assert!(is_topic_name("Shop_2.staging-eu"));
        assert!(!is_topic_name(""));
        assert!(!is_topic_name("shop/eu"));
    }

    #[test]
    fn should_name_topics_by_aggregate_and_groups_by_consumer() {
        let config = config(&[]).unwrap();
        assert_eq!(config.topic("order_item"), "shop.order_item");
        assert_eq!(
            config.group("notify_new_product"),
            "shop.notify_new_product"
        );
    }
}
