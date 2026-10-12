//! The SMTP adapter of [`Mailer`]: `lettre` over tokio and rustls, with pooled connections.

use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use puerto_core::mail::{Email, MailError, Mailer};

use crate::config::{MailConfig, MailConfigError};

/// Sends every [`Email`] from `MAIL_FROM` through the server at `SMTP_URL`.
#[derive(Clone)]
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpMailer {
    /// Opens no connection: the first send does, so a server that is down shows up as a retry,
    /// not as a worker that will not start. Call it inside the tokio runtime (`build_worker`):
    /// the connection pool lives on it.
    pub fn new(config: &MailConfig) -> Result<Self, MailConfigError> {
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(&config.smtp_url)
            .map_err(|_| MailConfig::invalid_smtp_url())?
            .build();
        let from = config.from.parse().map_err(|_| MailConfigError::Invalid {
            key: "MAIL_FROM",
            expected: "an address, like no-reply@example.com",
        })?;
        Ok(Self { transport, from })
    }

    fn message(&self, email: &Email) -> Result<Message, MailError> {
        let to: Mailbox = email
            .to
            .parse()
            .map_err(|e| MailError::Permanent(format!("recipient {:?}: {e}", email.to)))?;
        let builder = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(email.subject.clone());
        let message = match &email.text {
            Some(text) => builder.multipart(MultiPart::alternative_plain_html(
                text.clone(),
                email.html.clone(),
            )),
            None => builder
                .header(ContentType::TEXT_HTML)
                .body(email.html.clone()),
        };
        message.map_err(|e| MailError::Permanent(format!("message: {e}")))
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, email: &Email) -> Result<(), MailError> {
        let message = self.message(email)?;
        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(classify)
            .inspect_err(|error| tracing::warn!(%error, "smtp: send failed"))
    }
}

/// A 5xx answer is final; anything else (4xx, refused, timeout, TLS) may pass on a later try.
fn classify(error: smtp::Error) -> MailError {
    if error.is_permanent() {
        MailError::Permanent(error.to_string())
    } else {
        MailError::Transient(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mailer(smtp_url: &str) -> SmtpMailer {
        let config = MailConfig::from_vars(|key| match key {
            "SMTP_URL" => Some(smtp_url.to_string()),
            "MAIL_FROM" => Some("Shop <no-reply@shop.localhost>".to_string()),
            _ => None,
        })
        .unwrap();
        SmtpMailer::new(&config).unwrap()
    }

    /// A local port nothing listens on: bound, then released.
    fn closed_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[tokio::test]
    async fn should_fail_permanently_when_the_recipient_is_not_an_address() {
        // Arrange
        let mailer = mailer(&format!("smtp://127.0.0.1:{}", closed_port()));

        // Act
        let result = mailer
            .send(&Email::new("not an address", "Hi", "<p>Hi</p>"))
            .await;

        // Assert
        assert!(matches!(result, Err(MailError::Permanent(_))), "{result:?}");
    }

    #[tokio::test]
    async fn should_fail_transiently_when_the_server_cannot_be_reached() {
        // Arrange
        let mailer = mailer(&format!("smtp://127.0.0.1:{}", closed_port()));

        // Act
        let result = mailer
            .send(&Email::new("ada@example.com", "Hi", "<p>Hi</p>"))
            .await;

        // Assert
        assert!(matches!(result, Err(MailError::Transient(_))), "{result:?}");
    }

    #[tokio::test]
    async fn should_build_a_multipart_message_when_the_mail_has_text() {
        let mailer = mailer("smtp://localhost:1025");
        let email = Email::new("ada@example.com", "Welcome", "<p>Hi</p>").with_text("Hi");

        let formatted = String::from_utf8(mailer.message(&email).unwrap().formatted()).unwrap();

        assert!(formatted.contains("multipart/alternative"), "{formatted}");
        assert!(
            formatted.contains("From: Shop <no-reply@shop.localhost>"),
            "{formatted}"
        );
        assert!(formatted.contains("To: ada@example.com"), "{formatted}");
        assert!(formatted.contains("Subject: Welcome"), "{formatted}");
    }

    #[tokio::test]
    async fn should_build_an_html_message_when_the_mail_has_no_text() {
        let mailer = mailer("smtp://localhost:1025");

        let message = mailer
            .message(&Email::new("ada@example.com", "Welcome", "<p>Hi</p>"))
            .unwrap();
        let formatted = String::from_utf8(message.formatted()).unwrap();

        assert!(formatted.contains("Content-Type: text/html"), "{formatted}");
        assert!(!formatted.contains("multipart"), "{formatted}");
    }
}
