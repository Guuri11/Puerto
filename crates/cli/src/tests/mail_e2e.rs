// Phase 14.5 end to end (`make test/db`, `make test/kafka`): the generated API and worker as real
// processes, sending the mails of `EVENTS_MAIL` to a real SMTP server (Mailpit) — a POST sends one
// mail, a redelivered event does not send it again, a restart does not either, and a mail that
// finds the server down is retried until it is back. The worker reaches Mailpit through a proxy
// the test switches off and on.
use super::*;

use std::io::{Read as _, Write as _};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::compile_matrix::{EVENTS_MAIL, EVENTS_MAIL_KAFKA, Scenario};
use super::worker_e2e::{
    Project, build_project, create_product, free_port, http, logged, wait_until,
};

/// The consumer the worker runs `product_live` as.
const CONSUMER: &str = "mail_product_live";

/// A TCP proxy in front of Mailpit's SMTP port: `down()` refuses new connections and cuts the
/// open ones (the mailer's pool keeps them), `up()` lets them through again.
struct SmtpSwitch {
    port: u16,
    up: Arc<AtomicBool>,
    open: Arc<Mutex<Vec<TcpStream>>>,
}

impl SmtpSwitch {
    fn start(upstream: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("no free port");
        let port = listener.local_addr().unwrap().port();
        let up = Arc::new(AtomicBool::new(true));
        let open: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let (accept_up, accept_open) = (up.clone(), open.clone());
        std::thread::spawn(move || {
            for client in listener.incoming().flatten() {
                if !accept_up.load(Ordering::SeqCst) {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                }
                let Ok(server) = TcpStream::connect(&upstream) else {
                    continue;
                };
                let mut open = accept_open.lock().unwrap();
                open.push(client.try_clone().unwrap());
                open.push(server.try_clone().unwrap());
                pipe(client.try_clone().unwrap(), server.try_clone().unwrap());
                pipe(server, client);
            }
        });
        Self { port, up, open }
    }

    fn url(&self) -> String {
        format!("smtp://127.0.0.1:{}", self.port)
    }

    fn down(&self) {
        self.up.store(false, Ordering::SeqCst);
        for stream in self.open.lock().unwrap().drain(..) {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    fn up(&self) {
        self.up.store(true, Ordering::SeqCst);
    }
}

fn pipe(mut from: TcpStream, mut to: TcpStream) {
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut from, &mut to);
        let _ = to.shutdown(Shutdown::Write);
    });
}

/// Where the test reaches Mailpit: its SMTP `host:port` and its API port.
fn mailpit() -> Option<(String, u16)> {
    let smtp = std::env::var("PUERTO_TEST_SMTP_URL").ok()?;
    let api = std::env::var("PUERTO_TEST_MAILPIT_API").ok()?;
    let smtp = smtp.trim_start_matches("smtp://").to_string();
    let port = api.rsplit(':').next()?.trim_end_matches('/').parse().ok()?;
    Some((smtp, port))
}

/// Every message Mailpit holds, over HTTP/1.0 so the body is never chunked.
fn mailpit_messages(api_port: u16) -> Vec<serde_json::Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", api_port)).expect("Mailpit is not up");
    stream
        .write_all(b"GET /api/v1/messages?limit=1000 HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let body = response.split_once("\r\n\r\n").map_or("", |(_, b)| b);
    let json: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    json["messages"].as_array().cloned().unwrap_or_default()
}

/// The subjects of the messages sent to `to`.
fn sent_to(api_port: u16, to: &str) -> Vec<String> {
    mailpit_messages(api_port)
        .iter()
        .filter(|m| {
            m["To"]
                .as_array()
                .is_some_and(|tos| tos.iter().any(|t| t["Address"] == to))
        })
        .map(|m| m["Subject"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// An address no earlier run used: Mailpit keeps what it received while it runs.
fn address(tag: &str) -> String {
    format!("{tag}-{}@e2e.test", std::process::id())
}

/// Hands the `db` transport's deliveries of `consumer` back to the worker, as a redelivery after
/// a lost acknowledgement would.
fn redeliver(database_url: &str, consumer: &str) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let pool = sqlx::PgPool::connect(database_url).await.unwrap();
            let handed_back = sqlx::query(
                "UPDATE event_deliveries SET status = 'pending', next_attempt_at = now() \
                 WHERE consumer = $1",
            )
            .bind(consumer)
            .execute(&pool)
            .await
            .unwrap()
            .rows_affected();
            assert!(handed_back > 0, "no delivery of {consumer} to hand back");
            pool.close().await;
        });
}

/// `scenario` built with the mail settings: the worker sends through `switch`, retries every
/// 100 ms, and gives up only after 100 deliveries — longer than the server is ever down here.
fn build_mail_project(
    scenario: &Scenario,
    name: &str,
    database: &str,
    switch: &SmtpSwitch,
    extra_env: &[(&str, String)],
) -> Option<Project> {
    let mut env = vec![
        ("SMTP_URL", switch.url()),
        ("MAIL_FROM", "Shop <no-reply@e2e.test>".to_string()),
        ("MAX_DELIVERIES", "100".to_string()),
        ("RETRY_MAX_DELAY_MS", "100".to_string()),
    ];
    env.extend(extra_env.iter().cloned());
    build_project(scenario, name, database, &[], &env)
}

fn database_url(project: &Project) -> String {
    project
        .env
        .iter()
        .find(|(k, _)| k == "DATABASE_URL")
        .map(|(_, v)| v.clone())
        .expect("no DATABASE_URL")
}

/// What both transports must do; `redelivery` hands a sent mail's event back to the worker.
fn assert_mails_are_sent_once(
    project: &Project,
    api_binary: &str,
    switch: &SmtpSwitch,
    api_port_mailpit: u16,
    redelivery: Option<&dyn Fn()>,
) {
    let (api_port, worker_port) = (free_port(), free_port());
    let api_log = project.dir.join("api.log");
    let worker_log = project.dir.join("worker.log");
    let api = project.spawn(
        api_binary,
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

    // A POST sends the mail, rendered from its templates, to the product's sku.
    let first = address("first");
    create_product(api_port, &first);
    wait_until("the first mail to reach Mailpit", || {
        !sent_to(api_port_mailpit, &first).is_empty()
    });
    assert_eq!(
        sent_to(api_port_mailpit, &first),
        ["Product live, Desk lamp"]
    );

    // The same event delivered again finds its row in `sent_mails`: no second mail.
    if let Some(redeliver) = redelivery {
        redeliver();
        wait_until("the redelivered event to be skipped", || {
            logged(
                &worker_log,
                &["mail already sent for this event", "product_live"],
            )
        });
        assert_eq!(sent_to(api_port_mailpit, &first).len(), 1);
    }

    // A mail that finds the server down is retried, and sent once it is back.
    switch.down();
    let delayed = address("delayed");
    create_product(api_port, &delayed);
    wait_until("the mail to be retried", || {
        logged(&worker_log, &["mail: not sent, retrying", "product_live"])
    });
    assert!(sent_to(api_port_mailpit, &delayed).is_empty());
    switch.up();
    wait_until("the delayed mail to reach Mailpit", || {
        !sent_to(api_port_mailpit, &delayed).is_empty()
    });
    assert_eq!(sent_to(api_port_mailpit, &delayed).len(), 1);

    // A restarted worker sends what was written while it was down, and nothing it sent before.
    drop(worker);
    let after_restart = address("after-restart");
    create_product(api_port, &after_restart);
    let restarted_log = project.dir.join("worker-restarted.log");
    let restarted = project.spawn("worker", &restarted_log, &worker_env);
    wait_until("the restarted worker to send the next mail", || {
        !sent_to(api_port_mailpit, &after_restart).is_empty()
    });
    assert_eq!(sent_to(api_port_mailpit, &first).len(), 1);
    assert_eq!(sent_to(api_port_mailpit, &delayed).len(), 1);

    drop(restarted);
    drop(api);
}

#[test]
#[ignore = "slow: needs Postgres and Mailpit — run via `make test/db`"]
fn postgres_mail_is_sent_once_per_event_and_retried_while_smtp_is_down() {
    let Some((smtp, api_port_mailpit)) = mailpit() else {
        eprintln!(
            "PUERTO_TEST_SMTP_URL / PUERTO_TEST_MAILPIT_API not set — skipping (run `make test/db`)"
        );
        return;
    };
    let switch = SmtpSwitch::start(smtp);
    let Some(project) = build_mail_project(
        &EVENTS_MAIL,
        "e2e-mail",
        "puerto_matrix_mail_e2e",
        &switch,
        &[],
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let url = database_url(&project);

    assert_mails_are_sent_once(
        &project,
        "e2e-mail",
        &switch,
        api_port_mailpit,
        Some(&|| redeliver(&url, CONSUMER)),
    );
    cleanup(&project.dir);
}

#[test]
#[ignore = "slow: needs Kafka, Postgres and Mailpit — run via `make test/kafka`"]
fn kafka_mail_is_sent_once_per_event_and_retried_while_smtp_is_down() {
    let Some((smtp, api_port_mailpit)) = mailpit() else {
        eprintln!(
            "PUERTO_TEST_SMTP_URL / PUERTO_TEST_MAILPIT_API not set — skipping (run `make test/kafka`)"
        );
        return;
    };
    let Some(brokers) = std::env::var("PUERTO_TEST_KAFKA_BROKERS").ok() else {
        eprintln!("PUERTO_TEST_KAFKA_BROKERS not set — skipping (run `make test/kafka`)");
        return;
    };
    let switch = SmtpSwitch::start(smtp);
    let prefix = format!("e2email{}", std::process::id());
    let Some(project) = build_mail_project(
        &EVENTS_MAIL_KAFKA,
        "e2e-mail-kafka",
        "puerto_matrix_mail_kafka_e2e",
        &switch,
        &[("KAFKA_BROKERS", brokers), ("KAFKA_TOPIC_PREFIX", prefix)],
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/kafka`)");
        return;
    };

    // No redelivery step: on Kafka the offset, not a row, says what was delivered.
    assert_mails_are_sent_once(&project, "e2e-mail-kafka", &switch, api_port_mailpit, None);
    cleanup(&project.dir);
}
