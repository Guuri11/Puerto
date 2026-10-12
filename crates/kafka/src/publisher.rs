//! The relay's output on the Kafka transport.

use std::time::Duration;

use async_trait::async_trait;
use puerto_core::events::EventEnvelope;
use puerto_core::events::transport::{EventPublisher, TransportError};
use rdkafka::error::KafkaError;
use rdkafka::message::{Header, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};

use crate::KafkaConfig;
use crate::wire;

/// How long a publish waits for the broker to confirm the record before it counts as failed;
/// the relay then tries the same event again on its next pass.
const DELIVERY_TIMEOUT_MS: &str = "30000";
/// How long a publish waits for room in the producer's queue.
const QUEUE_TIMEOUT: Duration = Duration::from_secs(5);

/// Publishes each event to its aggregate's topic, keyed by the aggregate id, and answers `Ok`
/// only once every in-sync replica has it (`acks=all`). The producer is idempotent: a retry of
/// librdkafka's own never writes a record twice nor out of order.
pub struct KafkaPublisher {
    config: KafkaConfig,
    producer: FutureProducer,
}

impl KafkaPublisher {
    pub fn new(config: &KafkaConfig) -> Result<Self, KafkaError> {
        let producer = config
            .client()
            .set("acks", "all")
            .set("enable.idempotence", "true")
            .set("message.timeout.ms", DELIVERY_TIMEOUT_MS)
            .create()?;
        Ok(Self {
            config: config.clone(),
            producer,
        })
    }
}

#[async_trait]
impl EventPublisher for KafkaPublisher {
    async fn publish(&self, event: &EventEnvelope) -> Result<(), TransportError> {
        let topic = self.config.topic(&event.aggregate_type);
        let key = event.aggregate_id.to_string();
        let payload = event.payload.to_string();
        let headers =
            wire::headers_of(event)
                .iter()
                .fold(OwnedHeaders::new(), |headers, (key, value)| {
                    headers.insert(Header {
                        key,
                        value: Some(value.as_str()),
                    })
                });
        let record = FutureRecord::to(&topic)
            .key(&key)
            .payload(&payload)
            .headers(headers);
        match self.producer.send(record, QUEUE_TIMEOUT).await {
            Ok(_) => Ok(()),
            Err((err, _)) => {
                tracing::error!(topic, event_id = %event.id, error = %err, "kafka: publish failed");
                Err(TransportError::Unavailable)
            }
        }
    }
}
