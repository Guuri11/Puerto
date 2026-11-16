//! The step a worker consumer sends a mail through (ADR-0020 § 4): at-least-once delivery in,
//! one mail out — or, if the process dies between the send and the mark, two. Never none.

use std::future::Future;

use puerto_nucleo::correo::{CorreosEnviados, ErrorCorreo};
use puerto_nucleo::eventos::{MOTIVO_ERROR_PERMANENTE, ResultadoManejo};
use uuid::Uuid;

/// Sends `mail` for the event `id_evento` unless `enviados` says it already went out, then records it.
///
/// | | Outcome |
/// |---|---|
/// | already sent | `Ack`, without sending |
/// | the log cannot be read | `Reintentar` |
/// | sent | `Ack` (also when the mark fails: retrying would send it again) |
/// | [`ErrorCorreo::Permanente`] | `Descartar`, `error_permanente` |
/// | [`ErrorCorreo::Transitorio`] | `Reintentar` |
///
/// `send` is not polled unless the mail is due, so it can be the `send(…)` call itself.
pub async fn enviar_una_vez(
    sent: &dyn CorreosEnviados,
    event_id: Uuid,
    mail: &str,
    send: impl Future<Output = Result<(), ErrorCorreo>>,
) -> ResultadoManejo {
    match sent.fue_enviado(event_id, mail).await {
        Ok(true) => {
            tracing::info!(%event_id, mail, "correo ya enviado para este evento: se omite");
            return ResultadoManejo::Ack;
        }
        Ok(false) => {}
        Err(error) => {
            tracing::warn!(%event_id, mail, %error, "correo: no se puede leer el registro de enviados, se reintenta");
            return ResultadoManejo::Reintentar;
        }
    }
    match send.await {
        Ok(()) => {}
        Err(ErrorCorreo::Permanente(detail)) => {
            tracing::error!(%event_id, mail, %detail, "correo: rechazado, se descarta");
            return ResultadoManejo::descartar(MOTIVO_ERROR_PERMANENTE, detail);
        }
        Err(error) => {
            tracing::warn!(%event_id, mail, %error, "correo: no enviado, se reintenta");
            return ResultadoManejo::Reintentar;
        }
    }
    if let Err(error) = sent.marcar_enviado(event_id, mail).await {
        tracing::error!(
            %event_id, mail, %error,
            "correo: enviado pero no registrado; si el evento se reentrega se enviaría otra vez"
        );
    }
    tracing::info!(%event_id, mail, "correo: enviado");
    ResultadoManejo::Ack
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use puerto_nucleo::ErrorRepositorio;

    use super::*;
    use crate::pruebas::CorreosEnviadosEnMemoria;

    const MAIL: &str = "welcome";

    /// A send that counts how often it ran and answers `result`.
    async fn counted(
        calls: &AtomicUsize,
        result: Result<(), ErrorCorreo>,
    ) -> Result<(), ErrorCorreo> {
        calls.fetch_add(1, Ordering::SeqCst);
        result
    }

    #[tokio::test]
    async fn should_send_and_record_the_mail_when_it_was_not_sent_yet() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = enviar_una_vez(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, ResultadoManejo::Ack);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(sent.fue_enviado(event_id, MAIL).await, Ok(true));
    }

    #[tokio::test]
    async fn should_not_send_again_when_the_same_event_is_delivered_twice() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        // Act
        let first = enviar_una_vez(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;
        let second = enviar_una_vez(&sent, event_id, MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(
            (first, second),
            (ResultadoManejo::Ack, ResultadoManejo::Ack)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn should_send_each_mail_when_one_event_triggers_two() {
        let sent = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);

        enviar_una_vez(&sent, event_id, "welcome", counted(&calls, Ok(()))).await;
        enviar_una_vez(&sent, event_id, "receipt", counted(&calls, Ok(()))).await;

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn should_retry_without_sending_when_the_log_cannot_be_read() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::no_disponible();
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = enviar_una_vez(&sent, Uuid::now_v7(), MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, ResultadoManejo::Reintentar);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_retry_and_record_nothing_when_the_send_fails_transiently() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);
        let refused = Err(ErrorCorreo::Transitorio("connection refused".into()));

        // Act
        let outcome = enviar_una_vez(&sent, event_id, MAIL, counted(&calls, refused)).await;

        // Assert
        assert_eq!(outcome, ResultadoManejo::Reintentar);
        assert_eq!(sent.fue_enviado(event_id, MAIL).await, Ok(false));
    }

    #[tokio::test]
    async fn should_dead_letter_and_record_nothing_when_the_send_fails_permanently() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();
        let calls = AtomicUsize::new(0);
        let rejected = Err(ErrorCorreo::Permanente("550 no such user".into()));

        // Act
        let outcome = enviar_una_vez(&sent, event_id, MAIL, counted(&calls, rejected)).await;

        // Assert
        assert_eq!(
            outcome,
            ResultadoManejo::descartar(MOTIVO_ERROR_PERMANENTE, "550 no such user")
        );
        assert_eq!(sent.fue_enviado(event_id, MAIL).await, Ok(false));
    }

    #[tokio::test]
    async fn should_ack_when_the_mail_went_out_but_could_not_be_recorded() {
        // Arrange
        let sent = CorreosEnviadosEnMemoria::fallando_al_marcar(ErrorRepositorio::NoDisponible);
        let calls = AtomicUsize::new(0);

        // Act
        let outcome = enviar_una_vez(&sent, Uuid::now_v7(), MAIL, counted(&calls, Ok(()))).await;

        // Assert
        assert_eq!(outcome, ResultadoManejo::Ack);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
