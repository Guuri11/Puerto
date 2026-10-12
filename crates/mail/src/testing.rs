//! Fakes for a project's tests (feature `test-utils`): a mailer that records what it is handed and
//! a sent log in memory.

use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;
use puerto_core::RepositoryError;
use puerto_core::mail::{Email, MailError, Mailer, SentMails};
use uuid::Uuid;

/// Keeps every [`Email`] it is asked to send — or fails each one, with [`Self::failing_with`].
#[derive(Debug, Default)]
pub struct RecordingMailer {
    sent: Mutex<Vec<Email>>,
    failure: Option<MailError>,
}

impl RecordingMailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every send answers `error` and records nothing.
    pub fn failing_with(error: MailError) -> Self {
        Self {
            failure: Some(error),
            ..Self::default()
        }
    }

    /// What was sent, in order.
    pub fn sent(&self) -> Vec<Email> {
        self.sent.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[async_trait]
impl Mailer for RecordingMailer {
    async fn send(&self, email: &Email) -> Result<(), MailError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        self.sent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(email.clone());
        Ok(())
    }
}

/// The sent log in a `HashSet`; [`Self::unavailable`] and [`Self::failing_to_mark`] stand for a
/// database that is down.
#[derive(Debug, Default)]
pub struct InMemorySentMails {
    sent: Mutex<HashSet<(Uuid, String)>>,
    read_failure: Option<RepositoryError>,
    mark_failure: Option<RepositoryError>,
}

impl InMemorySentMails {
    pub fn new() -> Self {
        Self::default()
    }

    /// Neither read nor written: every call answers `RepositoryError::Unavailable`.
    pub fn unavailable() -> Self {
        Self {
            read_failure: Some(RepositoryError::Unavailable),
            mark_failure: Some(RepositoryError::Unavailable),
            ..Self::default()
        }
    }

    /// Reads, but `mark_sent` answers `error`.
    pub fn failing_to_mark(error: RepositoryError) -> Self {
        Self {
            mark_failure: Some(error),
            ..Self::default()
        }
    }
}

#[async_trait]
impl SentMails for InMemorySentMails {
    async fn was_sent(&self, event_id: Uuid, mail: &str) -> Result<bool, RepositoryError> {
        if let Some(error) = &self.read_failure {
            return Err(*error);
        }
        let sent = self.sent.lock().unwrap_or_else(|e| e.into_inner());
        Ok(sent.contains(&(event_id, mail.to_string())))
    }

    async fn mark_sent(&self, event_id: Uuid, mail: &str) -> Result<(), RepositoryError> {
        if let Some(error) = &self.mark_failure {
            return Err(*error);
        }
        self.sent
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((event_id, mail.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_record_every_mail_when_sending_succeeds() {
        let mailer = RecordingMailer::new();
        let email = Email::new("ada@example.com", "Welcome", "<p>Hi</p>");

        let result = mailer.send(&email).await;

        assert_eq!(result, Ok(()));
        assert_eq!(mailer.sent(), vec![email]);
    }

    #[tokio::test]
    async fn should_record_nothing_when_built_to_fail() {
        let mailer = RecordingMailer::failing_with(MailError::Transient("down".into()));

        let result = mailer.send(&Email::new("ada@example.com", "Hi", "")).await;

        assert_eq!(result, Err(MailError::Transient("down".into())));
        assert!(mailer.sent().is_empty());
    }

    #[tokio::test]
    async fn should_report_a_mail_sent_only_for_its_event_and_name() {
        let log = InMemorySentMails::new();
        let event_id = Uuid::now_v7();

        log.mark_sent(event_id, "welcome").await.unwrap();
        log.mark_sent(event_id, "welcome").await.unwrap();

        assert_eq!(log.was_sent(event_id, "welcome").await, Ok(true));
        assert_eq!(log.was_sent(event_id, "receipt").await, Ok(false));
        assert_eq!(log.was_sent(Uuid::now_v7(), "welcome").await, Ok(false));
    }

    #[tokio::test]
    async fn should_fail_every_call_when_built_unavailable() {
        let log = InMemorySentMails::unavailable();
        let event_id = Uuid::now_v7();

        assert_eq!(
            log.was_sent(event_id, "welcome").await,
            Err(RepositoryError::Unavailable)
        );
        assert_eq!(
            log.mark_sent(event_id, "welcome").await,
            Err(RepositoryError::Unavailable)
        );
    }

    #[tokio::test]
    async fn should_read_but_not_record_when_built_failing_to_mark() {
        let log = InMemorySentMails::failing_to_mark(RepositoryError::Unavailable);
        let event_id = Uuid::now_v7();

        let marked = log.mark_sent(event_id, "welcome").await;

        assert_eq!(marked, Err(RepositoryError::Unavailable));
        assert_eq!(log.was_sent(event_id, "welcome").await, Ok(false));
    }
}
