//! The sent-mail log against a real server — `make test/db`. Every test runs in a schema of its
//! own, with `migrations/sent_mails.sql` applied.

use puerto_core::mail::SentMails;
use puerto_sqlx::SENT_MAILS_MIGRATION;
use puerto_sqlx::mail::PgSentMails;
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

const SKIP: &str = "PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)";

async fn database() -> Option<PgPool> {
    let server = std::env::var("PUERTO_TEST_DATABASE_URL").ok()?;
    let url = format!("{server}/postgres");
    let schema = format!("mail_{}", Uuid::now_v7().simple());
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
    sqlx::raw_sql(SENT_MAILS_MIGRATION)
        .execute(&pool)
        .await
        .unwrap();
    Some(pool)
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_mail_is_sent_only_once_marked_for_its_event_and_name() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let log = PgSentMails::new(pool);
    let event_id = Uuid::now_v7();

    assert_eq!(log.was_sent(event_id, "welcome").await, Ok(false));
    log.mark_sent(event_id, "welcome").await.unwrap();

    assert_eq!(log.was_sent(event_id, "welcome").await, Ok(true));
    assert_eq!(log.was_sent(event_id, "receipt").await, Ok(false));
    assert_eq!(log.was_sent(Uuid::now_v7(), "welcome").await, Ok(false));
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_marking_a_mail_twice_keeps_one_row() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let log = PgSentMails::new(pool.clone());
    let event_id = Uuid::now_v7();

    log.mark_sent(event_id, "welcome").await.unwrap();
    let second = log.mark_sent(event_id, "welcome").await;

    assert_eq!(second, Ok(()));
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM sent_mails")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_sent_log_is_unavailable_when_its_table_is_missing() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    sqlx::raw_sql("DROP TABLE sent_mails")
        .execute(&pool)
        .await
        .unwrap();
    let log = PgSentMails::new(pool);

    let read = log.was_sent(Uuid::now_v7(), "welcome").await;
    let mark = log.mark_sent(Uuid::now_v7(), "welcome").await;

    assert_eq!(read, Err(puerto_core::RepositoryError::Unavailable));
    assert_eq!(mark, Err(puerto_core::RepositoryError::Unavailable));
}
