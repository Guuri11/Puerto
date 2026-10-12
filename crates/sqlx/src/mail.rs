//! The sent-mail log (ADR-0020): which mails a worker consumer already sent for which event. Its
//! table is [`crate::SENT_MAILS_MIGRATION`], which the first `puerto generate mail --on` writes
//! into the project.

use async_trait::async_trait;
use puerto_core::RepositoryError;
use puerto_core::mail::SentMails;
use sqlx::PgPool;
use uuid::Uuid;

use crate::map_sqlx_error;

/// `sent_mails`. Marking the same mail for the same event twice keeps one row.
pub struct PgSentMails {
    pool: PgPool,
}

impl PgSentMails {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SentMails for PgSentMails {
    async fn was_sent(&self, event_id: Uuid, mail: &str) -> Result<bool, RepositoryError> {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM sent_mails WHERE event_id = $1 AND mail = $2)",
        )
        .bind(event_id)
        .bind(mail)
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, %event_id, mail, "sent_mails: read failed");
            map_sqlx_error(&err)
        })
    }

    async fn mark_sent(&self, event_id: Uuid, mail: &str) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO sent_mails (event_id, mail) VALUES ($1, $2) \
             ON CONFLICT (event_id, mail) DO NOTHING",
        )
        .bind(event_id)
        .bind(mail)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|err| {
            tracing::error!(error = %err, %event_id, mail, "sent_mails: mark failed");
            map_sqlx_error(&err)
        })
    }
}
