//! Mail (ADR-0020): the message a project sends and the ports that send it. A project's
//! `{Mail}EnviadorCorreo` port takes its own typed data; its implementation renders the templates into an
//! [`Correo`] and hands it to a [`EnviadorCorreo`]. The SMTP adapter is `puerto-correo`'s, the sent log
//! `puerto-sqlx`'s.

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::ErrorRepositorio;

/// One message, rendered: who it goes to and what it says. The sender is the [`EnviadorCorreo`]'s.
///
/// `#[non_exhaustive]`: built with [`Correo::new`], so `cc` or `reply_to` later break no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correo {
    pub para: String,
    pub asunto: String,
    pub html: String,
    /// The plain-text alternative; without one the message is HTML only.
    pub texto: Option<String>,
}

impl Correo {
    pub fn new(
        para: impl Into<String>,
        asunto: impl Into<String>,
        html: impl Into<String>,
    ) -> Self {
        Self {
            para: para.into(),
            asunto: asunto.into(),
            html: html.into(),
            texto: None,
        }
    }

    pub fn con_texto(mut self, texto: impl Into<String>) -> Self {
        self.texto = Some(texto.into());
        self
    }
}

/// Why a message was not sent, as far as a caller needs to know: whether trying again can help.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorCorreo {
    /// It may pass: the server answered 4xx, could not be reached, or timed out.
    #[error("correo.transitorio: {0}")]
    Transitorio(String),
    /// It never will: the address or the message was rejected (5xx), or a template is broken.
    #[error("correo.permanente: {0}")]
    Permanente(String),
}

/// Sends a rendered message. `puerto_correo::EnviadorCorreoSmtp` in a project, a recording fake in tests.
#[async_trait]
pub trait EnviadorCorreo: Send + Sync {
    async fn enviar(&self, email: &Correo) -> Result<(), ErrorCorreo>;
}

/// Which mails went out for which event: what keeps an at-least-once delivery from sending the
/// same mail twice. Keyed by the event's id and the mail's name.
#[async_trait]
pub trait CorreosEnviados: Send + Sync {
    async fn fue_enviado(&self, id_evento: Uuid, correo: &str) -> Result<bool, ErrorRepositorio>;
    /// Marking a mail already marked is not an error.
    async fn marcar_enviado(&self, id_evento: Uuid, correo: &str) -> Result<(), ErrorRepositorio>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_html_only_when_built_without_text() {
        // Arrange / Act
        let email = Correo::new("ada@example.com", "Welcome", "<p>Hi Ada</p>");

        // Assert
        assert_eq!(email.para, "ada@example.com");
        assert_eq!(email.asunto, "Welcome");
        assert_eq!(email.html, "<p>Hi Ada</p>");
        assert_eq!(email.texto, None);
    }

    #[test]
    fn should_carry_the_text_alternative_when_given_one() {
        // Arrange / Act
        let email = Correo::new("ada@example.com", "Welcome", "<p>Hi</p>").con_texto("Hi");

        // Assert
        assert_eq!(email.texto.as_deref(), Some("Hi"));
    }

    #[test]
    fn should_keep_error_codes_stable_when_logged() {
        assert_eq!(
            ErrorCorreo::Transitorio("refused".into()).to_string(),
            "correo.transitorio: refused"
        );
        assert_eq!(
            ErrorCorreo::Permanente("550 no such user".into()).to_string(),
            "correo.permanente: 550 no such user"
        );
    }
}
