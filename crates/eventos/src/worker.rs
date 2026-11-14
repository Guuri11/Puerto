//! The worker process: the relay and every consumer as loops, the health probes and graceful
//! shutdown (ADR-0018 § 5, ADR-0008).

use std::convert::Infallible;
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use poem::{Route, Server, listener::TcpListener};
use puerto_nucleo::eventos::ManejadorEvento;
use puerto_nucleo::eventos::transporte::{
    DestinoDescartados, Outbox, PublicadorEventos, SuscripcionEventos,
};
use puerto_web::app::{PLAZO_APAGADO, iniciar_tracing, senal_apagado};
use puerto_web::{Error, health};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::ConfigWorker;
use crate::conductor::{Latido, Pasada, conducir};
use crate::consumidor::Consumidor;
use crate::relay::Relay;

/// What the worker runs, built by the project's `generated/registry.rs`: the relay and one
/// consumer per `[[consumidor]]`, each set up from the [`ConfigWorker`].
pub struct Bucles {
    config: ConfigWorker,
    loops: Vec<(Box<dyn Pasada>, Duration)>,
}

impl Bucles {
    pub fn new(config: &ConfigWorker) -> Self {
        Self {
            config: config.clone(),
            loops: Vec::new(),
        }
    }

    /// The relay from `outbox` to `publicador`, on `RELAY_BATCH_SIZE` / `RELAY_POLL_INTERVAL_MS`.
    pub fn relay<O, P>(mut self, outbox: O, publicador: P) -> Self
    where
        O: Outbox + 'static,
        P: PublicadorEventos + 'static,
    {
        let relay = Relay::new(outbox, publicador).con_tamano_lote(self.config.tamano_lote_relay);
        self.loops
            .push((Box::new(relay), self.config.intervalo_sondeo_relay));
        self
    }

    /// One consumer, on `MAX_DELIVERIES` / `RETRY_MAX_DELAY_MS` / `CONSUMER_POLL_INTERVAL_MS`.
    pub fn consumidor<S>(
        mut self,
        suscripcion: S,
        handler: impl ManejadorEvento + 'static,
        dead_letters: Arc<dyn DestinoDescartados>,
    ) -> Self
    where
        S: SuscripcionEventos + 'static,
    {
        let consumidor = Consumidor::new(suscripcion, handler, dead_letters)
            .con_entregas_maximas(self.config.max_deliveries)
            .con_espera_maxima_reintento(self.config.espera_maxima_reintento);
        self.loops.push((
            Box::new(consumidor),
            self.config.intervalo_sondeo_consumidor,
        ));
        self
    }

    /// The loops' names, in the order they were added: `relay`, `consumidor <name>`, …
    pub fn nombres(&self) -> Vec<String> {
        self.loops.iter().map(|(work, _)| work.name()).collect()
    }
}

/// The running worker. `main.rs` is
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), puerto_events::Error> {
///     let config = puerto_events::ConfigWorker::from_env()?;
///     puerto_events::Worker::new(config).try_run(generated::registry::build_worker).await
/// }
/// ```
pub struct Worker {
    config: ConfigWorker,
}

impl Worker {
    pub fn new(config: ConfigWorker) -> Self {
        Self { config }
    }

    /// Sets up tracing (`RUST_LOG`), builds the loops with `build` and runs them, serving
    /// `/health` and `/health/ready` on `WORKER_PORT`, until Ctrl-C or SIGTERM. The loops then
    /// finish the pass they are in, for at most `SHUTDOWN_DEADLINE`. For a `build` that cannot
    /// fail; see [`Worker::intentar_ejecutar`].
    pub async fn ejecutar<F, Fut>(self, build: F) -> Result<(), Error>
    where
        F: FnOnce(ConfigWorker) -> Fut,
        Fut: Future<Output = Bucles>,
    {
        self.intentar_ejecutar(|config| async move { Ok::<_, Infallible>(build(config).await) })
            .await
    }

    /// [`Worker::ejecutar`] with a `build` that can fail — the project's
    /// `generated::registry::build_worker`, which connects to the database (and the broker). Its
    /// error is logged and returned as [`Error::Startup`]: `main` prints it and exits 1.
    pub async fn intentar_ejecutar<F, Fut, X>(self, build: F) -> Result<(), Error>
    where
        F: FnOnce(ConfigWorker) -> Fut,
        Fut: Future<Output = Result<Bucles, X>>,
        X: Display,
    {
        iniciar_tracing();
        let loops = build(self.config.clone()).await.map_err(|e| {
            let message = e.to_string();
            tracing::error!(error = %message, "el arranque falló");
            Error::Arranque(message)
        })?;
        self.run_until(loops, senal_apagado()).await
    }

    async fn run_until(
        self,
        loops: Bucles,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), Error> {
        let (stop, shutdown) = watch::channel(false);
        let stop = Arc::new(stop);
        let mut heartbeats = Vec::new();
        let mut tasks = Vec::new();
        for (work, interval) in loops.loops {
            let heartbeat = Arc::new(Latido::new(work.name(), interval));
            heartbeats.push(Arc::clone(&heartbeat));
            tasks.push(tokio::spawn(conducir(
                work,
                interval,
                heartbeat,
                shutdown.clone(),
            )));
        }

        let ready = readiness(heartbeats);
        let stopping = Arc::clone(&stop);
        let served = Server::new(TcpListener::bind(format!("0.0.0.0:{}", self.config.port)))
            .run_with_graceful_shutdown(
                health::montar(Route::new(), ready),
                async move {
                    signal.await;
                    let _ = stopping.send(true);
                },
                Some(PLAZO_APAGADO),
            )
            .await;

        // Also when the server never started (the port is taken): the loops stop with it.
        let _ = stop.send(true);
        let drained = tokio::time::timeout(PLAZO_APAGADO, async {
            for task in tasks {
                let _ = task.await;
            }
        })
        .await;
        if drained.is_err() {
            tracing::warn!("worker: un bucle no terminó su pasada antes del plazo de apagado");
        }
        served?;
        Ok(())
    }
}

/// Ready while every loop has finished a good pass recently (`Latido::esta_fresco`).
fn readiness(heartbeats: Vec<Arc<Latido>>) -> health::SondaDisponibilidad {
    let heartbeats = Arc::new(heartbeats);
    health::sondear(move || {
        let heartbeats = Arc::clone(&heartbeats);
        async move { stale(&heartbeats, Instant::now()).is_empty() }
    })
}

/// The loops that have not finished a good pass recently, each one logged.
fn stale(heartbeats: &[Arc<Latido>], now: Instant) -> Vec<&str> {
    heartbeats
        .iter()
        .filter(|beat| !beat.esta_fresco(now))
        .map(|beat| beat.name())
        .inspect(|name| tracing::warn!(name, "worker: not ready"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pruebas::{DestinoFalso, Guion, OutboxFalso, PublicadorFalso, SuscripcionFalsa};

    #[tokio::test]
    async fn should_stop_with_the_build_error_when_the_loops_cannot_be_built() {
        // Arrange
        let worker = Worker::new(ConfigWorker::default());

        // Act
        let result = worker
            .intentar_ejecutar(|_| async { Err::<Bucles, _>("the database is down") })
            .await;

        // Assert: `main` prints the error with `Debug` and exits 1.
        let error = result.unwrap_err();
        assert!(matches!(error, Error::Arranque(_)), "{error:?}");
        assert_eq!(format!("{error:?}"), "the database is down");
    }

    #[test]
    fn should_build_the_relay_and_each_consumer_when_registered() {
        // Arrange
        let config = ConfigWorker::default();

        // Act
        let loops = Bucles::new(&config)
            .relay(OutboxFalso::default(), PublicadorFalso::default())
            .consumidor(
                SuscripcionFalsa::default(),
                Guion::Ack,
                Arc::new(DestinoFalso::default()),
            );

        // Assert
        assert_eq!(loops.nombres(), ["relay", "consumidor notify_new_product"]);
    }

    #[test]
    fn should_run_each_loop_at_its_configured_interval() {
        // Arrange
        let config = ConfigWorker {
            intervalo_sondeo_relay: Duration::from_millis(50),
            intervalo_sondeo_consumidor: Duration::from_millis(75),
            ..ConfigWorker::default()
        };

        // Act
        let loops = Bucles::new(&config)
            .relay(OutboxFalso::default(), PublicadorFalso::default())
            .consumidor(
                SuscripcionFalsa::default(),
                Guion::Ack,
                Arc::new(DestinoFalso::default()),
            );

        // Assert
        let intervals: Vec<_> = loops.loops.iter().map(|(_, i)| *i).collect();
        assert_eq!(
            intervals,
            [Duration::from_millis(50), Duration::from_millis(75)]
        );
    }

    #[test]
    fn should_name_the_stale_loops_until_every_loop_is_fresh() {
        // Arrange
        let now = Instant::now();
        let relay = Arc::new(Latido::new("relay".into(), Duration::from_millis(500)));
        let consumidor = Arc::new(Latido::new("consumer".into(), Duration::from_millis(500)));
        let heartbeats = vec![Arc::clone(&relay), Arc::clone(&consumidor)];
        relay.beat_at(now);

        // Act
        let one_fresh = stale(&heartbeats, now);
        consumidor.beat_at(now);
        let both_fresh = stale(&heartbeats, now);

        // Assert
        assert_eq!(one_fresh, ["consumer"]);
        assert!(both_fresh.is_empty());
    }

    #[tokio::test]
    async fn should_drain_the_loops_and_return_when_the_shutdown_signal_arrives() {
        // Arrange
        let outbox = OutboxFalso::with(&[crate::pruebas::envelope("product_created")]);
        let publicador = PublicadorFalso::aceptando(usize::MAX);
        let config = ConfigWorker::default().con_puerto(0);
        let loops = Bucles::new(&config).relay(outbox.clone(), publicador.clone());
        let signal = async { tokio::time::sleep(Duration::from_millis(200)).await };

        // Act
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            Worker::new(config).run_until(loops, signal),
        )
        .await;

        // Assert
        assert!(matches!(result, Ok(Ok(()))), "{result:?}");
        assert!(outbox.sin_publicar().is_empty());
    }

    #[tokio::test]
    async fn should_stop_the_loops_and_fail_when_the_port_is_taken() {
        // Arrange
        let taken = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let config = ConfigWorker::default().con_puerto(port);
        let loops = Bucles::new(&config).relay(OutboxFalso::default(), PublicadorFalso::default());

        // Act
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            Worker::new(config).run_until(loops, std::future::pending()),
        )
        .await;

        // Assert
        assert!(matches!(result, Ok(Err(Error::Servidor(_)))), "{result:?}");
    }
}
