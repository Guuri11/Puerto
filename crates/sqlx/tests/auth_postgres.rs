//! Local accounts against a real server — `make test/db`. Every test runs in a schema of its own,
//! with `migrations/users.sql` and `migrations/auth.sql` applied.

use std::time::Duration;

use puerto_core::auth::{AuthError, Authenticator, CredentialStore, SessionStore};
use puerto_sqlx::auth::{PgCredentials, PgSessions};
use puerto_sqlx::{AUTH_MIGRATION, USERS_MIGRATION};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

const SKIP: &str = "PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)";
const THIRTY_DAYS: Duration = Duration::from_secs(30 * 24 * 3600);

async fn database() -> Option<PgPool> {
    let server = std::env::var("PUERTO_TEST_DATABASE_URL").ok()?;
    let url = format!("{server}/postgres");
    let schema = format!("auth_{}", Uuid::now_v7().simple());
    let admin = PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;

    let options: PgConnectOptions = url.parse().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options.options([("search_path", schema.as_str())]))
        .await
        .unwrap();
    for migration in [USERS_MIGRATION, AUTH_MIGRATION] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    Some(pool)
}

/// A local user, as `register_user` writes one: issuer `local`, the email as subject.
async fn ada(pool: &PgPool, roles: &[&str]) -> Uuid {
    let id = Uuid::now_v7();
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    sqlx::query(
        "INSERT INTO users (id, issuer, subject, email, name, roles, created_at, updated_at) \
         VALUES ($1, 'local', 'ada@example.com', 'ada@example.com', 'Ada', $2, now(), now())",
    )
    .bind(id)
    .bind(roles)
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_session_authenticates_as_its_user_with_the_roles_column() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let user_id = ada(&pool, &["admin"]).await;
    let sessions = PgSessions::new(pool, THIRTY_DAYS);

    let session = sessions.open(user_id).await.unwrap();
    let principal = sessions.authenticate(&session.token).await.unwrap();

    assert_eq!(principal.user_id, user_id);
    assert_eq!(principal.subject, "ada@example.com");
    assert_eq!(principal.email.as_deref(), Some("ada@example.com"));
    assert_eq!(principal.name.as_deref(), Some("Ada"));
    assert_eq!(principal.roles, vec!["admin"]);
    let lifetime = session.expires_at - chrono::Utc::now();
    assert!(lifetime > chrono::TimeDelta::days(29), "{lifetime}");
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_session_keeps_only_the_hash_of_its_token() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let user_id = ada(&pool, &[]).await;
    let sessions = PgSessions::new(pool.clone(), THIRTY_DAYS);

    let session = sessions.open(user_id).await.unwrap();

    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored.len(), 32, "a SHA-256");
    assert_ne!(stored, session.token.as_bytes());
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_unknown_or_revoked_token_is_invalid() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let user_id = ada(&pool, &[]).await;
    let sessions = PgSessions::new(pool, THIRTY_DAYS);
    let session = sessions.open(user_id).await.unwrap();
    let other = sessions.open(user_id).await.unwrap();

    sessions.revoke(&session.token).await.unwrap();

    assert_eq!(
        sessions.authenticate(&session.token).await,
        Err(AuthError::Invalid)
    );
    assert_eq!(
        sessions.authenticate("made-up").await,
        Err(AuthError::Invalid)
    );
    // Revoking one session leaves the user's others open; revoking twice is fine.
    assert!(sessions.authenticate(&other.token).await.is_ok());
    assert_eq!(sessions.revoke(&session.token).await, Ok(()));
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_session_past_its_expiry_is_expired() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let user_id = ada(&pool, &[]).await;
    let sessions = PgSessions::new(pool, Duration::ZERO);

    let session = sessions.open(user_id).await.unwrap();

    assert_eq!(
        sessions.authenticate(&session.token).await,
        Err(AuthError::Expired)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_sessions_are_unavailable_when_their_table_is_missing() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    sqlx::raw_sql("DROP TABLE sessions")
        .execute(&pool)
        .await
        .unwrap();
    let sessions = PgSessions::new(pool, THIRTY_DAYS);

    assert_eq!(
        sessions.authenticate("any").await,
        Err(AuthError::Unavailable)
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_credentials_are_set_replaced_and_read_by_user() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let user_id = ada(&pool, &[]).await;
    let credentials = PgCredentials::new(pool);

    assert_eq!(credentials.password_hash(user_id).await, Ok(None));
    credentials
        .set_password_hash(user_id, "first")
        .await
        .unwrap();
    credentials
        .set_password_hash(user_id, "second")
        .await
        .unwrap();

    assert_eq!(
        credentials.password_hash(user_id).await,
        Ok(Some("second".to_string()))
    );
    assert_eq!(credentials.password_hash(Uuid::now_v7()).await, Ok(None));
}
