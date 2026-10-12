//! Domain events (ADR-0018): what a write records in the outbox, and the transport-neutral parts
//! every transport shares — the envelope, the handler a consumer implements, the delivery policy
//! and the ports the worker drives.

pub mod delivery;
pub mod handler;
pub mod transport;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

pub use handler::{
    EventHandler, HandlerOutcome, REASON_MAX_DELIVERIES_EXCEEDED, REASON_PAYLOAD_INVALID,
    REASON_PERMANENT_ERROR,
};

/// An event an aggregate records. Each entity's `{Entity}Event` implements it, generated from
/// puerto.toml: the payload is the entity's snapshot, never the domain model itself.
pub trait DomainEvent {
    /// Past tense, snake_case: `product_created`.
    fn event_type(&self) -> &'static str;
    /// The entity it happened to, snake_case: `product`.
    fn aggregate_type(&self) -> &'static str;
    fn aggregate_id(&self) -> Uuid;
    /// The payload's shape. A consumer that reads an older one can tell.
    fn schema_version(&self) -> u16;
    fn payload(&self) -> Value;
}

/// One event as it travels: the outbox row, a delivery, a Kafka message. Its `id` stays the same
/// across redeliveries, so a handler deduplicates on it.
///
/// `#[non_exhaustive]`: built with [`EventEnvelope::new`] (a new event) or
/// [`EventEnvelope::from_parts`] (one read back from a store), so a later field breaks no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub schema_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
    /// Context that is not the event's: `correlation_id`, later `tenant_id`.
    pub headers: BTreeMap<String, String>,
}

/// Every field of an envelope, for an adapter that reads one back from where it was stored.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeParts {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub schema_version: u16,
    pub occurred_at: DateTime<Utc>,
    pub payload: Value,
    pub headers: BTreeMap<String, String>,
}

impl EventEnvelope {
    /// A new event: the id is a UUID v7, so ids sort by the time they were made.
    pub fn new(event: &impl DomainEvent) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_type: event.event_type().to_string(),
            aggregate_type: event.aggregate_type().to_string(),
            aggregate_id: event.aggregate_id(),
            schema_version: event.schema_version(),
            occurred_at: Utc::now(),
            payload: event.payload(),
            headers: BTreeMap::new(),
        }
    }

    pub fn from_parts(parts: EnvelopeParts) -> Self {
        Self {
            id: parts.id,
            event_type: parts.event_type,
            aggregate_type: parts.aggregate_type,
            aggregate_id: parts.aggregate_id,
            schema_version: parts.schema_version,
            occurred_at: parts.occurred_at,
            payload: parts.payload,
            headers: parts.headers,
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// The payload as the snapshot type the consumer subscribed to.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, DecodeError> {
        serde_json::from_value(self.payload.clone()).map_err(|err| DecodeError {
            event_type: self.event_type.clone(),
            detail: err.to_string(),
        })
    }
}

/// A payload that is not the shape its reader expects. A handler answers it with
/// `HandlerOutcome::DeadLetter` and [`REASON_PAYLOAD_INVALID`]: retrying cannot fix it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("event.payload_invalid")]
pub struct DecodeError {
    pub event_type: String,
    /// What the decoder said, for the dead letter.
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ProductSnapshot {
        id: Uuid,
        name: String,
        price: i64,
    }

    struct ProductCreated(ProductSnapshot);

    impl DomainEvent for ProductCreated {
        fn event_type(&self) -> &'static str {
            "product_created"
        }
        fn aggregate_type(&self) -> &'static str {
            "product"
        }
        fn aggregate_id(&self) -> Uuid {
            self.0.id
        }
        fn schema_version(&self) -> u16 {
            3
        }
        fn payload(&self) -> Value {
            serde_json::to_value(&self.0).unwrap_or(Value::Null)
        }
    }

    fn product_created() -> ProductCreated {
        ProductCreated(ProductSnapshot {
            id: Uuid::now_v7(),
            name: "Desk lamp".into(),
            price: 2_500,
        })
    }

    #[test]
    fn should_carry_the_event_metadata_when_built_from_a_domain_event() {
        // Arrange
        let event = product_created();
        let before = Utc::now();

        // Act
        let envelope = EventEnvelope::new(&event);

        // Assert
        assert_eq!(envelope.event_type, "product_created");
        assert_eq!(envelope.aggregate_type, "product");
        assert_eq!(envelope.aggregate_id, event.0.id);
        assert_eq!(envelope.schema_version, 3);
        assert_eq!(envelope.payload, event.payload());
        assert!(envelope.headers.is_empty());
        assert!(envelope.occurred_at >= before && envelope.occurred_at <= Utc::now());
        assert_eq!(envelope.id.get_version_num(), 7);
    }

    #[test]
    fn should_give_every_envelope_its_own_id_when_built_from_the_same_event() {
        let event = product_created();
        assert_ne!(EventEnvelope::new(&event).id, EventEnvelope::new(&event).id);
    }

    #[test]
    fn should_keep_every_field_when_restored_from_its_parts() {
        let parts = EnvelopeParts {
            id: Uuid::now_v7(),
            event_type: "product_updated".into(),
            aggregate_type: "product".into(),
            aggregate_id: Uuid::now_v7(),
            schema_version: 2,
            occurred_at: Utc::now(),
            payload: json!({ "name": "Desk lamp" }),
            headers: BTreeMap::from([("correlation_id".into(), "req-1".into())]),
        };

        let envelope = EventEnvelope::from_parts(parts.clone());

        assert_eq!(envelope.id, parts.id);
        assert_eq!(envelope.event_type, parts.event_type);
        assert_eq!(envelope.aggregate_type, parts.aggregate_type);
        assert_eq!(envelope.aggregate_id, parts.aggregate_id);
        assert_eq!(envelope.schema_version, parts.schema_version);
        assert_eq!(envelope.occurred_at, parts.occurred_at);
        assert_eq!(envelope.payload, parts.payload);
        assert_eq!(envelope.headers, parts.headers);
    }

    #[test]
    fn should_add_a_header_when_one_is_given() {
        let envelope = EventEnvelope::new(&product_created())
            .with_header("correlation_id", "req-1")
            .with_header("correlation_id", "req-2");

        assert_eq!(
            envelope.headers,
            BTreeMap::from([("correlation_id".to_string(), "req-2".to_string())])
        );
    }

    #[test]
    fn should_round_trip_the_payload_when_decoded_as_its_snapshot() {
        let event = product_created();
        let envelope = EventEnvelope::new(&event);

        let decoded = envelope.decode::<ProductSnapshot>();

        assert_eq!(decoded, Ok(event.0));
    }

    #[test]
    fn should_reject_the_payload_when_decoded_as_another_shape() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct OrderSnapshot {
            total: i64,
        }
        let envelope = EventEnvelope::new(&product_created());

        let result = envelope.decode::<OrderSnapshot>();

        match result {
            Err(err) => {
                assert_eq!(err.to_string(), "event.payload_invalid");
                assert_eq!(err.event_type, "product_created");
                assert!(err.detail.contains("total"), "detail: {}", err.detail);
            }
            Ok(other) => panic!("decoded as another shape: {other:?}"),
        }
    }
}
