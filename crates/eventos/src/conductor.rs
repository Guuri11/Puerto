//! What turns a relay or a consumer into a loop: pass after pass, a pause when there is nothing
//! to do, a heartbeat for `/health/ready`, and a clean stop on shutdown.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use puerto_nucleo::eventos::transporte::ErrorTransporte;
use tokio::sync::watch;
use tokio::time::Instant;

/// How long a pass may take on top of the loop's interval before the loop counts as stuck: a
/// consumer's batch runs its handlers one after another.
pub const GRACIA_ATASCO: Duration = Duration::from_secs(30);

/// What one pass left behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progreso {
    /// A full batch: run the next pass at once.
    More,
    /// Nothing more for now: wait the poll interval.
    Idle,
}

/// One unit of work a loop repeats: a relay batch or a consumer batch.
#[async_trait]
pub trait Pasada: Send {
    fn name(&self) -> String;
    async fn pass(&mut self) -> Result<Progreso, ErrorTransporte>;
}

/// When a loop last finished a pass that did not fail. Readiness reads it.
#[derive(Debug)]
pub struct Latido {
    name: String,
    interval: Duration,
    last: Mutex<Option<Instant>>,
}

impl Latido {
    pub fn new(name: String, interval: Duration) -> Self {
        Self {
            name,
            interval,
            last: Mutex::new(None),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn beat_at(&self, at: Instant) {
        *self.last.lock().unwrap_or_else(|p| p.into_inner()) = Some(at);
    }

    /// Whether the loop finished a good pass within its interval plus [`GRACIA_ATASCO`]. A loop
    /// that has not finished one yet, or whose passes keep failing (the database is down), is
    /// not fresh.
    pub fn esta_fresco(&self, now: Instant) -> bool {
        let last = *self.last.lock().unwrap_or_else(|p| p.into_inner());
        last.is_some_and(|last| {
            now.saturating_duration_since(last) <= self.interval + GRACIA_ATASCO
        })
    }
}

/// Runs `work` until `shutdown` turns `true`. A pass in flight when it does is finished first —
/// a claim is never cut in half.
pub async fn conducir(
    mut work: Box<dyn Pasada>,
    interval: Duration,
    heartbeat: Arc<Latido>,
    mut shutdown: watch::Receiver<bool>,
) {
    let name = work.name();
    tracing::info!(name, "worker: bucle iniciado");
    while !*shutdown.borrow() {
        let wait = match work.pass().await {
            Ok(Progreso::More) => {
                heartbeat.beat_at(Instant::now());
                continue;
            }
            Ok(Progreso::Idle) => {
                heartbeat.beat_at(Instant::now());
                interval
            }
            Err(err) => {
                tracing::warn!(name, error = %err, "worker: falló la pasada");
                interval
            }
        };
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = shutdown.changed() => {}
        }
    }
    tracing::info!(name, "worker: bucle detenido");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Answers the scripted results in turn, then `Idle`, counting its passes.
    struct Script {
        results: Vec<Result<Progreso, ErrorTransporte>>,
        passes: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Pasada for Script {
        fn name(&self) -> String {
            "script".into()
        }
        async fn pass(&mut self) -> Result<Progreso, ErrorTransporte> {
            self.passes.fetch_add(1, Ordering::SeqCst);
            if self.results.is_empty() {
                Ok(Progreso::Idle)
            } else {
                self.results.remove(0)
            }
        }
    }

    fn heartbeat() -> Arc<Latido> {
        Arc::new(Latido::new("script".into(), Duration::from_millis(500)))
    }

    #[tokio::test(start_paused = true)]
    async fn should_run_again_at_once_while_there_is_more_and_wait_when_idle() {
        // Arrange
        let passes = Arc::new(AtomicUsize::new(0));
        let script = Script {
            results: vec![Ok(Progreso::More), Ok(Progreso::More)],
            passes: Arc::clone(&passes),
        };
        let (stop, shutdown) = watch::channel(false);
        let task = tokio::spawn(conducir(
            Box::new(script),
            Duration::from_secs(10),
            heartbeat(),
            shutdown,
        ));

        // Act — two `More`, one `Idle`, then the 10 s pause
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Assert
        assert_eq!(passes.load(Ordering::SeqCst), 3);
        let _ = stop.send(true);
        let _ = task.await;
    }

    #[tokio::test(start_paused = true)]
    async fn should_stop_without_waiting_out_the_interval_when_shut_down() {
        // Arrange
        let passes = Arc::new(AtomicUsize::new(0));
        let script = Script {
            results: vec![],
            passes: Arc::clone(&passes),
        };
        let (stop, shutdown) = watch::channel(false);
        let task = tokio::spawn(conducir(
            Box::new(script),
            Duration::from_secs(3600),
            heartbeat(),
            shutdown,
        ));
        tokio::time::sleep(Duration::from_millis(1)).await;

        // Act
        let _ = stop.send(true);
        let stopped = tokio::time::timeout(Duration::from_secs(1), task).await;

        // Assert
        assert!(stopped.is_ok(), "the loop kept sleeping after shutdown");
        assert_eq!(passes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn should_beat_only_when_a_pass_succeeds() {
        // Arrange
        let beat = heartbeat();
        let script = Script {
            results: vec![Err(ErrorTransporte::NoDisponible)],
            passes: Arc::new(AtomicUsize::new(0)),
        };
        let (stop, shutdown) = watch::channel(false);
        let task = tokio::spawn(conducir(
            Box::new(script),
            Duration::from_secs(10),
            Arc::clone(&beat),
            shutdown,
        ));

        // Act
        tokio::time::sleep(Duration::from_secs(1)).await;
        let after_failure = beat.esta_fresco(Instant::now());
        tokio::time::sleep(Duration::from_secs(10)).await;
        let after_success = beat.esta_fresco(Instant::now());

        // Assert
        assert!(!after_failure);
        assert!(after_success);
        let _ = stop.send(true);
        let _ = task.await;
    }

    #[test]
    fn should_not_be_fresh_before_the_first_pass() {
        assert!(!heartbeat().esta_fresco(Instant::now()));
    }

    #[test]
    fn should_stay_fresh_within_the_interval_and_the_grace_and_go_stale_after() {
        let beat = heartbeat();
        let at = Instant::now();
        beat.beat_at(at);

        let limit = Duration::from_millis(500) + GRACIA_ATASCO;
        assert!(beat.esta_fresco(at + limit));
        assert!(!beat.esta_fresco(at + limit + Duration::from_millis(1)));
    }
}
