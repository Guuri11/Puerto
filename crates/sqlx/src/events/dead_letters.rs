use async_trait::async_trait;
use puerto_nucleo::eventos::transporte::{DestinoDescartados, ErrorTransporte, RegistroDescartado};
use sqlx::PgPool;
use uuid::Uuid;

use super::unavailable;

/// `dead_letters`, whatever the transport. Recording the same event for the same consumer twice
/// keeps the first record (`dead_letters_event_consumer`).
pub struct PgDeadLetters {
    pool: PgPool,
}

impl PgDeadLetters {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DestinoDescartados for PgDeadLetters {
    async fn registrar(&self, record: &RegistroDescartado) -> Result<(), ErrorTransporte> {
        let attempts = i32::try_from(record.intentos).unwrap_or(i32::MAX);
        sqlx::query(
            "INSERT INTO dead_letters \
             (id, event_id, consumer, event_type, attempts, reason, detail, payload) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (event_id, consumer) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(record.id_evento)
        .bind(&record.consumidor)
        .bind(&record.tipo_evento)
        .bind(attempts)
        .bind(&record.motivo)
        .bind(&record.detalle)
        .bind(&record.payload)
        .execute(&self.pool)
        .await
        .map_err(unavailable("dead_letters: record"))?;
        tracing::warn!(
            consumer = %record.consumidor,
            event_id = ?record.id_evento,
            reason = %record.motivo,
            "dead_letters: recorded"
        );
        Ok(())
    }
}
