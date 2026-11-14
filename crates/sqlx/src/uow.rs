//! The Postgres side of a generated unit of work (ADR-0018 § 2): one transaction that saves an
//! aggregate and records its events in the outbox. A generated `Pg{Entity}UnitOfWork` wraps a
//! [`TransaccionPostgres`] and only writes its entity's upsert, through [`TransaccionPostgres::tx`].

use puerto_nucleo::{ErrorRepositorio, EventoDominio, SobreEvento};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use crate::mapear_error_sqlx;

/// An open transaction. Dropping it without [`TransaccionPostgres::commit`] rolls it back — an early
/// `?` in a use case is the rollback.
pub struct TransaccionPostgres {
    tx: Transaction<'static, Postgres>,
}

impl TransaccionPostgres {
    pub async fn begin(pool: &PgPool) -> Result<Self, ErrorRepositorio> {
        let tx = pool.begin().await.map_err(|err| {
            tracing::error!(error = %err, "unit_of_work: falló el begin");
            mapear_error_sqlx(&err)
        })?;
        Ok(Self { tx })
    }

    /// The connection the entity's own writes run on: `query(…).execute(uow.tx())`.
    pub fn tx(&mut self) -> &mut PgConnection {
        &mut self.tx
    }

    /// Records a new event in the outbox, inside this transaction.
    pub async fn registrar(&mut self, event: &impl EventoDominio) -> Result<(), ErrorRepositorio> {
        self.registrar_sobre(&SobreEvento::new(event)).await
    }

    /// Records an envelope built by the caller — one carrying headers (`correlation_id`).
    pub async fn registrar_sobre(&mut self, event: &SobreEvento) -> Result<(), ErrorRepositorio> {
        let headers = serde_json::to_value(&event.cabeceras).map_err(|err| {
            tracing::error!(error = %err, event_type = %event.tipo_evento, "unit_of_work: las cabeceras no se pueden serializar");
            ErrorRepositorio::DatosInvalidos
        })?;
        sqlx::query(
            "INSERT INTO outbox \
             (id, tipo_agregado, id_agregado, tipo_evento, version_esquema, payload, cabeceras, ocurrido_en) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(event.id)
        .bind(&event.tipo_agregado)
        .bind(event.id_agregado)
        .bind(&event.tipo_evento)
        .bind(i32::from(event.version_esquema))
        .bind(&event.payload)
        .bind(headers)
        .bind(event.ocurrido_en)
        .execute(&mut *self.tx)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, event_type = %event.tipo_evento, "unit_of_work: falló la inserción en el outbox");
            mapear_error_sqlx(&err)
        })?;
        Ok(())
    }

    pub async fn commit(self) -> Result<(), ErrorRepositorio> {
        self.tx.commit().await.map_err(|err| {
            tracing::error!(error = %err, "unit_of_work: falló el commit");
            mapear_error_sqlx(&err)
        })
    }
}
