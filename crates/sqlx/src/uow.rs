//! The Postgres side of a generated unit of work (ADR-0018 § 2): one transaction that saves an
//! aggregate and records its events in the outbox. A generated `Pg{Entity}UnitOfWork` wraps a
//! [`PgTransaction`] and only writes its entity's upsert, through [`PgTransaction::tx`].

use puerto_core::{DomainEvent, EventEnvelope, RepositoryError};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

use crate::map_sqlx_error;

/// An open transaction. Dropping it without [`PgTransaction::commit`] rolls it back — an early
/// `?` in a use case is the rollback.
pub struct PgTransaction {
    tx: Transaction<'static, Postgres>,
}

impl PgTransaction {
    pub async fn begin(pool: &PgPool) -> Result<Self, RepositoryError> {
        let tx = pool.begin().await.map_err(|err| {
            tracing::error!(error = %err, "unit_of_work: begin failed");
            map_sqlx_error(&err)
        })?;
        Ok(Self { tx })
    }

    /// The connection the entity's own writes run on: `query(…).execute(uow.tx())`.
    pub fn tx(&mut self) -> &mut PgConnection {
        &mut self.tx
    }

    /// Records a new event in the outbox, inside this transaction.
    pub async fn record(&mut self, event: &impl DomainEvent) -> Result<(), RepositoryError> {
        self.record_envelope(&EventEnvelope::new(event)).await
    }

    /// Records an envelope built by the caller — one carrying headers (`correlation_id`).
    pub async fn record_envelope(&mut self, event: &EventEnvelope) -> Result<(), RepositoryError> {
        let headers = serde_json::to_value(&event.headers).map_err(|err| {
            tracing::error!(error = %err, event_type = %event.event_type, "unit_of_work: headers not serialisable");
            RepositoryError::InvalidData
        })?;
        sqlx::query(
            "INSERT INTO outbox \
             (id, aggregate_type, aggregate_id, event_type, schema_version, payload, headers, occurred_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(event.id)
        .bind(&event.aggregate_type)
        .bind(event.aggregate_id)
        .bind(&event.event_type)
        .bind(i32::from(event.schema_version))
        .bind(&event.payload)
        .bind(headers)
        .bind(event.occurred_at)
        .execute(&mut *self.tx)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, event_type = %event.event_type, "unit_of_work: outbox insert failed");
            map_sqlx_error(&err)
        })?;
        Ok(())
    }

    pub async fn commit(self) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(|err| {
            tracing::error!(error = %err, "unit_of_work: commit failed");
            map_sqlx_error(&err)
        })
    }
}
