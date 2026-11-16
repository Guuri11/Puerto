//! The Postgres side of domain events (ADR-0018 § 4): the outbox the relay reads, the `db`
//! transport (`entregas_eventos`) and the dead-letter sink every transport writes to. The tables
//! are `migrations/outbox.sql`, which `puerto new --eventos` writes into the project.

mod dead_letters;
mod deliveries;
mod outbox;

pub use dead_letters::DescartadosPostgres;
pub use deliveries::{EntregasPostgres, RESERVA_POR_DEFECTO, SuscripcionPostgres};
pub use outbox::{CLAVE_BLOQUEO_RELAY, OutboxPostgres};

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use puerto_nucleo::eventos::transporte::ErrorTransporte;
use puerto_nucleo::eventos::{PartesSobre, SobreEvento};
use serde_json::Value;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

/// The outbox columns every query that hands out an envelope selects, prefixed by `o.`.
const ENVELOPE_COLUMNS: &str = "o.id, o.secuencia, o.tipo_agregado, o.id_agregado, o.tipo_evento, \
                                o.version_esquema, o.payload, o.cabeceras, o.ocurrido_en";

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
            seq: row.try_get("secuencia")?,
            aggregate_type: row.try_get("tipo_agregado")?,
            aggregate_id: row.try_get("id_agregado")?,
            event_type: row.try_get("tipo_evento")?,
            schema_version: row.try_get("version_esquema")?,
            payload: row.try_get("payload")?,
            headers: row.try_get("cabeceras")?,
            occurred_at: row.try_get("ocurrido_en")?,
        })
    }
}

impl TryFrom<OutboxRow> for SobreEvento {
    type Error = ErrorTransporte;

    /// A schema version outside `u16`, or headers that are not a string map, are rows nobody
    /// wrote through a unit of work.
    fn try_from(row: OutboxRow) -> Result<Self, Self::Error> {
        let schema_version =
            u16::try_from(row.schema_version).map_err(|_| ErrorTransporte::DatosInvalidos)?;
        let headers: BTreeMap<String, String> =
            serde_json::from_value(row.headers).map_err(|_| ErrorTransporte::DatosInvalidos)?;
        Ok(SobreEvento::from_partes(PartesSobre {
            id: row.id,
            tipo_evento: row.event_type,
            tipo_agregado: row.aggregate_type,
            id_agregado: row.aggregate_id,
            version_esquema: schema_version,
            ocurrido_en: row.occurred_at,
            payload: row.payload,
            cabeceras: headers,
        }))
    }
}

/// Reads one row as an envelope, logging why it could not.
fn envelope_from_pg(row: &PgRow, context: &str) -> Result<SobreEvento, ErrorTransporte> {
    let row = OutboxRow::from_pg(row).map_err(|err| {
        tracing::error!(error = %err, "{context}: fila del outbox ilegible");
        ErrorTransporte::DatosInvalidos
    })?;
    let id = row.id;
    SobreEvento::try_from(row).inspect_err(|_| {
        tracing::error!(%id, "{context}: la fila del outbox no es un sobre");
    })
}

fn unavailable(context: &str) -> impl FnOnce(sqlx::Error) -> ErrorTransporte + '_ {
    move |err| {
        tracing::error!(error = %err, "{context}: falló la base de datos");
        ErrorTransporte::NoDisponible
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

        let envelope = SobreEvento::try_from(row.clone());

        let Ok(envelope) = envelope else {
            panic!("rejected a valid row: {envelope:?}");
        };
        assert_eq!(envelope.id, row.id);
        assert_eq!(envelope.tipo_agregado, "product");
        assert_eq!(envelope.id_agregado, row.aggregate_id);
        assert_eq!(envelope.tipo_evento, "product_created");
        assert_eq!(envelope.version_esquema, 1);
        assert_eq!(envelope.payload, row.payload);
        assert_eq!(envelope.ocurrido_en, row.occurred_at);
        assert_eq!(
            envelope.cabeceras,
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
                SobreEvento::try_from(row),
                Err(ErrorTransporte::DatosInvalidos)
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
            SobreEvento::try_from(row).map(|e| e.version_esquema),
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
            SobreEvento::try_from(row),
            Err(ErrorTransporte::DatosInvalidos)
        );
    }
}
