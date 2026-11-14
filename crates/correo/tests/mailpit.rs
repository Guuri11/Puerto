//! `EnviadorCorreoSmtp` against a real SMTP server: a throwaway Mailpit started by `make test/mail`, which
//! accepts only recipients at `@accepted.test` (`MP_SMTP_ALLOWED_RECIPIENTS`).

use puerto_correo::{ConfigCorreo, Correo, EnviadorCorreo, EnviadorCorreoSmtp, ErrorCorreo};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SKIP: &str =
    "PUERTO_TEST_SMTP_URL / PUERTO_TEST_MAILPIT_API not set — skipping (run `make test/mail`)";

/// Mailpit's SMTP URL and API, when `make test/mail` started one.
fn mailpit() -> Option<(String, String)> {
    Some((
        std::env::var("PUERTO_TEST_SMTP_URL").ok()?,
        std::env::var("PUERTO_TEST_MAILPIT_API").ok()?,
    ))
}

fn mailer(url_smtp: &str) -> EnviadorCorreoSmtp {
    let config = ConfigCorreo::from_variables(|key| match key {
        "SMTP_URL" => Some(url_smtp.to_string()),
        "MAIL_FROM" => Some("Puerto <no-reply@puerto.test>".into()),
        _ => None,
    })
    .unwrap();
    EnviadorCorreoSmtp::new(&config).unwrap()
}

/// `GET <PUERTO_TEST_MAILPIT_API>/api/v1/messages`, over HTTP/1.0 so the body is never chunked.
async fn messages(api: &str) -> Value {
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
    let Some((url_smtp, api)) = mailpit() else {
        return eprintln!("{SKIP}");
    };
    // Arrange
    let to = unique_address("accepted.test");
    let email = Correo::new(&to, "Welcome, Ada", "<p>Hi Ada</p>").con_texto("Hi Ada");

    // Act
    let result = mailer(&url_smtp).enviar(&email).await;

    // Assert
    assert_eq!(result, Ok(()));
    let messages = messages(&api).await;
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
    let Some((url_smtp, _)) = mailpit() else {
        return eprintln!("{SKIP}");
    };
    // Arrange
    let email = Correo::new(unique_address("rejected.test"), "Hi", "<p>Hi</p>");

    // Act
    let result = mailer(&url_smtp).enviar(&email).await;

    // Assert
    assert!(
        matches!(result, Err(ErrorCorreo::Permanente(_))),
        "{result:?}"
    );
}
