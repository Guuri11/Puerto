// Phase 13.4 (ADR-0018 § 5): the `worker/` crate of an `events` project and
// `puerto generate consumer`.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A fresh `--db --events <transport>` project (no web: nothing here concerns the frontend).
fn events_project(name: &str, transport: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::apply_db_to_new_project(&output).unwrap();
    scaffold::apply_events_to_new_project(&output, transport).unwrap();
    scaffold::apply_no_web(&output).unwrap();
    (dir, output)
}

/// An events project with a CRUD `Product` that declares `product_published`.
fn product_project(name: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = events_project(name, "db");
    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Product", &output, &fields, OverwritePolicy::Ask).unwrap();
    scaffold::run_generate_event("Product", "product_published", &output).unwrap();
    (dir, output)
}

fn on(events: &[&str]) -> Vec<String> {
    events.iter().map(|e| e.to_string()).collect()
}

fn error_of(result: Result<(), Box<dyn std::error::Error>>) -> String {
    result.err().map(|e| e.to_string()).unwrap_or_default()
}

// ── puerto new ───────────────────────────────────────────────────────────

#[test]
fn new_with_db_events_writes_the_worker_and_adds_it_to_the_workspace() {
    let (dir, output) = events_project("worker_new", "db");

    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains("members = [\"business\", \"infrastructure\", \"presentation\", \"worker\"]"),
        "{root}"
    );
    assert!(root.contains("puerto-events = \""), "{root}");
    for path in [
        "worker/Cargo.toml",
        "worker/AGENTS.md",
        "worker/src/main.rs",
        "worker/src/generated.rs",
        "worker/src/generated/registry.rs",
        "worker/src/consumers.rs",
    ] {
        assert!(output.join(path).exists(), "{path} missing");
    }
    let manifest = read(&output, "worker/Cargo.toml");
    assert!(manifest.contains("puerto-events = { workspace = true }"));
    let main = read(&output, "worker/src/main.rs");
    assert!(main.contains("puerto_events::Worker::new(config)"));
    assert!(main.contains(".run(generated::registry::build_worker)"));
    cleanup(&dir);
}

#[test]
fn new_with_db_events_relays_to_nobody_until_a_consumer_is_added() {
    let (dir, output) = events_project("worker_registry_empty", "db");

    let registry = read(&output, "worker/src/generated/registry.rs");

    assert!(registry.starts_with("// AUTO-GENERATED"), "{registry}");
    assert!(
        registry.contains(
            "Loops::new(&config).relay(PgOutbox::new(pool.clone()), PgDeliveries::new(pool))"
        ),
        "{registry}"
    );
    assert!(!registry.contains(".consumer("), "{registry}");
    cleanup(&dir);
}

#[test]
fn new_with_db_events_adds_the_worker_targets_and_port() {
    let (dir, output) = events_project("worker_make", "db");

    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("\nworker: ## "), "{makefile}");
    assert!(makefile.contains("$(CARGO) run -p worker"), "{makefile}");
    assert!(makefile.contains("\nevents/dead-letters: "), "{makefile}");
    assert!(makefile.contains("FROM dead_letters"), "{makefile}");
    for env in [".env", ".env.example"] {
        assert!(read(&output, env).contains("WORKER_PORT=8081"), "{env}");
    }
    cleanup(&dir);
}

#[test]
fn new_with_db_events_tells_agents_that_the_registry_is_never_hand_edited() {
    let (dir, output) = events_project("worker_agents", "db");

    let agents = read(&output, "AGENTS.md");
    assert!(
        agents.contains("Never hand-edit `worker/src/generated/registry.rs`"),
        "{agents}"
    );
    assert!(agents.contains("puerto generate consumer"), "{agents}");
    assert!(agents.contains("idempotent"), "{agents}");
    let worker_agents = read(&output, "worker/AGENTS.md");
    assert!(worker_agents.contains("idempotent"), "{worker_agents}");
    cleanup(&dir);
}

#[test]
fn new_with_kafka_events_writes_no_worker_yet() {
    let (dir, output) = events_project("worker_kafka", "kafka");

    assert!(!output.join("worker").exists());
    assert!(!read(&output, "Cargo.toml").contains("\"worker\""));
    cleanup(&dir);
}

#[test]
fn new_without_events_writes_no_worker() {
    let dir = temp_dir("worker_no_events");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("worker-no-events", &dir).unwrap();
    scaffold::apply_db_to_new_project(&output).unwrap();

    assert!(!output.join("worker").exists());
    assert!(!read(&output, "Cargo.toml").contains("puerto-events"));
    assert!(!read(&output, "Makefile").contains("events/dead-letters"));
    cleanup(&dir);
}

// ── puerto generate consumer ─────────────────────────────────────────────

#[test]
fn generate_consumer_writes_the_stub_the_toml_entry_and_the_registry_line() {
    let (dir, output) = product_project("consumer_generate");

    scaffold::run_generate_consumer(
        "notify_new_product",
        &on(&["product_created", "product_published"]),
        &output,
    )
    .unwrap();

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(
        config.consumer,
        [puerto_toml::Consumer {
            name: "notify_new_product".into(),
            events: on(&["product_created", "product_published"]),
        }]
    );
    let stub = read(&output, "worker/src/consumers/notify_new_product.rs");
    assert!(
        stub.contains("impl EventHandler for NotifyNewProduct"),
        "{stub}"
    );
    assert!(stub.contains("PRODUCT_CREATED => match event.decode::<ProductSnapshot>()"));
    assert!(stub.contains("PRODUCT_PUBLISHED => match event.decode::<ProductSnapshot>()"));
    assert!(stub.contains("HandlerOutcome::dead_letter(REASON_PAYLOAD_INVALID, err.detail)"));
    assert!(stub.contains("idempotent"), "{stub}");
    for test in [
        "fn should_ack_when_a_product_created_event_is_valid()",
        "fn should_ack_again_when_the_same_event_is_delivered_twice()",
        "fn should_dead_letter_when_the_payload_is_malformed()",
    ] {
        assert!(stub.contains(test), "{test}");
    }
    assert!(read(&output, "worker/src/consumers.rs").contains("pub mod notify_new_product;\n"));
    let registry = read(&output, "worker/src/generated/registry.rs");
    assert!(
        registry.contains(".route(\"notify_new_product\", &[PRODUCT_CREATED, PRODUCT_PUBLISHED])")
    );
    assert!(registry.contains("PgSubscription::new(pool.clone(), \"notify_new_product\")"));
    assert!(registry.contains("use crate::consumers::notify_new_product::NotifyNewProduct;"));
    cleanup(&dir);
}

#[test]
fn generate_consumer_keeps_the_modules_sorted_and_shares_the_dead_letter_sink() {
    let (dir, output) = product_project("consumer_two");

    scaffold::run_generate_consumer("notify_new_product", &on(&["product_created"]), &output)
        .unwrap();
    scaffold::run_generate_consumer("audit", &on(&["product_deleted"]), &output).unwrap();

    let modules = read(&output, "worker/src/consumers.rs");
    assert!(
        modules.ends_with("pub mod audit;\npub mod notify_new_product;\n"),
        "{modules}"
    );
    let registry = read(&output, "worker/src/generated/registry.rs");
    assert!(
        registry.contains("            dead_letters.clone(),\n"),
        "{registry}"
    );
    assert!(
        registry.contains("            dead_letters,\n"),
        "{registry}"
    );
    // Routed in declaration order.
    let notify = registry
        .find(".route(\"notify_new_product\"")
        .unwrap_or(usize::MAX);
    let audit = registry.find(".route(\"audit\"").unwrap_or(0);
    assert!(notify < audit, "{registry}");
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_an_undeclared_event_and_writes_nothing() {
    let (dir, output) = product_project("consumer_unknown_event");

    let err = error_of(scaffold::run_generate_consumer(
        "notify_new_product",
        &on(&["product_created", "product_sold"]),
        &output,
    ));

    assert!(err.contains("product_sold"), "{err}");
    assert!(
        err.contains("product_published"),
        "names the declared ones: {err}"
    );
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    assert!(
        !output
            .join("worker/src/consumers/notify_new_product.rs")
            .exists()
    );
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_an_existing_name() {
    let (dir, output) = product_project("consumer_duplicate");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["product_deleted"]),
        &output,
    ));

    assert!(err.contains("already declared"), "{err}");
    assert_eq!(puerto_toml::read(&output).unwrap().consumer.len(), 1);
    cleanup(&dir);
}

#[test]
fn generate_consumer_never_overwrites_a_handler_on_disk() {
    let (dir, output) = product_project("consumer_overwrite");
    let path = output.join("worker/src/consumers/audit.rs");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "// mine\n").unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["product_created"]),
        &output,
    ));

    assert!(err.contains("never overwritten"), "{err}");
    assert_eq!(read(&output, "worker/src/consumers/audit.rs"), "// mine\n");
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_names_that_would_not_compile() {
    let (dir, output) = product_project("consumer_bad_names");

    for (name, expected) in [
        ("NotifyNewProduct", "snake_case"),
        ("match", "keyword"),
        ("loops", "already uses"),
        ("product_event", "already uses"),
        ("product_snapshot", "already uses"),
    ] {
        let err = error_of(scaffold::run_generate_consumer(
            name,
            &on(&["product_created"]),
            &output,
        ));
        assert!(err.contains(expected), "{name}: {err}");
    }
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    cleanup(&dir);
}

#[test]
fn generate_consumer_is_refused_outside_a_db_events_project() {
    let (dir, output) = events_project("consumer_kafka", "kafka");

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["product_created"]),
        &output,
    ));

    assert!(err.contains("--events db"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_regenerates_the_registry() {
    let (dir, output) = product_project("consumer_bootstrap");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::write(
        output.join("worker/src/generated/registry.rs"),
        "// stale\n",
    )
    .unwrap();

    scaffold::run_bootstrap_command(&output, false).unwrap();

    let registry = read(&output, "worker/src/generated/registry.rs");
    assert!(
        registry.contains(".route(\"audit\", &[PRODUCT_CREATED])"),
        "{registry}"
    );
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_refuses_when_a_declared_consumer_has_no_handler() {
    let (dir, output) = product_project("consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::remove_file(output.join("worker/src/consumers/audit.rs")).unwrap();

    let err = error_of(scaffold::run_bootstrap_command(&output, false));

    assert!(
        err.contains("worker/src/consumers/audit.rs is missing"),
        "{err}"
    );
    cleanup(&dir);
}

// ── puerto validate ─────────────────────────────────────────────────────

fn manifest_with_consumer(consumer: &str) -> String {
    format!(
        "[project]\nname = \"shop\"\ndb = true\nevents = \"db\"\n\n[[entity]]\nname = \"Product\"\n\
         use_cases = [\"create_product\", \"get_product\", \"list_products\", \"update_product\", \
         \"delete_product\"]\ndb = true\nevents = [\"product_published\"]\n\n{consumer}"
    )
}

fn validate_manifest(name: &str, manifest: &str) -> Result<(), String> {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), manifest).unwrap();
    let result = validate::run_validate(&dir).map_err(|e| e.to_string());
    cleanup(&dir);
    result
}

#[test]
fn validate_accepts_a_consumer_of_declared_events() {
    let manifest = manifest_with_consumer(
        "[[consumer]]\nname = \"notify_new_product\"\nevents = [\"product_created\", \"product_published\"]\n",
    );

    assert_eq!(validate_manifest("validate_consumer_ok", &manifest), Ok(()));
}

#[test]
fn validate_rejects_a_consumer_of_an_undeclared_event() {
    let manifest =
        manifest_with_consumer("[[consumer]]\nname = \"audit\"\nevents = [\"product_sold\"]\n");

    let err = validate_manifest("validate_consumer_event", &manifest).unwrap_err();

    assert!(
        err.contains("event 'product_sold' is not declared"),
        "{err}"
    );
}

#[test]
fn validate_rejects_bad_duplicate_and_empty_consumers() {
    let manifest = manifest_with_consumer(
        "[[consumer]]\nname = \"Audit\"\nevents = [\"product_created\"]\n\n\
         [[consumer]]\nname = \"notify\"\nevents = [\"product_created\"]\n\n\
         [[consumer]]\nname = \"notify\"\nevents = []\n",
    );

    let err = validate_manifest("validate_consumer_names", &manifest).unwrap_err();

    assert!(err.contains("invalid consumer name 'Audit'"), "{err}");
    assert!(err.contains("duplicate consumer name 'notify'"), "{err}");
    assert!(
        err.contains("consumer 'notify': subscribes to no event"),
        "{err}"
    );
}

#[test]
fn validate_rejects_consumers_in_a_project_without_events() {
    let manifest = "[project]\nname = \"shop\"\ndb = true\n\n[[consumer]]\nname = \"audit\"\nevents = [\"product_created\"]\n";

    let err = validate_manifest("validate_consumer_no_events", manifest).unwrap_err();

    assert!(err.contains("project.events is not set"), "{err}");
}

#[test]
fn validate_rejects_a_declared_consumer_whose_handler_is_missing() {
    let (dir, output) = product_project("validate_consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::remove_file(output.join("worker/src/consumers/audit.rs")).unwrap();

    let err = validate::run_validate(&output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(
        err.contains("worker/src/consumers/audit.rs is missing"),
        "{err}"
    );
    cleanup(&dir);
}
