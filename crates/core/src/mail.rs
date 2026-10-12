//! Mail (ADR-0020): the message a project sends and the ports that send it. A project's
//! `{Mail}Mailer` port takes its own typed data; its implementation renders the templates into an
//! [`Email`] and hands it to a [`Mailer`]. The SMTP adapter is `puerto-mail`'s, the sent log
//! `puerto-sqlx`'s.

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::RepositoryError;

/// One message, rendered: who it goes to and what it says. The sender is the [`Mailer`]'s.
///
/// `#[non_exhaustive]`: built with [`Email::new`], so `cc` or `reply_to` later break no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub html: String,
    /// The plain-text alternative; without one the message is HTML only.
    pub text: Option<String>,
}

impl Email {
    pub fn new(to: impl Into<String>, subject: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            subject: subject.into(),
            html: html.into(),
            text: None,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

/// Why a message was not sent, as far as a caller needs to know: whether trying again can help.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MailError {
    /// It may pass: the server answered 4xx, could not be reached, or timed out.
    #[error("mail.transient: {0}")]
    Transient(String),
    /// It never will: the address or the message was rejected (5xx), or a template is broken.
    #[error("mail.permanent: {0}")]
    Permanent(String),
}

/// Sends a rendered message. `puerto_mail::SmtpMailer` in a project, a recording fake in tests.
#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, email: &Email) -> Result<(), MailError>;
}

/// Which mails went out for which event: what keeps an at-least-once delivery from sending the
/// same mail twice. Keyed by the event's id and the mail's name.
#[async_trait]
pub trait SentMails: Send + Sync {
    async fn was_sent(&self, event_id: Uuid, mail: &str) -> Result<bool, RepositoryError>;
    /// Marking a mail already marked is not an error.
    async fn mark_sent(&self, event_id: Uuid, mail: &str) -> Result<(), RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_html_only_when_built_without_text() {
        // Arrange / Act
        let email = Email::new("ada@example.com", "Welcome", "<p>Hi Ada</p>");

        // Assert
        assert_eq!(email.to, "ada@example.com");
        assert_eq!(email.subject, "Welcome");
        assert_eq!(email.html, "<p>Hi Ada</p>");
        assert_eq!(email.text, None);
    }

    #[test]
    fn should_carry_the_text_alternative_when_given_one() {
        // Arrange / Act
        let email = Email::new("ada@example.com", "Welcome", "<p>Hi</p>").with_text("Hi");

        // Assert
        assert_eq!(email.text.as_deref(), Some("Hi"));
    }

    #[test]
    fn should_keep_error_codes_stable_when_logged() {
        assert_eq!(
            MailError::Transient("refused".into()).to_string(),
            "mail.transient: refused"
        );
        assert_eq!(
            MailError::Permanent("550 no such user".into()).to_string(),
            "mail.permanent: 550 no such user"
        );
    }
}
