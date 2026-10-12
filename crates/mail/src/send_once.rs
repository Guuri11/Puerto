//! The step a worker consumer sends a mail through (ADR-0020 § 4): at-least-once delivery in,
//! one mail out — or, if the process dies between the send and the mark, two. Never none.

use std::future::Future;

use puerto_core::events::{HandlerOutcome, REASON_PERMANENT_ERROR};
use puerto_core::mail::{MailError, SentMails};
use uuid::Uuid;

/// Sends `mail` for the event `event_id` unless `sent` says it already went out, then records it.
///
/// | | Outcome |
/// |---|---|
/// | already sent | `Ack`, without sending |
/// | the log cannot be read | `Retry` |
/// | sent | `Ack` (also when the mark fails: retrying would send it again) |
/// | [`MailError::Permanent`] | dead letter `permanent_error` |
/// | [`MailError::Transient`] | `Retry` |
///
/// `send` is not polled unless the mail is due, so it can be the `send(…)` call itself.
pub async fn send_once(
    sent: &dyn SentMails,
    event_id: Uuid,
    mail: &str,
    send: impl Future<Output = Result<(), MailError>>,
) -> HandlerOutcome {
    match sent.was_sent(event_id, mail).await {
        Ok(true) => {
            tracing::info!(%event_id, mail, "mail already sent for this event: skipped");
            return HandlerOutcome::Ack;
        }
        Ok(false) => {}
        Err(error) => {
            tracing::warn!(%event_id, mail, %error, "mail: the sent log cannot be read, retrying");
            return HandlerOutcome::Retry;
        }
    }
    match send.await {
        Ok(()) => {}
        Err(MailError::Permanent(detail)) => {
            tracing::error!(%event_id, mail, %detail, "mail: rejected, dead-lettering");
            return HandlerOutcome::dead_letter(REASON_PERMANENT_ERROR, detail);
        }
        Err(error) => {
            tracing::warn!(%event_id, mail, %error, "mail: not sent, retrying");
            return HandlerOutcome::Retry;
        }
    }
    if let Err(error) = sent.mark_sent(event_id, mail).await {
        tracing::error!(
            %event_id, mail, %error,
            "mail: sent but not recorded — a redelivery of this event would send it again"
        );
    }
    tracing::info!(%event_id, mail, "mail: sent");
    HandlerOutcome::Ack
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use puerto_core::RepositoryError;

    use super::*;
    use crate::testing::InMemorySentMails;

    const MAIL: &str = "welcome";

    /// A send that counts how often it ran and answers `result`.
    async fn counted(calls: &AtomicUsize, result: Result<(), MailError>) -> Result<(), MailError> {
        calls.fetch_add(1, Ordering::SeqCst);
        result
    }

    #[tokio::test]
    async fn should_send_and_record_the_mail_when_it_was_not_sent_yet() {
        // Arrange
        let sent = InMemorySentMails::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = send_once(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, HandlerOutcome::Ack);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(sent.was_sent(event_id, MAIL).await, Ok(true));
    }

    #[tokio::test]
    async fn should_not_send_again_when_the_same_event_is_delivered_twice() {
        // Arrange
        let sent = InMemorySentMails::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        // Act
        let first = send_once(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;
        let second = send_once(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!((first, second), (HandlerOutcome::Ack, HandlerOutcome::Ack));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn should_send_each_mail_when_one_event_triggers_two() {
        let sent = InMemorySentMails::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        send_once(&sent, event_id, "welcome", counted(&calls, Ok(()))).await;
        send_once(&sent, event_id, "receipt", counted(&calls, Ok(()))).await;

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn should_retry_without_sending_when_the_log_cannot_be_read() {
        // Arrange
        let sent = InMemorySentMails::unavailable();
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = send_once(&sent, Uuid::now_v7(), MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, HandlerOutcome::Retry);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_retry_and_record_nothing_when_the_send_fails_transiently() {
        // Arrange
        let sent = InMemorySentMails::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);
        let refused = Err(MailError::Transient("connection refused".into()));

        // Act
        let outcome = send_once(&sent, event_id, MAIL, counted(&calls, refused)).await;

        // Assert
        assert_eq!(outcome, HandlerOutcome::Retry);
        assert_eq!(sent.was_sent(event_id, MAIL).await, Ok(false));
    }

    #[tokio::test]
    async fn should_dead_letter_and_record_nothing_when_the_send_fails_permanently() {
        // Arrange
        let sent = InMemorySentMails::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);
        let rejected = Err(MailError::Permanent("550 no such user".into()));

        // Act
        let outcome = send_once(&sent, event_id, MAIL, counted(&calls, rejected)).await;

        // Assert
        assert_eq!(
            outcome,
            HandlerOutcome::dead_letter(REASON_PERMANENT_ERROR, "550 no such user")
        );
        assert_eq!(sent.was_sent(event_id, MAIL).await, Ok(false));
    }

    #[tokio::test]
    async fn should_ack_when_the_mail_went_out_but_could_not_be_recorded() {
        // Arrange
        let sent = InMemorySentMails::failing_to_mark(RepositoryError::Unavailable);
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = send_once(&sent, Uuid::now_v7(), MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, HandlerOutcome::Ack);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
