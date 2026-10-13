//! The Postgres adapter every Puerto project with a database shares (ADR-0017): the connection
//! pool, migrations at startup, the readiness ping and the one mapping from driver errors to
//! [`RepositoryError`].
//!
//! The migrations are the project's own: `sqlx::migrate!` embeds a directory at compile time, so
//! the project declares its [`Migrator`] and hands it over. A generated `infrastructure/src/db.rs`
//! is
//!
//! ```ignore
//! pub use puerto_sqlx::*;
//!
//! pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
//! ```
//!
//! and its repositories call `crate::db::map_sqlx_error`.
//!
//! A project with domain events (ADR-0018) also uses [`uow::PgTransaction`] in its generated units
//! of work, and its worker the outbox, the `db` transport and the dead-letter sink in [`events`].
//! Their tables are [`OUTBOX_MIGRATION`], which `puerto new --events` writes into the project.
//!
//! A worker that sends mail (ADR-0020) keeps what it sent in [`mail::PgSentMails`], over
//! [`SENT_MAILS_MIGRATION`].
//!
//! A project with local accounts (ADR-0021 § 2) keeps their passwords and sessions in
//! [`auth::PgCredentials`] and [`auth::PgSessions`], over [`USERS_MIGRATION`] and
//! [`AUTH_MIGRATION`].

pub mod auth;
pub mod events;
pub mod mail;
pub mod uow;

use puerto_core::RepositoryError;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgConnection, PgPoolOptions};
use sqlx::{Connection, PgPool};
use std::time::Duration;

/// The outbox, delivery and dead-letter tables (`migrations/outbox.sql`). Puerto's CLI ships a
/// byte-for-byte copy of it as a project migration.
pub const OUTBOX_MIGRATION: &str = include_str!("../migrations/outbox.sql");

/// The sent-mail log (`migrations/sent_mails.sql`). Puerto's CLI ships a byte-for-byte copy of it
/// as a project migration.
pub const SENT_MAILS_MIGRATION: &str = include_str!("../migrations/sent_mails.sql");

/// The users every auth project has (`migrations/users.sql`): [`auth::PgSessions`] reads it, so its
/// schema is part of this crate's contract. Puerto's CLI ships a byte-for-byte copy of it as a
/// project migration.
pub const USERS_MIGRATION: &str = include_str!("../migrations/users.sql");

/// The credentials and sessions of local accounts (`migrations/auth.sql`), after
/// [`USERS_MIGRATION`]. Puerto's CLI ships a byte-for-byte copy of it as a project migration.
pub const AUTH_MIGRATION: &str = include_str!("../migrations/auth.sql");

/// Why the database could not be used at startup — read by the person starting the process, so
/// it says where and why, never the password.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DatabaseError {
    /// `target` is the URL's `host[:port]/database`: no user, no password.
    #[error("cannot connect to the database at {target}: {cause}")]
    ConnectionError { target: String, cause: String },
    #[error("cannot apply the database migrations: {cause}")]
    MigrationError { cause: String },
}

pub async fn create_postgres_pool(database_url: &str) -> Result<PgPool, DatabaseError> {
    let failed = |e: sqlx::Error| DatabaseError::ConnectionError {
        target: describe_url(database_url),
        cause: e.to_string(),
    };
    let options: PgConnectOptions = database_url.parse().map_err(failed)?;
    // One connection first, on its own: the pool retries a refused connection until its acquire
    // timeout and then reports only that it timed out, not why.
    let probe = PgConnection::connect_with(&options).await.map_err(failed)?;
    let _ = probe.close().await;
    Ok(PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect_lazy_with(options))
}

/// A connection URL as it can be shown: `host[:port]/database`, without the scheme, the
/// credentials and the query. A string that is not a URL comes back as it is.
fn describe_url(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, rest)| rest);
    rest.split('?').next().unwrap_or(rest).to_string()
}

/// Applies the project's pending migrations — `migrator` is its `sqlx::migrate!("./migrations")`.
pub async fn run_migrations(pool: &PgPool, migrator: &Migrator) -> Result<(), DatabaseError> {
    migrator
        .run(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError {
            cause: e.to_string(),
        })
}

/// Whether the database answers a trivial query — what `/health/ready` reports.
///
/// Bounded by the pool's acquire timeout, so a database that is down makes the probe slow to
/// answer rather than hang forever.
pub async fn ping(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

/// The one place that knows Postgres error codes: a driver error, as the repository port speaks it.
/// The caller logs it first, with its own context.
pub fn map_sqlx_error(err: &sqlx::Error) -> RepositoryError {
    // 23505 = unique_violation: a `unique` column already taken. (No let-chain: `rust-version`.)
    let unique_violation = err
        .as_database_error()
        .and_then(|db_err| db_err.code())
        .is_some_and(|code| code == "23505");
    if unique_violation {
        RepositoryError::Duplicate
    } else {
        RepositoryError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pool that never reaches a server: nothing listens on port 1.
    fn unreachable_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(200))
            .connect_lazy("postgres://puerto@127.0.0.1:1/puerto")
            .unwrap()
    }

    /// A driver error carrying a Postgres SQLSTATE, without a server.
    #[derive(Debug)]
    struct FakeDbError(&'static str);

    impl std::fmt::Display for FakeDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "SQLSTATE {}", self.0)
        }
    }

    impl std::error::Error for FakeDbError {}

    impl sqlx::error::DatabaseError for FakeDbError {
        fn message(&self) -> &str {
            self.0
        }
        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(self.0.into())
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    fn database_error(code: &'static str) -> sqlx::Error {
        sqlx::Error::Database(Box::new(FakeDbError(code)))
    }

    #[test]
    fn should_map_a_unique_violation_to_duplicate() {
        assert_eq!(
            map_sqlx_error(&database_error("23505")),
            RepositoryError::Duplicate
        );
    }

    #[test]
    fn should_map_another_database_error_to_unavailable() {
        // 23503 = foreign_key_violation: a constraint, but not a taken unique value.
        assert_eq!(
            map_sqlx_error(&database_error("23503")),
            RepositoryError::Unavailable
        );
    }

    #[test]
    fn should_map_a_non_database_error_to_unavailable() {
        assert_eq!(
            map_sqlx_error(&sqlx::Error::PoolTimedOut),
            RepositoryError::Unavailable
        );
    }

    #[tokio::test]
    async fn should_report_not_ready_when_the_database_is_unreachable() {
        assert!(!ping(&unreachable_pool()).await);
    }

    #[tokio::test]
    async fn should_reject_a_database_url_that_is_not_one() {
        let result = create_postgres_pool("not a database url").await;

        assert!(matches!(result, Err(DatabaseError::ConnectionError { .. })));
    }

    #[tokio::test]
    async fn should_say_where_and_why_the_connection_failed_without_the_password() {
        // Arrange: nothing listens on port 1.
        let url = "postgres://devuser:s3cret@127.0.0.1:1/shop";

        // Act
        let started = std::time::Instant::now();
        let message = create_postgres_pool(url).await.unwrap_err().to_string();

        // Assert
        assert!(message.contains("127.0.0.1:1/shop"), "{message}");
        assert!(message.to_lowercase().contains("refused"), "{message}");
        assert!(!message.contains("s3cret"), "{message}");
        assert!(!message.contains("devuser"), "{message}");
        // A database that is down is reported at once, not after the pool's acquire timeout.
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn should_describe_a_database_url_by_its_host_and_database_only() {
        assert_eq!(
            describe_url("postgres://u:p@db.internal:5432/shop?sslmode=require"),
            "db.internal:5432/shop"
        );
        assert_eq!(describe_url("postgres://localhost/shop"), "localhost/shop");
        assert_eq!(describe_url("not a database url"), "not a database url");
    }

    // ── Against a real server: `make test/db` ─────────────────────────────

    async fn test_database() -> Option<PgPool> {
        let server = std::env::var("PUERTO_TEST_DATABASE_URL").ok()?;
        Some(
            create_postgres_pool(&format!("{server}/postgres"))
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    #[ignore = "needs Postgres: make test/db"]
    async fn postgres_unique_violation_maps_to_duplicate() {
        let Some(pool) = test_database().await else {
            eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
            return;
        };
        let mut conn = pool.acquire().await.unwrap();
        // A temporary table lives and dies with this connection: nothing to clean up.
        sqlx::query("CREATE TEMPORARY TABLE skus (sku TEXT UNIQUE)")
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO skus VALUES ('A-1')")
            .execute(&mut *conn)
            .await
            .unwrap();

        let err = sqlx::query("INSERT INTO skus VALUES ('A-1')")
            .execute(&mut *conn)
            .await
            .unwrap_err();

        assert_eq!(map_sqlx_error(&err), RepositoryError::Duplicate);
    }

    #[tokio::test]
    #[ignore = "needs Postgres: make test/db"]
    async fn postgres_ping_reports_ready() {
        let Some(pool) = test_database().await else {
            eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
            return;
        };

        assert!(ping(&pool).await);
    }
}
