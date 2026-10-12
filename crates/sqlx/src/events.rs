//! The Postgres side of domain events (ADR-0018 § 4): the outbox the relay reads, the `db`
//! transport (`event_deliveries`) and the dead-letter sink every transport writes to. The tables
//! are `migrations/outbox.sql`, which `puerto new --events` writes into the project.

mod dead_letters;
mod deliveries;
mod outbox;

pub use dead_letters::PgDeadLetters;
pub use deliveries::{DEFAULT_CLAIM_LEASE, PgDeliveries, PgSubscription};
pub use outbox::{PgOutbox, RELAY_LOCK_KEY};

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use puerto_core::events::transport::TransportError;
use puerto_core::events::{EnvelopeParts, EventEnvelope};
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

/// The outbox columns every query that hands out an envelope selects, prefixed by `o.`.
const ENVELOPE_COLUMNS: &str = "o.id, o.seq, o.aggregate_type, o.aggregate_id, o.event_type, \
                                o.schema_version, o.payload, o.headers, o.occurred_at";

/// An outbox row as Postgres stores it, before it is checked to be an envelope.
#[derive(Debug, Clone, PartialEq)]
struct OutboxRow {
    id: Uuid,
    seq: i64,
    aggregate_type: String,
    aggregate_id: Uuid,
    event_type: String,
    schema_version: i32,
    payload: Value,
    headers: Value,
    occurred_at: DateTime<Utc>,
}

impl OutboxRow {
    fn from_pg(row: &PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            aggregate_type: row.try_get("aggregate_type")?,
            aggregate_id: row.try_get("aggregate_id")?,
            event_type: row.try_get("event_type")?,
            schema_version: row.try_get("schema_version")?,
            payload: row.try_get("payload")?,
            headers: row.try_get("headers")?,
            occurred_at: row.try_get("occurred_at")?,
        })
    }
}

impl TryFrom<OutboxRow> for EventEnvelope {
    type Error = TransportError;

    /// A schema version outside `u16`, or headers that are not a string map, are rows nobody
    /// wrote through a unit of work.
    fn try_from(row: OutboxRow) -> Result<Self, Self::Error> {
        let schema_version =
            u16::try_from(row.schema_version).map_err(|_| TransportError::InvalidData)?;
        let headers: BTreeMap<String, String> =
            serde_json::from_value(row.headers).map_err(|_| TransportError::InvalidData)?;
        Ok(EventEnvelope::from_parts(EnvelopeParts {
            id: row.id,
            event_type: row.event_type,
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            schema_version,
            occurred_at: row.occurred_at,
            payload: row.payload,
            headers,
        }))
    }
}

/// Reads one row as an envelope, logging why it could not.
fn envelope_from_pg(row: &PgRow, context: &str) -> Result<EventEnvelope, TransportError> {
    let row = OutboxRow::from_pg(row).map_err(|err| {
        tracing::error!(error = %err, "{context}: unreadable outbox row");
        TransportError::InvalidData
    })?;
    let id = row.id;
    EventEnvelope::try_from(row).inspect_err(|_| {
        tracing::error!(%id, "{context}: outbox row is not an envelope");
    })
}

fn unavailable(context: &str) -> impl FnOnce(sqlx::Error) -> TransportError + '_ {
    move |err| {
        tracing::error!(error = %err, "{context}: database failed");
        TransportError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row() -> OutboxRow {
        OutboxRow {
            id: Uuid::now_v7(),
            seq: 7,
            aggregate_type: "product".into(),
            aggregate_id: Uuid::now_v7(),
            event_type: "product_created".into(),
            schema_version: 1,
            payload: json!({ "name": "Desk lamp" }),
            headers: json!({ "correlation_id": "req-1" }),
            occurred_at: Utc::now(),
        }
    }

    #[test]
    fn should_read_every_column_when_the_row_is_an_envelope() {
        let row = row();

        let envelope = EventEnvelope::try_from(row.clone());

        let Ok(envelope) = envelope else {
            panic!("rejected a valid row: {envelope:?}");
        };
        assert_eq!(envelope.id, row.id);
        assert_eq!(envelope.aggregate_type, "product");
        assert_eq!(envelope.aggregate_id, row.aggregate_id);
        assert_eq!(envelope.event_type, "product_created");
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.payload, row.payload);
        assert_eq!(envelope.occurred_at, row.occurred_at);
        assert_eq!(
            envelope.headers,
            BTreeMap::from([("correlation_id".to_string(), "req-1".to_string())])
        );
    }

    #[test]
    fn should_reject_the_row_when_its_schema_version_is_out_of_range() {
        for version in [-1, i32::from(u16::MAX) + 1] {
            let row = OutboxRow {
                schema_version: version,
                ..row()
            };
            assert_eq!(
                EventEnvelope::try_from(row),
                Err(TransportError::InvalidData)
            );
        }
    }

    #[test]
    fn should_accept_the_largest_schema_version_when_it_fits() {
        let row = OutboxRow {
            schema_version: i32::from(u16::MAX),
            ..row()
        };
        assert_eq!(
            EventEnvelope::try_from(row).map(|e| e.schema_version),
            Ok(u16::MAX)
        );
    }

    #[test]
    fn should_reject_the_row_when_its_headers_are_not_a_string_map() {
        let row = OutboxRow {
            headers: json!({ "retries": 3 }),
            ..row()
        };
        assert_eq!(
            EventEnvelope::try_from(row),
            Err(TransportError::InvalidData)
        );
    }
}
