use async_trait::async_trait;
use puerto_nucleo::eventos::transporte::{DestinoDescartados, ErrorTransporte, RegistroDescartado};
use sqlx::PgPool;
use uuid::Uuid;

use super::unavailable;

/// `descartados`, whatever the transport. Recording the same event for the same consumer twice
/// keeps the first record (`descartados_evento_consumidor`).
pub struct DescartadosPostgres {
    pool: PgPool,
}

impl DescartadosPostgres {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DestinoDescartados for DescartadosPostgres {
    async fn registrar(&self, registrar: &RegistroDescartado) -> Result<(), ErrorTransporte> {
        let attempts = i32::try_from(registrar.intentos).unwrap_or(i32::MAX);
        sqlx::query(
            "INSERT INTO descartados \
             (id, id_evento, consumidor, tipo_evento, intentos, motivo, detalle, payload) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (id_evento, consumidor) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(registrar.id_evento)
        .bind(&registrar.consumidor)
        .bind(&registrar.tipo_evento)
        .bind(attempts)
        .bind(&registrar.motivo)
        .bind(&registrar.detalle)
        .bind(&registrar.payload)
        .execute(&self.pool)
        .await
        .map_err(unavailable("descartados: registrar"))?;
        tracing::warn!(
            consumer = %registrar.consumidor,
            event_id = ?registrar.id_evento,
            reason = %registrar.motivo,
            "descartados: registrado"
        );
        Ok(())
    }
}
