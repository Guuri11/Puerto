use async_trait::async_trait;
use puerto_core::events::transport::{DeadLetterRecord, DeadLetterSink, TransportError};
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
impl DeadLetterSink for PgDeadLetters {
    async fn record(&self, record: &DeadLetterRecord) -> Result<(), TransportError> {
        let attempts = i32::try_from(record.attempts).unwrap_or(i32::MAX);
        sqlx::query(
            "INSERT INTO dead_letters \
             (id, event_id, consumer, event_type, attempts, reason, detail, payload) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (event_id, consumer) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(record.event_id)
        .bind(&record.consumer)
        .bind(&record.event_type)
        .bind(attempts)
        .bind(&record.reason)
        .bind(&record.detail)
        .bind(&record.payload)
        .execute(&self.pool)
        .await
        .map_err(unavailable("dead_letters: record"))?;
        tracing::warn!(
            consumer = %record.consumer,
            event_id = ?record.event_id,
            reason = %record.reason,
            "dead_letters: recorded"
        );
        Ok(())
    }
}
