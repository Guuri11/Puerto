//! `SmtpMailer` against a real SMTP server: a throwaway Mailpit started by `make test/mail`, which
//! accepts only recipients at `@accepted.test` (`MP_SMTP_ALLOWED_RECIPIENTS`).

use puerto_mail::{Email, MailConfig, MailError, Mailer, SmtpMailer};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} is not set — run `make test/mail`"))
}

fn mailer() -> SmtpMailer {
    let config = MailConfig::from_vars(|key| match key {
        "SMTP_URL" => Some(var("PUERTO_TEST_SMTP_URL")),
        "MAIL_FROM" => Some("Puerto <no-reply@puerto.test>".into()),
        _ => None,
    })
    .unwrap();
    SmtpMailer::new(&config).unwrap()
}

/// `GET <PUERTO_TEST_MAILPIT_API>/api/v1/messages`, over HTTP/1.0 so the body is never chunked.
async fn messages() -> Value {
    let api = var("PUERTO_TEST_MAILPIT_API");
    let host = api.trim_start_matches("http://");
    let mut stream = TcpStream::connect(host).await.unwrap();
    let request = format!("GET /api/v1/messages HTTP/1.0\r\nHost: {host}\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn unique_address(domain: &str) -> String {
    format!("{}@{domain}", uuid::Uuid::now_v7().simple())
}

#[tokio::test]
#[ignore = "needs Mailpit: make test/mail"]
async fn should_deliver_the_mail_when_the_server_accepts_it() {
    // Arrange
    let to = unique_address("accepted.test");
    let email = Email::new(&to, "Welcome, Ada", "<p>Hi Ada</p>").with_text("Hi Ada");

    // Act
    let result = mailer().send(&email).await;

    // Assert
    assert_eq!(result, Ok(()));
    let messages = messages().await;
    let delivered = messages["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["To"][0]["Address"] == to.as_str())
        .unwrap_or_else(|| panic!("{to} not in {messages}"));
    assert_eq!(delivered["Subject"], "Welcome, Ada");
    assert_eq!(delivered["From"]["Address"], "no-reply@puerto.test");
}

#[tokio::test]
#[ignore = "needs Mailpit: make test/mail"]
async fn should_fail_permanently_when_the_server_rejects_the_recipient() {
    // Arrange
    let email = Email::new(unique_address("rejected.test"), "Hi", "<p>Hi</p>");

    // Act
    let result = mailer().send(&email).await;

    // Assert
    assert!(matches!(result, Err(MailError::Permanent(_))), "{result:?}");
}
