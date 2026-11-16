//! Fakes for a project's tests (feature `test-utils`): a mailer that records what it is handed and
//! a sent log in memory.

use std::collections::HashSet;
use std::sync::Mutex;

use async_trait::async_trait;
use puerto_nucleo::ErrorRepositorio;
use puerto_nucleo::correo::{Correo, CorreosEnviados, EnviadorCorreo, ErrorCorreo};
use uuid::Uuid;

/// Keeps every [`Correo`] it is asked to send — or fails each one, with [`Self::fallando_con`].
#[derive(Debug, Default)]
pub struct EnviadorCorreoGrabador {
    sent: Mutex<Vec<Correo>>,
    failure: Option<ErrorCorreo>,
}

impl EnviadorCorreoGrabador {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every send answers `error` and records nothing.
    pub fn fallando_con(error: ErrorCorreo) -> Self {
        Self {
            failure: Some(error),
            ..Self::default()
        }
    }

    /// What was sent, in order.
    pub fn enviados(&self) -> Vec<Correo> {
        self.sent.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[async_trait]
impl EnviadorCorreo for EnviadorCorreoGrabador {
    async fn enviar(&self, email: &Correo) -> Result<(), ErrorCorreo> {
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

/// The sent log in a `HashSet`; [`Self::no_disponible`] and [`Self::fallando_al_marcar`] stand for a
/// database that is down.
#[derive(Debug, Default)]
pub struct CorreosEnviadosEnMemoria {
    sent: Mutex<HashSet<(Uuid, String)>>,
    read_failure: Option<ErrorRepositorio>,
    mark_failure: Option<ErrorRepositorio>,
}

impl CorreosEnviadosEnMemoria {
    pub fn new() -> Self {
        Self::default()
    }

    /// Neither read nor written: every call answers `ErrorRepositorio::NoDisponible`.
    pub fn no_disponible() -> Self {
        Self {
            read_failure: Some(ErrorRepositorio::NoDisponible),
            mark_failure: Some(ErrorRepositorio::NoDisponible),
            ..Self::default()
        }
    }

    /// Reads, but `marcar_enviado` answers `error`.
    pub fn fallando_al_marcar(error: ErrorRepositorio) -> Self {
        Self {
            mark_failure: Some(error),
            ..Self::default()
        }
    }
}

#[async_trait]
impl CorreosEnviados for CorreosEnviadosEnMemoria {
    async fn fue_enviado(&self, event_id: Uuid, mail: &str) -> Result<bool, ErrorRepositorio> {
        if let Some(error) = &self.read_failure {
            return Err(*error);
        }
        let sent = self.sent.lock().unwrap_or_else(|e| e.into_inner());
        Ok(sent.contains(&(event_id, mail.to_string())))
    }

    async fn marcar_enviado(&self, event_id: Uuid, mail: &str) -> Result<(), ErrorRepositorio> {
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
        let mailer = EnviadorCorreoGrabador::new();
        let email = Correo::new("ada@example.com", "Welcome", "<p>Hi</p>");

        let result = mailer.enviar(&email).await;

        assert_eq!(result, Ok(()));
        assert_eq!(mailer.enviados(), vec![email]);
    }

    #[tokio::test]
    async fn should_record_nothing_when_built_to_fail() {
        let mailer = EnviadorCorreoGrabador::fallando_con(ErrorCorreo::Transitorio("down".into()));

        let result = mailer
            .enviar(&Correo::new("ada@example.com", "Hi", ""))
            .await;

        assert_eq!(result, Err(ErrorCorreo::Transitorio("down".into())));
        assert!(mailer.enviados().is_empty());
    }

    #[tokio::test]
    async fn should_report_a_mail_sent_only_for_its_event_and_name() {
        let log = CorreosEnviadosEnMemoria::new();
        let event_id = Uuid::now_v7();

        log.marcar_enviado(event_id, "welcome").await.unwrap();
        log.marcar_enviado(event_id, "welcome").await.unwrap();

        assert_eq!(log.fue_enviado(event_id, "welcome").await, Ok(true));
        assert_eq!(log.fue_enviado(event_id, "receipt").await, Ok(false));
        assert_eq!(log.fue_enviado(Uuid::now_v7(), "welcome").await, Ok(false));
    }

    #[tokio::test]
    async fn should_fail_every_call_when_built_unavailable() {
        let log = CorreosEnviadosEnMemoria::no_disponible();
        let event_id = Uuid::now_v7();

        assert_eq!(
            log.fue_enviado(event_id, "welcome").await,
            Err(ErrorRepositorio::NoDisponible)
        );
        assert_eq!(
            log.marcar_enviado(event_id, "welcome").await,
            Err(ErrorRepositorio::NoDisponible)
        );
    }

    #[tokio::test]
    async fn should_read_but_not_record_when_built_failing_to_mark() {
        let log = CorreosEnviadosEnMemoria::fallando_al_marcar(ErrorRepositorio::NoDisponible);
        let event_id = Uuid::now_v7();

        let marked = log.marcar_enviado(event_id, "welcome").await;

        assert_eq!(marked, Err(ErrorRepositorio::NoDisponible));
        assert_eq!(log.fue_enviado(event_id, "welcome").await, Ok(false));
    }
}
