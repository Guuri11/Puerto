//! The mail configuration, read once at startup from the environment (and `.env`).
//!
//! | Variable | | |
//! |---|---|---|
//! | `SMTP_URL` | required | `smtp://localhost:1025` (Mailpit), `smtps://user:pass@smtp.example.com` |
//! | `MAIL_FROM` | required | the sender: `no-reply@shop.example`, `Shop <no-reply@shop.example>` |
//!
//! `SMTP_URL` may carry a password, so neither an error nor `Debug` ever shows it.

use std::fmt;

use lettre::message::Mailbox;

/// A mail variable that is missing or set to a value the mailer cannot use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorConfigCorreo {
    #[error("{clave} no está definida: defínela en .env (.env.example tiene una)")]
    Falta { clave: &'static str },
    #[error("{clave} no es válida: se esperaba {esperado}")]
    Invalido {
        clave: &'static str,
        esperado: &'static str,
    },
}

const SMTP_URL_EXPECTED: &str = "smtp://host:puerto, o smtps://usuario:contraseña@host";
const MAIL_FROM_EXPECTED: &str =
    "una dirección, como no-reply@example.com o Tienda <no-reply@example.com>";

/// Built by [`ConfigCorreo::from_env`], or [`ConfigCorreo::from_variables`] (tests).
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConfigCorreo {
    pub url_smtp: String,
    pub remitente: String,
}

impl ConfigCorreo {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, ErrorConfigCorreo> {
        dotenvy::dotenv().ok();
        Self::from_variables(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_variables(var: impl Fn(&str) -> Option<String>) -> Result<Self, ErrorConfigCorreo> {
        let required = |key: &'static str| {
            var(key)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or(ErrorConfigCorreo::Falta { clave: key })
        };
        let url_smtp = required("SMTP_URL")?;
        if !(url_smtp.starts_with("smtp://") || url_smtp.starts_with("smtps://")) {
            return Err(ErrorConfigCorreo::Invalido {
                clave: "SMTP_URL",
                esperado: SMTP_URL_EXPECTED,
            });
        }
        let from = required("MAIL_FROM")?;
        from.parse::<Mailbox>()
            .map_err(|_| ErrorConfigCorreo::Invalido {
                clave: "MAIL_FROM",
                esperado: MAIL_FROM_EXPECTED,
            })?;
        Ok(Self {
            url_smtp,
            remitente: from,
        })
    }

    pub(crate) fn invalid_smtp_url() -> ErrorConfigCorreo {
        ErrorConfigCorreo::Invalido {
            clave: "SMTP_URL",
            esperado: SMTP_URL_EXPECTED,
        }
    }
}

impl fmt::Debug for ConfigCorreo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MailConfig")
            .field("smtp_url", &redact(&self.url_smtp))
            .field("from", &self.remitente)
            .finish()
    }
}

/// `smtps://user:secret@host` → `smtps://user:***@host`.
fn redact(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    match rest.rsplit_once('@') {
        Some((credentials, host)) => match credentials.split_once(':') {
            Some((user, _)) => format!("{scheme}://{user}:***@{host}"),
            None => url.to_string(),
        },
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn should_read_the_smtp_url_and_sender_when_both_are_set() {
        // Arrange
        let env = vars(&[
            ("SMTP_URL", "smtp://localhost:1025"),
            ("MAIL_FROM", "Shop <no-reply@shop.localhost>"),
        ]);

        // Act
        let config = ConfigCorreo::from_variables(env);

        // Assert
        assert_eq!(
            config,
            Ok(ConfigCorreo {
                url_smtp: "smtp://localhost:1025".into(),
                remitente: "Shop <no-reply@shop.localhost>".into(),
            })
        );
    }

    #[test]
    fn should_reject_the_config_when_smtp_url_is_missing_or_empty() {
        for env in [vec![], vec![("SMTP_URL", "  ")]] {
            let mut env = env;
            env.push(("MAIL_FROM", "no-reply@shop.localhost"));
            assert_eq!(
                ConfigCorreo::from_variables(vars(&env)),
                Err(ErrorConfigCorreo::Falta { clave: "SMTP_URL" })
            );
        }
    }

    #[test]
    fn should_reject_the_config_when_mail_from_is_missing() {
        let env = vars(&[("SMTP_URL", "smtp://localhost:1025")]);
        assert_eq!(
            ConfigCorreo::from_variables(env),
            Err(ErrorConfigCorreo::Falta { clave: "MAIL_FROM" })
        );
    }

    #[test]
    fn should_reject_the_config_when_smtp_url_is_not_smtp() {
        let env = vars(&[
            ("SMTP_URL", "https://user:secret@mail.example.com"),
            ("MAIL_FROM", "no-reply@shop.localhost"),
        ]);

        let error = ConfigCorreo::from_variables(env).unwrap_err();

        assert_eq!(error, ConfigCorreo::invalid_smtp_url());
        assert!(!error.to_string().contains("secret"), "{error}");
    }

    #[test]
    fn should_reject_the_config_when_mail_from_is_not_an_address() {
        let env = vars(&[
            ("SMTP_URL", "smtp://localhost:1025"),
            ("MAIL_FROM", "no-reply"),
        ]);
        assert!(matches!(
            ConfigCorreo::from_variables(env),
            Err(ErrorConfigCorreo::Invalido {
                clave: "MAIL_FROM",
                ..
            })
        ));
    }

    #[test]
    fn should_hide_the_password_when_the_config_is_debug_printed() {
        let config = ConfigCorreo {
            url_smtp: "smtps://mailer:s3cret@smtp.example.com".into(),
            remitente: "no-reply@example.com".into(),
        };

        let printed = format!("{config:?}");

        assert!(!printed.contains("s3cret"), "{printed}");
        assert!(
            printed.contains("smtps://mailer:***@smtp.example.com"),
            "{printed}"
        );
    }
}
