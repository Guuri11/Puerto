//! The sent-mail log (ADR-0020): which mails a worker consumer already sent for which event. Its
//! table is [`crate::MIGRACION_CORREOS_ENVIADOS`], which the first `puerto generate correo --on` writes
//! into the project.

use async_trait::async_trait;
use puerto_nucleo::ErrorRepositorio;
use puerto_nucleo::correo::CorreosEnviados;
use sqlx::PgPool;
use uuid::Uuid;

use crate::mapear_error_sqlx;

/// `sent_mails`. Marking the same mail for the same event twice keeps one row.
pub struct CorreosEnviadosPostgres {
    pool: PgPool,
}

impl CorreosEnviadosPostgres {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CorreosEnviados for CorreosEnviadosPostgres {
    async fn fue_enviado(&self, event_id: Uuid, correo: &str) -> Result<bool, ErrorRepositorio> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM correos_enviados WHERE id_evento = $1 AND correo = $2)",
        )
        .bind(event_id)
        .bind(correo)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, %event_id, correo, "sent_mails: read failed");
            mapear_error_sqlx(&err)
        })
    }

    async fn marcar_enviado(&self, event_id: Uuid, correo: &str) -> Result<(), ErrorRepositorio> {
        sqlx::query(
            "INSERT INTO correos_enviados (id_evento, correo) VALUES ($1, $2) \
             ON CONFLICT (id_evento, correo) DO NOTHING",
        )
        .bind(event_id)
        .bind(correo)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|err| {
            tracing::error!(error = %err, %event_id, correo, "sent_mails: mark failed");
            mapear_error_sqlx(&err)
        })
    }
}
