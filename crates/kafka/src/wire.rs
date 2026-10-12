//! An envelope as a Kafka record and back: the payload is the event's JSON payload, the rest of
//! the envelope travels in headers. Pure, so every rule is tested without a broker.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use puerto_core::events::{EnvelopeParts, EventEnvelope};
use uuid::Uuid;

pub(crate) const EVENT_ID: &str = "event_id";
pub(crate) const EVENT_TYPE: &str = "event_type";
pub(crate) const AGGREGATE_TYPE: &str = "aggregate_type";
pub(crate) const AGGREGATE_ID: &str = "aggregate_id";
pub(crate) const SCHEMA_VERSION: &str = "schema_version";
pub(crate) const OCCURRED_AT: &str = "occurred_at";

/// A record's headers as read: a header may have no value.
pub(crate) type RawHeaders = Vec<(String, Option<Vec<u8>>)>;

const ENVELOPE_HEADERS: [&str; 6] = [
    EVENT_ID,
    EVENT_TYPE,
    AGGREGATE_TYPE,
    AGGREGATE_ID,
    SCHEMA_VERSION,
    OCCURRED_AT,
];

/// The record's headers: the envelope's fields, then its own headers. One of those named like a
/// field is left out — the field wins.
pub(crate) fn headers_of(event: &EventEnvelope) -> Vec<(String, String)> {
    let fields = [
        (EVENT_ID, event.id.to_string()),
        (EVENT_TYPE, event.event_type.clone()),
        (AGGREGATE_TYPE, event.aggregate_type.clone()),
        (AGGREGATE_ID, event.aggregate_id.to_string()),
        (SCHEMA_VERSION, event.schema_version.to_string()),
        (OCCURRED_AT, event.occurred_at.to_rfc3339()),
    ];
    fields
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .chain(
            event
                .headers
                .iter()
                .filter(|(key, _)| !ENVELOPE_HEADERS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone())),
        )
        .collect()
}

/// A record read back as an envelope, or why it is not one (the dead letter's detail).
pub(crate) fn envelope_from(
    headers: &[(String, Option<Vec<u8>>)],
    payload: Option<&[u8]>,
) -> Result<EventEnvelope, String> {
    let text = |key: &str| -> Result<String, String> {
        let value = headers
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_deref())
            .ok_or_else(|| format!("header `{key}` is missing"))?;
        String::from_utf8(value.to_vec()).map_err(|_| format!("header `{key}` is not UTF-8"))
    };
    let parse_err = |key: &str| format!("header `{key}` is not valid");
    let id = Uuid::parse_str(&text(EVENT_ID)?).map_err(|_| parse_err(EVENT_ID))?;
    let aggregate_id =
        Uuid::parse_str(&text(AGGREGATE_ID)?).map_err(|_| parse_err(AGGREGATE_ID))?;
    let schema_version = text(SCHEMA_VERSION)?
        .parse::<u16>()
        .map_err(|_| parse_err(SCHEMA_VERSION))?;
    let occurred_at = DateTime::parse_from_rfc3339(&text(OCCURRED_AT)?)
        .map_err(|_| parse_err(OCCURRED_AT))?
        .with_timezone(&Utc);
    let payload = serde_json::from_slice(payload.ok_or("the payload is missing")?)
        .map_err(|err| format!("the payload is not JSON: {err}"))?;
    let mut extra = BTreeMap::new();
    for (key, value) in headers {
        if ENVELOPE_HEADERS.contains(&key.as_str()) {
            continue;
        }
        let value = value.as_deref().unwrap_or_default();
        let value = String::from_utf8(value.to_vec())
            .map_err(|_| format!("header `{key}` is not UTF-8"))?;
        extra.insert(key.clone(), value);
    }
    Ok(EventEnvelope::from_parts(EnvelopeParts {
        id,
        event_type: text(EVENT_TYPE)?,
        aggregate_type: text(AGGREGATE_TYPE)?,
        aggregate_id,
        schema_version,
        occurred_at,
        payload,
        headers: extra,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use puerto_core::DomainEvent;
    use serde_json::{Value, json};

    struct Created(Uuid);

    impl DomainEvent for Created {
        fn event_type(&self) -> &'static str {
            "product_created"
        }
        fn aggregate_type(&self) -> &'static str {
            "product"
        }
        fn aggregate_id(&self) -> Uuid {
            self.0
        }
        fn schema_version(&self) -> u16 {
            3
        }
        fn payload(&self) -> Value {
            json!({ "id": self.0, "name": "Desk lamp" })
        }
    }

    fn envelope() -> EventEnvelope {
        EventEnvelope::new(&Created(Uuid::now_v7())).with_header("trace_id", "abc")
    }

    fn as_record(event: &EventEnvelope) -> (RawHeaders, Vec<u8>) {
        let headers = headers_of(event)
            .into_iter()
            .map(|(k, v)| (k, Some(v.into_bytes())))
            .collect();
        (headers, event.payload.to_string().into_bytes())
    }

    fn without(headers: RawHeaders, key: &str) -> RawHeaders {
        headers.into_iter().filter(|(k, _)| k != key).collect()
    }

    #[test]
    fn should_read_back_the_envelope_it_wrote() {
        let event = envelope();
        let (headers, payload) = as_record(&event);
        assert_eq!(envelope_from(&headers, Some(&payload)), Ok(event));
    }

    #[test]
    fn should_keep_the_envelope_field_when_a_header_is_named_like_one() {
        let event = envelope().with_header(EVENT_TYPE, "spoofed");
        let headers = headers_of(&event);
        let event_types: Vec<&str> = headers
            .iter()
            .filter(|(k, _)| k == EVENT_TYPE)
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(event_types, vec!["product_created"]);
    }

    #[test]
    fn should_reject_a_record_missing_an_envelope_header() {
        let (headers, payload) = as_record(&envelope());
        for key in ENVELOPE_HEADERS {
            assert_eq!(
                envelope_from(&without(headers.clone(), key), Some(&payload)),
                Err(format!("header `{key}` is missing"))
            );
        }
    }

    #[test]
    fn should_reject_a_record_whose_envelope_header_does_not_parse() {
        let (headers, payload) = as_record(&envelope());
        for key in [EVENT_ID, AGGREGATE_ID, SCHEMA_VERSION, OCCURRED_AT] {
            let mut broken = without(headers.clone(), key);
            broken.push((key.to_string(), Some(b"70000x".to_vec())));
            assert_eq!(
                envelope_from(&broken, Some(&payload)),
                Err(format!("header `{key}` is not valid"))
            );
        }
    }

    #[test]
    fn should_reject_a_record_whose_header_is_not_utf8() {
        let (headers, payload) = as_record(&envelope());
        for key in [EVENT_TYPE, "trace_id"] {
            let mut broken = without(headers.clone(), key);
            broken.push((key.to_string(), Some(vec![0xff, 0xfe])));
            assert_eq!(
                envelope_from(&broken, Some(&payload)),
                Err(format!("header `{key}` is not UTF-8"))
            );
        }
    }

    #[test]
    fn should_reject_a_record_without_a_json_payload() {
        let (headers, _) = as_record(&envelope());
        assert_eq!(
            envelope_from(&headers, None),
            Err("the payload is missing".into())
        );
        assert!(
            envelope_from(&headers, Some(b"not json"))
                .is_err_and(|detail| detail.starts_with("the payload is not JSON"))
        );
    }

    #[test]
    fn should_read_a_header_without_a_value_as_empty() {
        let (mut headers, payload) = as_record(&envelope());
        headers.push(("tenant".into(), None));
        let event = envelope_from(&headers, Some(&payload)).unwrap();
        assert_eq!(event.headers.get("tenant").map(String::as_str), Some(""));
    }
}
