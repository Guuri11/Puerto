use async_trait::async_trait;
use puerto_core::EventEnvelope;
use puerto_core::events::transport::{Outbox, TransportError};
use sqlx::{PgConnection, PgPool};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{ENVELOPE_COLUMNS, envelope_from_pg, unavailable};

/// The advisory lock the relay holds: only one worker replica relays, so the outbox order is
/// kept. The bytes of `"puerto"`.
pub const RELAY_LOCK_KEY: i64 = 0x7075_6572_746f;

/// The outbox, as the relay reads it.
pub struct PgOutbox {
    pool: PgPool,
    lock_key: i64,
    /// The connection holding the advisory lock, once this relay leads. A session lock lives as
    /// long as its connection, so it is kept out of the pool.
    leader: Mutex<Option<PgConnection>>,
}

impl PgOutbox {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            lock_key: RELAY_LOCK_KEY,
            leader: Mutex::new(None),
        }
    }

    /// Another lock key: two projects relaying from one database each need their own.
    pub fn with_lock_key(mut self, key: i64) -> Self {
        self.lock_key = key;
        self
    }
}

#[async_trait]
impl Outbox for PgOutbox {
    async fn try_lead(&self) -> Result<bool, TransportError> {
        let mut leader = self.leader.lock().await;
        if let Some(conn) = leader.as_mut() {
            if sqlx::query("SELECT 1").execute(&mut *conn).await.is_ok() {
                return Ok(true);
            }
            // The connection died, and the lock with it: compete for it again.
            tracing::warn!("outbox: lost the relay lock's connection");
            *leader = None;
        }
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(unavailable("outbox: try_lead"))?
            .detach();
        let leads: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(self.lock_key)
            .fetch_one(&mut conn)
            .await
            .map_err(unavailable("outbox: try_lead"))?;
        if leads {
            tracing::info!("outbox: this worker relays");
            *leader = Some(conn);
        }
        Ok(leads)
    }

    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<EventEnvelope>, TransportError> {
        let rows = sqlx::query(&format!(
            "SELECT {ENVELOPE_COLUMNS} FROM outbox o \
             WHERE o.published_at IS NULL ORDER BY o.seq LIMIT $1"
        ))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(unavailable("outbox: fetch_unpublished"))?;
        rows.iter()
            .map(|row| envelope_from_pg(row, "outbox: fetch_unpublished"))
            .collect()
    }

    async fn mark_published(&self, ids: &[Uuid]) -> Result<(), TransportError> {
        sqlx::query(
            "UPDATE outbox SET published_at = now() \
             WHERE id = ANY($1) AND published_at IS NULL",
        )
        .bind(ids)
        .execute(&self.pool)
        .await
        .map_err(unavailable("outbox: mark_published"))?;
        Ok(())
    }
}
