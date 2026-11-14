//! An envelope as a Kafka record and back: the payload is the event's JSON payload, the rest of
//! the envelope travels in headers. Pure, so every rule is tested without a broker.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use puerto_nucleo::eventos::{PartesSobre, SobreEvento};
use uuid::Uuid;

pub(crate) const EVENT_ID: &str = "id_evento";
pub(crate) const EVENT_TYPE: &str = "tipo_evento";
pub(crate) const AGGREGATE_TYPE: &str = "tipo_agregado";
pub(crate) const AGGREGATE_ID: &str = "id_agregado";
pub(crate) const SCHEMA_VERSION: &str = "version_esquema";
pub(crate) const OCCURRED_AT: &str = "ocurrido_en";

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
pub(crate) fn headers_of(event: &SobreEvento) -> Vec<(String, String)> {
    let fields = [
        (EVENT_ID, event.id.to_string()),
        (EVENT_TYPE, event.tipo_evento.clone()),
        (AGGREGATE_TYPE, event.tipo_agregado.clone()),
        (AGGREGATE_ID, event.id_agregado.to_string()),
        (SCHEMA_VERSION, event.version_esquema.to_string()),
        (OCCURRED_AT, event.ocurrido_en.to_rfc3339()),
    ];
    fields
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .chain(
            event
                .cabeceras
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
) -> Result<SobreEvento, String> {
    let text = |key: &str| -> Result<String, String> {
        let value = headers
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_deref())
            .ok_or_else(|| format!("falta la cabecera `{key}`"))?;
        String::from_utf8(value.to_vec()).map_err(|_| format!("la cabecera `{key}` no es UTF-8"))
    };
    let parse_err = |key: &str| format!("la cabecera `{key}` no es válida");
    let id = Uuid::parse_str(&text(EVENT_ID)?).map_err(|_| parse_err(EVENT_ID))?;
    let aggregate_id =
        Uuid::parse_str(&text(AGGREGATE_ID)?).map_err(|_| parse_err(AGGREGATE_ID))?;
    let schema_version = text(SCHEMA_VERSION)?
        .parse::<u16>()
        .map_err(|_| parse_err(SCHEMA_VERSION))?;
    let occurred_at = DateTime::parse_from_rfc3339(&text(OCCURRED_AT)?)
        .map_err(|_| parse_err(OCCURRED_AT))?
        .with_timezone(&Utc);
    let payload = serde_json::from_slice(payload.ok_or("falta el payload")?)
        .map_err(|err| format!("el payload no es JSON: {err}"))?;
    let mut extra = BTreeMap::new();
    for (key, value) in headers {
        if ENVELOPE_HEADERS.contains(&key.as_str()) {
            continue;
        }
        let value = value.as_deref().unwrap_or_default();
        let value = String::from_utf8(value.to_vec())
            .map_err(|_| format!("la cabecera `{key}` no es UTF-8"))?;
        extra.insert(key.clone(), value);
    }
    Ok(SobreEvento::from_partes(PartesSobre {
        id,
        tipo_evento: text(EVENT_TYPE)?,
        tipo_agregado: text(AGGREGATE_TYPE)?,
        id_agregado: aggregate_id,
        version_esquema: schema_version,
        ocurrido_en: occurred_at,
        payload,
        cabeceras: extra,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use puerto_nucleo::EventoDominio;
    use serde_json::{Value, json};

    struct Created(Uuid);

    impl EventoDominio for Created {
        fn tipo_evento(&self) -> &'static str {
            "product_created"
        }
        fn tipo_agregado(&self) -> &'static str {
            "product"
        }
        fn id_agregado(&self) -> Uuid {
            self.0
        }
        fn version_esquema(&self) -> u16 {
            3
        }
        fn payload(&self) -> Value {
            json!({ "id": self.0, "name": "Desk lamp" })
        }
    }

    fn envelope() -> SobreEvento {
        SobreEvento::new(&Created(Uuid::now_v7())).con_cabecera("trace_id", "abc")
    }

    fn as_record(event: &SobreEvento) -> (RawHeaders, Vec<u8>) {
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
        let event = envelope().con_cabecera(EVENT_TYPE, "spoofed");
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
                Err(format!("falta la cabecera `{key}`"))
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
                Err(format!("la cabecera `{key}` no es válida"))
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
                Err(format!("la cabecera `{key}` no es UTF-8"))
            );
        }
    }

    #[test]
    fn should_reject_a_record_without_a_json_payload() {
        let (headers, _) = as_record(&envelope());
        assert_eq!(
            envelope_from(&headers, None),
            Err("falta el payload".into())
        );
        assert!(
            envelope_from(&headers, Some(b"not json"))
                .is_err_and(|detail| detail.starts_with("el payload no es JSON"))
        );
    }

    #[test]
    fn should_read_a_header_without_a_value_as_empty() {
        let (mut headers, payload) = as_record(&envelope());
        headers.push(("tenant".into(), None));
        let event = envelope_from(&headers, Some(&payload)).unwrap();
        assert_eq!(event.cabeceras.get("tenant").map(String::as_str), Some(""));
    }
}
