// Phase 13.4 end to end (`make test/db`): the generated API and worker as real processes against
// Postgres — an event written by a POST reaches its consumer, a consumer that always retries ends
// in `dead_letters`, and an event written while the worker is down is delivered after a restart.
// Phase 13.5 (`make test/kafka`): the same on the `kafka` transport, plus the order of one
// aggregate's events and a record that is not an event.
use super::*;

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::compile_matrix::{
    EVENTS_DB, EVENTS_KAFKA, Scenario, apply_scenario, patch_runtime_crates, scenario_workspace,
    test_database_server,
};

/// How long any one expectation may take: a pass every 100 ms, so this is generous.
const WAIT: Duration = Duration::from_secs(60);

/// A consumer that never succeeds: every delivery of it is a `Retry`.
const ALWAYS_RETRY: &str = r#"use async_trait::async_trait;
use puerto_core::events::{EventEnvelope, EventHandler, HandlerOutcome};

pub struct AlwaysRetry;

#[async_trait]
impl EventHandler for AlwaysRetry {
    async fn handle(&self, _event: &EventEnvelope) -> HandlerOutcome {
        HandlerOutcome::Retry
    }
}
"#;

/// A child process killed when the test ends, whatever happens.
pub(super) struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .expect("no free port")
}

/// One HTTP/1.1 request: `(status, body)`, or `None` when nothing answers yet.
pub(super) fn http(port: u16, method: &str, path: &str, body: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let status = response.split(' ').nth(1)?.parse().ok()?;
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    Some((status, body))
}

pub(super) fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        assert!(start.elapsed() < WAIT, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(super) fn log_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect()
}

pub(super) fn logged(path: &Path, needles: &[&str]) -> bool {
    log_lines(path)
        .iter()
        .any(|line| needles.iter().all(|n| line.contains(n)))
}

fn product_body(sku: &str) -> String {
    format!(
        r#"{{"name":"Desk lamp","sku":"{sku}","price":2500,"status":"Draft","tags":[],"meta":{{}}}}"#
    )
}

/// Creates a product and returns its id.
pub(super) fn create_product(port: u16, sku: &str) -> String {
    let body = product_body(sku);
    let (status, response) =
        http(port, "POST", "/api/products", &body).expect("the API did not answer");
    assert_eq!(status, 201, "POST /api/products: {response}");
    let at = response.find("\"id\":\"").expect("no id in the response") + 6;
    response[at..at + 36].to_string()
}

pub(super) struct Project {
    pub(super) dir: PathBuf,
    pub(super) output: PathBuf,
    pub(super) env: Vec<(String, String)>,
}

impl Project {
    pub(super) fn spawn(&self, binary: &str, log: &Path, extra: &[(&str, String)]) -> Running {
        let file = fs::File::create(log).unwrap();
        let child = Command::new(self.dir.join("target/debug").join(binary))
            .current_dir(&self.output)
            .envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .envs(extra.iter().map(|(k, v)| (*k, v.as_str())))
            .stdout(Stdio::from(file.try_clone().unwrap()))
            .stderr(Stdio::from(file))
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {binary}: {e}"));
        Running(child)
    }
}

/// `scenario` plus a consumer that always retries (and `extra_consumers`), migrated and built.
pub(super) fn build_project(
    scenario: &Scenario,
    name: &str,
    database: &str,
    extra_consumers: &[(&str, &[&str])],
    extra_env: &[(&str, String)],
) -> Option<Project> {
    let server = test_database_server()?;
    let dir = scenario_workspace(&format!("{}_e2e", scenario.name));
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    apply_scenario(scenario, &output);
    for (consumer, events) in extra_consumers {
        let events: Vec<String> = events.iter().map(|e| e.to_string()).collect();
        scaffold::run_generate_consumer(consumer, &events, &output).unwrap();
    }
    scaffold::run_generate_consumer("always_retry", &["product_created".into()], &output).unwrap();
    fs::write(
        output.join("worker/src/consumers/always_retry.rs"),
        ALWAYS_RETRY,
    )
    .unwrap();
    patch_runtime_crates(&output);

    let database_url = format!("{server}/{database}");
    for args in [
        &["database", "drop", "-y"][..],
        &["database", "create"][..],
        &["migrate", "run", "--source", "infrastructure/migrations"][..],
    ] {
        let status = Command::new("sqlx")
            .args(args)
            .env("DATABASE_URL", &database_url)
            .current_dir(&output)
            .status()
            .expect("sqlx CLI not found — cargo install sqlx-cli");
        assert!(status.success(), "`sqlx {}` failed", args.join(" "));
    }
    let env: Vec<(String, String)> = [
        ("DATABASE_URL", database_url.as_str()),
        ("SQLX_OFFLINE", "false"),
        ("RUST_LOG", "info"),
        ("NO_COLOR", "1"),
        ("PUERTO_BANNER", "false"),
        ("RELAY_POLL_INTERVAL_MS", "100"),
        ("CONSUMER_POLL_INTERVAL_MS", "100"),
        ("MAX_DELIVERIES", "5"),
        // The policy waits 1 s, 5 s, 15 s, 60 s between deliveries: no wait here.
        ("RETRY_MAX_DELAY_MS", "0"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .chain(extra_env.iter().map(|(k, v)| (k.to_string(), v.clone())))
    .collect();

    let built = Command::new("cargo")
        .args(["build", "--workspace"])
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .current_dir(&output)
        .output()
        .expect("failed to run cargo");
    if !built.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&built.stderr));
        panic!("the events project did not build");
    }
    Some(Project { dir, output, env })
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_events_worker_delivers_dead_letters_and_survives_a_restart() {
    let Some(project) = build_project(
        &EVENTS_DB,
        "e2e-events",
        "puerto_matrix_events_e2e",
        &[],
        &[],
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let (api_port, worker_port) = (free_port(), free_port());
    let api_log = project.dir.join("api.log");
    let worker_log = project.dir.join("worker.log");
    let api = project.spawn(
        "e2e-events",
        &api_log,
        &[("SERVICE_PORT", api_port.to_string())],
    );
    let worker_env = [("WORKER_PORT", worker_port.to_string())];
    let worker = project.spawn("worker", &worker_log, &worker_env);
    wait_until("the API to be ready", || {
        http(api_port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });
    wait_until("the worker to be ready", || {
        http(worker_port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });

    // A POST reaches the generated consumer.
    let first = create_product(api_port, "E2E-1");
    wait_until("notify_new_product to see product_created", || {
        logged(
            &worker_log,
            &[
                "notify_new_product: product_created",
                &format!("product_id={first}"),
            ],
        )
    });

    // A consumer that always retries is dead-lettered on its fifth delivery.
    wait_until("always_retry to be dead-lettered", || {
        logged(
            &worker_log,
            &["dead_letters: recorded", "consumer=always_retry"],
        )
    });
    let retries = log_lines(&worker_log)
        .iter()
        .filter(|l| l.contains("consumer: retry") && l.contains("always_retry"))
        .count();
    assert_eq!(retries, 4, "deliveries 1–4 retry, the 5th dead-letters");

    // An event written while the worker is down stays in the outbox until it is back.
    drop(worker);
    let second = create_product(api_port, "E2E-2");
    std::thread::sleep(Duration::from_millis(500));
    assert!(!logged(&worker_log, &[&format!("product_id={second}")]));
    let restarted_log = project.dir.join("worker-restarted.log");
    let restarted = project.spawn("worker", &restarted_log, &worker_env);
    wait_until("the restarted worker to deliver the second product", || {
        logged(
            &restarted_log,
            &[
                "notify_new_product: product_created",
                &format!("product_id={second}"),
            ],
        )
    });
    // The first one was settled before the restart: it is not delivered again.
    assert!(!logged(&restarted_log, &[&format!("product_id={first}")]));

    drop(restarted);
    drop(api);
    cleanup(&project.dir);
}

/// Writes `payload` to `topic` as a bare record: no envelope headers.
fn produce_raw(brokers: &str, topic: &str, payload: &str) {
    use rdkafka::producer::{BaseProducer, BaseRecord, Producer};
    let producer: BaseProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .create()
        .expect("cannot create a Kafka producer");
    producer
        .send(BaseRecord::<str, str>::to(topic).payload(payload))
        .map_err(|(err, _)| err)
        .expect("cannot queue the record");
    producer
        .flush(Duration::from_secs(10))
        .expect("the record was not written");
}

/// The index of the first log line holding every needle.
fn line_of(path: &Path, needles: &[&str]) -> Option<usize> {
    log_lines(path)
        .iter()
        .position(|line| needles.iter().all(|n| line.contains(n)))
}

#[test]
#[ignore = "slow: needs Kafka and Postgres — run via `make test/kafka`"]
fn kafka_events_worker_delivers_in_order_dead_letters_and_survives_a_restart() {
    let Some(brokers) = std::env::var("PUERTO_TEST_KAFKA_BROKERS").ok() else {
        eprintln!("PUERTO_TEST_KAFKA_BROKERS not set — skipping (run `make test/kafka`)");
        return;
    };
    // Topics and groups of their own: a rerun never reads what an earlier one left.
    let prefix = format!("e2e{}", std::process::id());
    let Some(project) = build_project(
        &EVENTS_KAFKA,
        "e2e-kafka",
        "puerto_matrix_kafka_e2e",
        &[("audit", &["product_created", "product_updated"])],
        &[
            ("KAFKA_BROKERS", brokers.clone()),
            ("KAFKA_TOPIC_PREFIX", prefix.clone()),
        ],
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/kafka`)");
        return;
    };
    let (api_port, worker_port) = (free_port(), free_port());
    let api_log = project.dir.join("api.log");
    let worker_log = project.dir.join("worker.log");
    let api = project.spawn(
        "e2e-kafka",
        &api_log,
        &[("SERVICE_PORT", api_port.to_string())],
    );
    let worker_env = [("WORKER_PORT", worker_port.to_string())];
    let worker = project.spawn("worker", &worker_log, &worker_env);
    wait_until("the API to be ready", || {
        http(api_port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });
    wait_until("the worker to be ready", || {
        http(worker_port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });

    // A POST reaches the generated consumer, through Kafka.
    let first = create_product(api_port, "KAFKA-1");
    wait_until("notify_new_product to see product_created", || {
        logged(
            &worker_log,
            &[
                "notify_new_product: product_created",
                &format!("product_id={first}"),
            ],
        )
    });

    // Two events of one aggregate arrive in the order they happened.
    let (status, response) = http(
        api_port,
        "PUT",
        &format!("/api/products/{first}"),
        &product_body("KAFKA-1"),
    )
    .expect("the API did not answer");
    assert_eq!(status, 200, "PUT /api/products/{first}: {response}");
    let product = format!("product_id={first}");
    wait_until("audit to see product_updated", || {
        logged(&worker_log, &["audit: product_updated", &product])
    });
    let created = line_of(&worker_log, &["audit: product_created", &product]);
    let updated = line_of(&worker_log, &["audit: product_updated", &product]);
    assert!(
        created.is_some() && created < updated,
        "audit saw product_updated before product_created"
    );

    // A consumer that always retries is retried in process, then dead-lettered.
    wait_until("always_retry to be dead-lettered", || {
        logged(
            &worker_log,
            &["dead_letters: recorded", "consumer=always_retry"],
        )
    });
    let retries = log_lines(&worker_log)
        .iter()
        .filter(|l| l.contains("consumer: retry") && l.contains("always_retry"))
        .count();
    assert_eq!(retries, 4, "deliveries 1–4 retry, the 5th dead-letters");

    // A record that is not an event is dead-lettered, and the next one is still consumed.
    produce_raw(&brokers, &format!("{prefix}.product"), "not an event");
    wait_until("the poisoned record to be dead-lettered", || {
        logged(
            &worker_log,
            &[
                "dead_letters: recorded",
                "consumer=notify_new_product",
                "event_id=None",
            ],
        )
    });
    let after_poison = create_product(api_port, "KAFKA-2");
    wait_until("notify_new_product to see the next product", || {
        logged(
            &worker_log,
            &[
                "notify_new_product: product_created",
                &format!("product_id={after_poison}"),
            ],
        )
    });

    // An event written while the worker is down (killed) is published once it is back; what was
    // committed before the restart is not delivered again.
    drop(worker);
    let third = create_product(api_port, "KAFKA-3");
    let restarted_log = project.dir.join("worker-restarted.log");
    let restarted = project.spawn("worker", &restarted_log, &worker_env);
    wait_until("the restarted worker to deliver the third product", || {
        logged(
            &restarted_log,
            &[
                "notify_new_product: product_created",
                &format!("product_id={third}"),
            ],
        )
    });
    // `first` was settled by every consumer long before the worker was killed; `after_poison`
    // may not have been (audit, always_retry), and redelivering it is at-least-once, not a bug.
    assert!(!logged(&restarted_log, &[&format!("product_id={first}")]));

    drop(restarted);
    drop(api);
    cleanup(&project.dir);
}
