//! The worker process: the relay and every consumer as loops, the health probes and graceful
//! shutdown (ADR-0018 § 5, ADR-0008).

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use poem::{Route, Server, listener::TcpListener};
use puerto_core::events::EventHandler;
use puerto_core::events::transport::{DeadLetterSink, EventPublisher, EventSubscription, Outbox};
use puerto_web::app::{SHUTDOWN_DEADLINE, init_tracing, shutdown_signal};
use puerto_web::{Error, health};
use tokio::sync::watch;
use tokio::time::Instant;

use crate::WorkerConfig;
use crate::consumer::Consumer;
use crate::driver::{Heartbeat, Pass, drive};
use crate::relay::Relay;

/// What the worker runs, built by the project's `generated/registry.rs`: the relay and one
/// consumer per `[[consumer]]`, each set up from the [`WorkerConfig`].
pub struct Loops {
    config: WorkerConfig,
    loops: Vec<(Box<dyn Pass>, Duration)>,
}

impl Loops {
    pub fn new(config: &WorkerConfig) -> Self {
        Self {
            config: config.clone(),
            loops: Vec::new(),
        }
    }

    /// The relay from `outbox` to `publisher`, on `RELAY_BATCH_SIZE` / `RELAY_POLL_INTERVAL_MS`.
    pub fn relay<O, P>(mut self, outbox: O, publisher: P) -> Self
    where
        O: Outbox + 'static,
        P: EventPublisher + 'static,
    {
        let relay = Relay::new(outbox, publisher).with_batch_size(self.config.relay_batch_size);
        self.loops
            .push((Box::new(relay), self.config.relay_poll_interval));
        self
    }

    /// One consumer, on `MAX_DELIVERIES` / `RETRY_MAX_DELAY_MS` / `CONSUMER_POLL_INTERVAL_MS`.
    pub fn consumer<S>(
        mut self,
        subscription: S,
        handler: impl EventHandler + 'static,
        dead_letters: Arc<dyn DeadLetterSink>,
    ) -> Self
    where
        S: EventSubscription + 'static,
    {
        let consumer = Consumer::new(subscription, handler, dead_letters)
            .with_max_deliveries(self.config.max_deliveries)
            .with_max_retry_delay(self.config.max_retry_delay);
        self.loops
            .push((Box::new(consumer), self.config.consumer_poll_interval));
        self
    }

    /// The loops' names, in the order they were added: `relay`, `consumer <name>`, …
    pub fn names(&self) -> Vec<String> {
        self.loops.iter().map(|(work, _)| work.name()).collect()
    }
}

/// The running worker. `main.rs` is
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), puerto_events::Error> {
///     let config = puerto_events::WorkerConfig::from_env()?;
///     puerto_events::Worker::new(config).run(generated::registry::build_worker).await
/// }
/// ```
pub struct Worker {
    config: WorkerConfig,
}

impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        Self { config }
    }

    /// Sets up tracing (`RUST_LOG`), builds the loops with `build` — the project's
    /// `generated::registry::build_worker` — and runs them, serving `/health` and `/health/ready`
    /// on `WORKER_PORT`, until Ctrl-C or SIGTERM. The loops then finish the pass they are in,
    /// for at most `SHUTDOWN_DEADLINE`.
    pub async fn run<F, Fut>(self, build: F) -> Result<(), Error>
    where
        F: FnOnce(WorkerConfig) -> Fut,
        Fut: Future<Output = Loops>,
    {
        init_tracing();
        let loops = build(self.config.clone()).await;
        self.run_until(loops, shutdown_signal()).await
    }

    async fn run_until(
        self,
        loops: Loops,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), Error> {
        let (stop, shutdown) = watch::channel(false);
        let stop = Arc::new(stop);
        let mut heartbeats = Vec::new();
        let mut tasks = Vec::new();
        for (work, interval) in loops.loops {
            let heartbeat = Arc::new(Heartbeat::new(work.name(), interval));
            heartbeats.push(Arc::clone(&heartbeat));
            tasks.push(tokio::spawn(drive(
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
                health::mount(Route::new(), ready),
                async move {
                    signal.await;
                    let _ = stopping.send(true);
                },
                Some(SHUTDOWN_DEADLINE),
            )
            .await;

        // Also when the server never started (the port is taken): the loops stop with it.
        let _ = stop.send(true);
        let drained = tokio::time::timeout(SHUTDOWN_DEADLINE, async {
            for task in tasks {
                let _ = task.await;
            }
        })
        .await;
        if drained.is_err() {
            tracing::warn!("worker: a loop did not finish its pass before the shutdown deadline");
        }
        served?;
        Ok(())
    }
}

/// Ready while every loop has finished a good pass recently (`Heartbeat::is_fresh`).
fn readiness(heartbeats: Vec<Arc<Heartbeat>>) -> health::ReadinessProbe {
    let heartbeats = Arc::new(heartbeats);
    health::probe(move || {
        let heartbeats = Arc::clone(&heartbeats);
        async move { stale(&heartbeats, Instant::now()).is_empty() }
    })
}

/// The loops that have not finished a good pass recently, each one logged.
fn stale(heartbeats: &[Arc<Heartbeat>], now: Instant) -> Vec<&str> {
    heartbeats
        .iter()
        .filter(|beat| !beat.is_fresh(now))
        .map(|beat| beat.name())
        .inspect(|name| tracing::warn!(name, "worker: not ready"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FakeOutbox, FakePublisher, FakeSink, FakeSubscription, Scripted};

    #[test]
    fn should_build_the_relay_and_each_consumer_when_registered() {
        // Arrange
        let config = WorkerConfig::default();

        // Act
        let loops = Loops::new(&config)
            .relay(FakeOutbox::default(), FakePublisher::default())
            .consumer(
                FakeSubscription::default(),
                Scripted::Ack,
                Arc::new(FakeSink::default()),
            );

        // Assert
        assert_eq!(loops.names(), ["relay", "consumer notify_new_product"]);
    }

    #[test]
    fn should_run_each_loop_at_its_configured_interval() {
        // Arrange
        let config = WorkerConfig {
            relay_poll_interval: Duration::from_millis(50),
            consumer_poll_interval: Duration::from_millis(75),
            ..WorkerConfig::default()
        };

        // Act
        let loops = Loops::new(&config)
            .relay(FakeOutbox::default(), FakePublisher::default())
            .consumer(
                FakeSubscription::default(),
                Scripted::Ack,
                Arc::new(FakeSink::default()),
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
        let relay = Arc::new(Heartbeat::new("relay".into(), Duration::from_millis(500)));
        let consumer = Arc::new(Heartbeat::new(
            "consumer".into(),
            Duration::from_millis(500),
        ));
        let heartbeats = vec![Arc::clone(&relay), Arc::clone(&consumer)];
        relay.beat_at(now);

        // Act
        let one_fresh = stale(&heartbeats, now);
        consumer.beat_at(now);
        let both_fresh = stale(&heartbeats, now);

        // Assert
        assert_eq!(one_fresh, ["consumer"]);
        assert!(both_fresh.is_empty());
    }

    #[tokio::test]
    async fn should_drain_the_loops_and_return_when_the_shutdown_signal_arrives() {
        // Arrange
        let outbox = FakeOutbox::with(&[crate::test_support::envelope("product_created")]);
        let publisher = FakePublisher::accepting(usize::MAX);
        let config = WorkerConfig::default().with_port(0);
        let loops = Loops::new(&config).relay(outbox.clone(), publisher.clone());
        let signal = async { tokio::time::sleep(Duration::from_millis(200)).await };

        // Act
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            Worker::new(config).run_until(loops, signal),
        )
        .await;

        // Assert
        assert!(matches!(result, Ok(Ok(()))), "{result:?}");
        assert!(outbox.unpublished().is_empty());
    }

    #[tokio::test]
    async fn should_stop_the_loops_and_fail_when_the_port_is_taken() {
        // Arrange
        let taken = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let config = WorkerConfig::default().with_port(port);
        let loops = Loops::new(&config).relay(FakeOutbox::default(), FakePublisher::default());

        // Act
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            Worker::new(config).run_until(loops, std::future::pending()),
        )
        .await;

        // Assert
        assert!(matches!(result, Ok(Err(Error::Server(_)))), "{result:?}");
    }
}
