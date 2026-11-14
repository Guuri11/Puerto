use async_trait::async_trait;
use puerto_nucleo::SobreEvento;
use puerto_nucleo::eventos::transporte::{ErrorTransporte, Outbox};
use sqlx::{PgConnection, PgPool};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{ENVELOPE_COLUMNS, envelope_from_pg, unavailable};

/// The advisory lock the relay holds: only one worker replica relays, so the outbox order is
/// kept. The bytes of `"puerto"`.
pub const CLAVE_BLOQUEO_RELAY: i64 = 0x7075_6572_746f;

/// The outbox, as the relay reads it.
pub struct OutboxPostgres {
    pool: PgPool,
    lock_key: i64,
    /// The connection holding the advisory lock, once this relay leads. A session lock lives as
    /// long as its connection, so it is kept out of the pool.
    leader: Mutex<Option<PgConnection>>,
}

impl OutboxPostgres {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            lock_key: CLAVE_BLOQUEO_RELAY,
            leader: Mutex::new(None),
        }
    }

    /// Another lock key: two projects relaying from one database each need their own.
    pub fn con_clave_bloqueo(mut self, key: i64) -> Self {
        self.lock_key = key;
        self
    }
}

#[async_trait]
impl Outbox for OutboxPostgres {
    async fn intentar_liderar(&self) -> Result<bool, ErrorTransporte> {
        let mut leader = self.leader.lock().await;
        if let Some(conn) = leader.as_mut() {
            if sqlx::query("SELECT 1").execute(&mut *conn).await.is_ok() {
                return Ok(true);
            }
            // The connection died, and the lock with it: compete for it again.
            tracing::warn!("outbox: se perdió la conexión del bloqueo del relay");
            *leader = None;
        }
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(unavailable("outbox: intentar_liderar"))?
            .detach();
        let leads: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(self.lock_key)
            .fetch_one(&mut conn)
            .await
            .map_err(unavailable("outbox: intentar_liderar"))?;
        if leads {
            tracing::info!("outbox: este worker hace el relay");
            *leader = Some(conn);
        }
        Ok(leads)
    }

    async fn buscar_sin_publicar(&self, limit: u32) -> Result<Vec<SobreEvento>, ErrorTransporte> {
        let rows = sqlx::query(&format!(
            "SELECT {ENVELOPE_COLUMNS} FROM outbox o \
             WHERE o.publicado_en IS NULL ORDER BY o.secuencia LIMIT $1"
        ))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(unavailable("outbox: buscar_sin_publicar"))?;
        rows.iter()
            .map(|row| envelope_from_pg(row, "outbox: buscar_sin_publicar"))
            .collect()
    }

    async fn marcar_publicados(&self, ids: &[Uuid]) -> Result<(), ErrorTransporte> {
        sqlx::query(
            "UPDATE outbox SET publicado_en = now() \
             WHERE id = ANY($1) AND publicado_en IS NULL",
        )
        .bind(ids)
        .execute(&self.pool)
        .await
        .map_err(unavailable("outbox: marcar_publicados"))?;
        Ok(())
    }
}
