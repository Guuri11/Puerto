//! Domain events (ADR-0018): what a write records in the outbox, and the transport-neutral parts
//! every transport shares — the envelope, the handler a consumer implements, the delivery policy
//! and the ports the worker drives.

pub mod entrega;
pub mod manejador;
pub mod transporte;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

pub use manejador::{
    MOTIVO_ENTREGAS_MAXIMAS_SUPERADAS, MOTIVO_ERROR_PERMANENTE, MOTIVO_PAYLOAD_INVALIDO,
    ManejadorEvento, ResultadoManejo,
};

/// An event an aggregate records. Each entity's `{Entity}Event` implements it, generated from
/// puerto.toml: the payload is the entity's snapshot, never the domain model itself.
pub trait EventoDominio {
    /// Past tense, snake_case: `product_created`.
    fn tipo_evento(&self) -> &'static str;
    /// The entity it happened to, snake_case: `product`.
    fn tipo_agregado(&self) -> &'static str;
    fn id_agregado(&self) -> Uuid;
    /// The payload's shape. A consumer that reads an older one can tell.
    fn version_esquema(&self) -> u16;
    fn payload(&self) -> Value;
}

/// One event as it travels: the outbox row, a delivery, a Kafka message. Its `id` stays the same
/// across redeliveries, so a handler deduplicates on it.
///
/// `#[non_exhaustive]`: built with [`SobreEvento::new`] (a new event) or
/// [`SobreEvento::from_partes`] (one read back from a store), so a later field breaks no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SobreEvento {
    pub id: Uuid,
    pub tipo_evento: String,
    pub tipo_agregado: String,
    pub id_agregado: Uuid,
    pub version_esquema: u16,
    pub ocurrido_en: DateTime<Utc>,
    pub payload: Value,
    /// Context that is not the event's: `correlation_id`, later `tenant_id`.
    pub cabeceras: BTreeMap<String, String>,
}

/// Every field of an envelope, for an adapter that reads one back from where it was stored.
#[derive(Debug, Clone, PartialEq)]
pub struct PartesSobre {
    pub id: Uuid,
    pub tipo_evento: String,
    pub tipo_agregado: String,
    pub id_agregado: Uuid,
    pub version_esquema: u16,
    pub ocurrido_en: DateTime<Utc>,
    pub payload: Value,
    pub cabeceras: BTreeMap<String, String>,
}

impl SobreEvento {
    /// A new event: the id is a UUID v7, so ids sort by the time they were made.
    pub fn new(event: &impl EventoDominio) -> Self {
        Self {
            id: Uuid::now_v7(),
            tipo_evento: event.tipo_evento().to_string(),
            tipo_agregado: event.tipo_agregado().to_string(),
            id_agregado: event.id_agregado(),
            version_esquema: event.version_esquema(),
            ocurrido_en: Utc::now(),
            payload: event.payload(),
            cabeceras: BTreeMap::new(),
        }
    }

    pub fn from_partes(parts: PartesSobre) -> Self {
        Self {
            id: parts.id,
            tipo_evento: parts.tipo_evento,
            tipo_agregado: parts.tipo_agregado,
            id_agregado: parts.id_agregado,
            version_esquema: parts.version_esquema,
            ocurrido_en: parts.ocurrido_en,
            payload: parts.payload,
            cabeceras: parts.cabeceras,
        }
    }

    pub fn con_cabecera(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.cabeceras.insert(key.into(), value.into());
        self
    }

    /// The payload as the snapshot type the consumer subscribed to.
    pub fn decodificar<T: DeserializeOwned>(&self) -> Result<T, ErrorDecodificacion> {
        serde_json::from_value(self.payload.clone()).map_err(|err| ErrorDecodificacion {
            tipo_evento: self.tipo_evento.clone(),
            detalle: err.to_string(),
        })
    }
}

/// A payload that is not the shape its reader expects. A handler answers it with
/// `ResultadoManejo::Descartar` and [`MOTIVO_PAYLOAD_INVALIDO`]: retrying cannot fix it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("evento.payload_invalido")]
pub struct ErrorDecodificacion {
    pub tipo_evento: String,
    /// What the decoder said, for the dead letter.
    pub detalle: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ProductSnapshot {
        id: Uuid,
        nombre: String,
        price: i64,
    }

    struct ProductCreated(ProductSnapshot);

    impl EventoDominio for ProductCreated {
        fn tipo_evento(&self) -> &'static str {
            "product_created"
        }
        fn tipo_agregado(&self) -> &'static str {
            "product"
        }
        fn id_agregado(&self) -> Uuid {
            self.0.id
        }
        fn version_esquema(&self) -> u16 {
            3
        }
        fn payload(&self) -> Value {
            serde_json::to_value(&self.0).unwrap_or(Value::Null)
        }
    }

    fn product_created() -> ProductCreated {
        ProductCreated(ProductSnapshot {
            id: Uuid::now_v7(),
            nombre: "Desk lamp".into(),
            price: 2_500,
        })
    }

    #[test]
    fn should_carry_the_event_metadata_when_built_from_a_domain_event() {
        // Arrange
        let event = product_created();
        let before = Utc::now();

        // Act
        let sobre = SobreEvento::new(&event);

        // Assert
        assert_eq!(sobre.tipo_evento, "product_created");
        assert_eq!(sobre.tipo_agregado, "product");
        assert_eq!(sobre.id_agregado, event.0.id);
        assert_eq!(sobre.version_esquema, 3);
        assert_eq!(sobre.payload, event.payload());
        assert!(sobre.cabeceras.is_empty());
        assert!(sobre.ocurrido_en >= before && sobre.ocurrido_en <= Utc::now());
        assert_eq!(sobre.id.get_version_num(), 7);
    }

    #[test]
    fn should_give_every_envelope_its_own_id_when_built_from_the_same_event() {
        let event = product_created();
        assert_ne!(SobreEvento::new(&event).id, SobreEvento::new(&event).id);
    }

    #[test]
    fn should_keep_every_field_when_restored_from_its_parts() {
        let parts = PartesSobre {
            id: Uuid::now_v7(),
            tipo_evento: "product_updated".into(),
            tipo_agregado: "product".into(),
            id_agregado: Uuid::now_v7(),
            version_esquema: 2,
            ocurrido_en: Utc::now(),
            payload: json!({ "name": "Desk lamp" }),
            cabeceras: BTreeMap::from([("correlation_id".into(), "req-1".into())]),
        };

        let sobre = SobreEvento::from_partes(parts.clone());

        assert_eq!(sobre.id, parts.id);
        assert_eq!(sobre.tipo_evento, parts.tipo_evento);
        assert_eq!(sobre.tipo_agregado, parts.tipo_agregado);
        assert_eq!(sobre.id_agregado, parts.id_agregado);
        assert_eq!(sobre.version_esquema, parts.version_esquema);
        assert_eq!(sobre.ocurrido_en, parts.ocurrido_en);
        assert_eq!(sobre.payload, parts.payload);
        assert_eq!(sobre.cabeceras, parts.cabeceras);
    }

    #[test]
    fn should_add_a_header_when_one_is_given() {
        let sobre = SobreEvento::new(&product_created())
            .con_cabecera("correlation_id", "req-1")
            .con_cabecera("correlation_id", "req-2");

        assert_eq!(
            sobre.cabeceras,
            BTreeMap::from([("correlation_id".to_string(), "req-2".to_string())])
        );
    }

    #[test]
    fn should_round_trip_the_payload_when_decoded_as_its_snapshot() {
        let event = product_created();
        let sobre = SobreEvento::new(&event);

        let decoded = sobre.decodificar::<ProductSnapshot>();

        assert_eq!(decoded, Ok(event.0));
    }

    #[test]
    fn should_reject_the_payload_when_decoded_as_another_shape() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct OrderSnapshot {
            total: i64,
        }
        let sobre = SobreEvento::new(&product_created());

        let result = sobre.decodificar::<OrderSnapshot>();

        match result {
            Err(err) => {
                assert_eq!(err.to_string(), "evento.payload_invalido");
                assert_eq!(err.tipo_evento, "product_created");
                assert!(err.detalle.contains("total"), "detail: {}", err.detalle);
            }
            Ok(other) => panic!("decoded as another shape: {other:?}"),
        }
    }
}
