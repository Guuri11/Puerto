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
pub enum MailConfigError {
    #[error("{key} is not set — set it in .env (.env.example has one)")]
    Missing { key: &'static str },
    #[error("{key} is not valid: expected {expected}")]
    Invalid {
        key: &'static str,
        expected: &'static str,
    },
}

const SMTP_URL_EXPECTED: &str = "smtp://host:port, or smtps://user:password@host";
const MAIL_FROM_EXPECTED: &str =
    "an address, like no-reply@example.com or Shop <no-reply@example.com>";

/// Built by [`MailConfig::from_env`], or [`MailConfig::from_vars`] (tests).
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MailConfig {
    pub smtp_url: String,
    pub from: String,
}

impl MailConfig {
    /// Loads `.env` when there is one, then reads the process environment.
    pub fn from_env() -> Result<Self, MailConfigError> {
        dotenvy::dotenv().ok();
        Self::from_vars(|key| std::env::var(key).ok())
    }

    /// Reads the configuration from any variable source; an empty value counts as unset.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Self, MailConfigError> {
        let required = |key: &'static str| {
            var(key)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or(MailConfigError::Missing { key })
        };
        let smtp_url = required("SMTP_URL")?;
        if !(smtp_url.starts_with("smtp://") || smtp_url.starts_with("smtps://")) {
            return Err(MailConfigError::Invalid {
                key: "SMTP_URL",
                expected: SMTP_URL_EXPECTED,
            });
        }
        let from = required("MAIL_FROM")?;
        from.parse::<Mailbox>()
            .map_err(|_| MailConfigError::Invalid {
                key: "MAIL_FROM",
                expected: MAIL_FROM_EXPECTED,
            })?;
        Ok(Self { smtp_url, from })
    }

    pub(crate) fn invalid_smtp_url() -> MailConfigError {
        MailConfigError::Invalid {
            key: "SMTP_URL",
            expected: SMTP_URL_EXPECTED,
        }
    }
}

impl fmt::Debug for MailConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MailConfig")
            .field("smtp_url", &redact(&self.smtp_url))
            .field("from", &self.from)
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
        let config = MailConfig::from_vars(env);

        // Assert
        assert_eq!(
            config,
            Ok(MailConfig {
                smtp_url: "smtp://localhost:1025".into(),
                from: "Shop <no-reply@shop.localhost>".into(),
            })
        );
    }

    #[test]
    fn should_reject_the_config_when_smtp_url_is_missing_or_empty() {
        for env in [vec![], vec![("SMTP_URL", "  ")]] {
            let mut env = env;
            env.push(("MAIL_FROM", "no-reply@shop.localhost"));
            assert_eq!(
                MailConfig::from_vars(vars(&env)),
                Err(MailConfigError::Missing { key: "SMTP_URL" })
            );
        }
    }

    #[test]
    fn should_reject_the_config_when_mail_from_is_missing() {
        let env = vars(&[("SMTP_URL", "smtp://localhost:1025")]);
        assert_eq!(
            MailConfig::from_vars(env),
            Err(MailConfigError::Missing { key: "MAIL_FROM" })
        );
    }

    #[test]
    fn should_reject_the_config_when_smtp_url_is_not_smtp() {
        let env = vars(&[
            ("SMTP_URL", "https://user:secret@mail.example.com"),
            ("MAIL_FROM", "no-reply@shop.localhost"),
        ]);

        let error = MailConfig::from_vars(env).unwrap_err();

        assert_eq!(error, MailConfig::invalid_smtp_url());
        assert!(!error.to_string().contains("secret"), "{error}");
    }

    #[test]
    fn should_reject_the_config_when_mail_from_is_not_an_address() {
        let env = vars(&[
            ("SMTP_URL", "smtp://localhost:1025"),
            ("MAIL_FROM", "no-reply"),
        ]);
        assert!(matches!(
            MailConfig::from_vars(env),
            Err(MailConfigError::Invalid {
                key: "MAIL_FROM",
                ..
            })
        ));
    }

    #[test]
    fn should_hide_the_password_when_the_config_is_debug_printed() {
        let config = MailConfig {
            smtp_url: "smtps://mailer:s3cret@smtp.example.com".into(),
            from: "no-reply@example.com".into(),
        };

        let printed = format!("{config:?}");

        assert!(!printed.contains("s3cret"), "{printed}");
        assert!(
            printed.contains("smtps://mailer:***@smtp.example.com"),
            "{printed}"
        );
    }
}
