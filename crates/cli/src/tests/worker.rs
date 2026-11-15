// Phase 13.4 (ADR-0018 § 5): the `worker/` crate of an `events` project and
// `puerto generar consumidor`; Phase 13.5: the same worker on the `kafka` transport.
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
    product_project_on(name, "db")
}

fn product_project_on(name: &str, transport: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = events_project(name, transport);
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

// ── puerto nuevo ───────────────────────────────────────────────────────────

#[test]
fn new_with_db_events_writes_the_worker_and_adds_it_to_the_workspace() {
    let (dir, output) = events_project("worker_new", "db");

    let root = read(&output, "Cargo.toml");
    // `presentacion/*` takes the worker in: the members do not change.
    assert!(
        root.contains(
            "members = [\"nucleo\", \"migrador\", \"contextos/*/*\", \"presentacion/*\"]"
        ),
        "{root}"
    );
    assert!(root.contains("puerto-eventos = \""), "{root}");
    for path in [
        "presentacion/worker/Cargo.toml",
        "presentacion/worker/AGENTS.md",
        "presentacion/worker/src/main.rs",
        "presentacion/worker/src/generado.rs",
        "presentacion/worker/src/generado/registro.rs",
        "presentacion/worker/src/consumidores.rs",
    ] {
        assert!(output.join(path).exists(), "{path} missing");
    }
    let manifest = read(&output, "presentacion/worker/Cargo.toml");
    assert!(manifest.contains("puerto-eventos = { workspace = true }"));
    let main = read(&output, "presentacion/worker/src/main.rs");
    assert!(main.contains("puerto_eventos::Worker::new(config)"));
    assert!(main.contains(".intentar_ejecutar(generado::registro::build_worker)"));
    cleanup(&dir);
}

#[test]
fn new_with_db_events_relays_to_nobody_until_a_consumer_is_added() {
    let (dir, output) = events_project("worker_registry_empty", "db");

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");

    assert!(registry.starts_with("// AUTO-GENERATED"), "{registry}");
    assert!(
        registry.contains(
            "Bucles::new(&config).relay(OutboxPostgres::new(pool.clone()), EntregasPostgres::new(pool))"
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
    assert!(
        makefile.contains("\nworker: docker-compose/up ## "),
        "{makefile}"
    );
    assert!(makefile.contains("$(CARGO) run -p worker"), "{makefile}");
    assert!(makefile.contains("\nevents/dead-letters: "), "{makefile}");
    assert!(makefile.contains("FROM descartados"), "{makefile}");
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
        agents.contains("Never hand-edit `presentacion/worker/src/generado/registro.rs`"),
        "{agents}"
    );
    assert!(agents.contains("puerto generar consumidor"), "{agents}");
    assert!(agents.contains("idempotent"), "{agents}");
    let worker_agents = read(&output, "presentacion/worker/AGENTS.md");
    assert!(worker_agents.contains("idempotent"), "{worker_agents}");
    cleanup(&dir);
}

#[test]
fn new_with_kafka_events_writes_the_worker_on_the_kafka_transport() {
    let (dir, output) = events_project("worker_kafka", "kafka");

    let root = read(&output, "Cargo.toml");
    // `presentacion/*` takes the worker in: the members do not change.
    assert!(
        root.contains(
            "members = [\"nucleo\", \"migrador\", \"contextos/*/*\", \"presentacion/*\"]"
        ),
        "{root}"
    );
    assert!(root.contains("puerto-eventos = \""), "{root}");
    assert!(root.contains("puerto-kafka = \""), "{root}");
    let manifest = read(&output, "presentacion/worker/Cargo.toml");
    assert!(
        manifest.contains("puerto-kafka = { workspace = true }"),
        "{manifest}"
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(registry.starts_with("// AUTO-GENERATED"), "{registry}");
    assert!(
        registry.contains("ConfigKafka::from_env(\"worker_kafka\")"),
        "{registry}"
    );
    assert!(
        registry.contains(".relay(OutboxPostgres::new(pool), publisher)"),
        "{registry}"
    );
    assert!(!registry.contains("EntregasPostgres"), "{registry}");
    assert!(!registry.contains(".consumer("), "{registry}");
    let worker_agents = read(&output, "presentacion/worker/AGENTS.md");
    assert!(
        worker_agents.contains("## Kafka transport"),
        "{worker_agents}"
    );
    assert!(worker_agents.contains("KAFKA_BROKERS"), "{worker_agents}");
    cleanup(&dir);
}

#[test]
fn new_with_kafka_events_writes_no_event_deliveries_in_the_outbox_migration() {
    let (dir, output) = events_project("worker_kafka_outbox", "kafka");

    let migrations: Vec<String> = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let outbox = migrations
        .iter()
        .find(|m| m.ends_with("_outbox.sql"))
        .unwrap_or_else(|| panic!("no outbox migration in {migrations:?}"));
    let sql = read(&output, &format!("migraciones/{outbox}"));

    assert!(sql.contains("CREATE TABLE outbox ("), "{sql}");
    assert!(sql.contains("CREATE TABLE descartados ("), "{sql}");
    assert!(
        sql.contains("CREATE UNIQUE INDEX descartados_evento_consumidor"),
        "{sql}"
    );
    assert!(!sql.contains("entregas_eventos"), "{sql}");
    cleanup(&dir);
}

#[test]
fn new_with_kafka_events_adds_redpanda_to_docker_compose_and_the_brokers_to_env() {
    let (dir, output) = events_project("worker_kafka_compose", "kafka");

    let compose = read(&output, "docker-compose.yml");
    assert!(compose.contains("\n  postgres:\n"), "{compose}");
    assert!(compose.contains("\n  redpanda:\n"), "{compose}");
    assert!(compose.contains("redpandadata/redpanda"), "{compose}");
    assert!(compose.contains("\n  redpanda-data:\n"), "{compose}");
    for env in [".env", ".env.example"] {
        let content = read(&output, env);
        assert!(content.contains("KAFKA_BROKERS=localhost:9092"), "{env}");
        assert!(content.contains("WORKER_PORT=8081"), "{env}");
    }
    cleanup(&dir);
}

#[test]
fn new_with_db_events_adds_no_broker() {
    let (dir, output) = events_project("worker_db_compose", "db");

    assert!(!read(&output, "docker-compose.yml").contains("redpanda"));
    assert!(!read(&output, ".env").contains("KAFKA_BROKERS"));
    assert!(!read(&output, "Cargo.toml").contains("puerto-kafka"));
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
    assert!(!read(&output, "Cargo.toml").contains("puerto-eventos"));
    assert!(!read(&output, "Makefile").contains("events/dead-letters"));
    cleanup(&dir);
}

// ── puerto generar consumidor ─────────────────────────────────────────────

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
    let stub = read(
        &output,
        "presentacion/worker/src/consumidores/notify_new_product.rs",
    );
    assert!(
        stub.contains("impl ManejadorEvento for NotifyNewProduct"),
        "{stub}"
    );
    assert!(stub.contains("PRODUCT_CREATED => match event.decodificar::<ProductSnapshot>()"));
    assert!(stub.contains("PRODUCT_PUBLISHED => match event.decodificar::<ProductSnapshot>()"));
    assert!(stub.contains("ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, err.detalle)"));
    assert!(stub.contains("idempotent"), "{stub}");
    for test in [
        "fn should_ack_when_a_product_created_event_is_valid()",
        "fn should_ack_again_when_the_same_event_is_delivered_twice()",
        "fn should_dead_letter_when_the_payload_is_malformed()",
    ] {
        assert!(stub.contains(test), "{test}");
    }
    assert!(
        read(&output, "presentacion/worker/src/consumidores.rs")
            .contains("pub mod notify_new_product;\n")
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry
            .contains(".enrutar(\"notify_new_product\", &[PRODUCT_CREATED, PRODUCT_PUBLISHED])")
    );
    assert!(registry.contains("SuscripcionPostgres::new(pool.clone(), \"notify_new_product\")"));
    assert!(registry.contains("use crate::consumidores::notify_new_product::NotifyNewProduct;"));
    cleanup(&dir);
}

#[test]
fn generate_consumer_keeps_the_modules_sorted_and_shares_the_dead_letter_sink() {
    let (dir, output) = product_project("consumer_two");

    scaffold::run_generate_consumer("notify_new_product", &on(&["product_created"]), &output)
        .unwrap();
    scaffold::run_generate_consumer("audit", &on(&["product_deleted"]), &output).unwrap();

    let modules = read(&output, "presentacion/worker/src/consumidores.rs");
    assert!(
        modules.ends_with("pub mod audit;\npub mod notify_new_product;\n"),
        "{modules}"
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
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
        .find(".enrutar(\"notify_new_product\"")
        .unwrap_or(usize::MAX);
    let audit = registry.find(".enrutar(\"audit\"").unwrap_or(0);
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
            .join("presentacion/worker/src/consumidores/notify_new_product.rs")
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
    let path = output.join("presentacion/worker/src/consumidores/audit.rs");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "// mine\n").unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["product_created"]),
        &output,
    ));

    assert!(err.contains("never overwritten"), "{err}");
    assert_eq!(
        read(&output, "presentacion/worker/src/consumidores/audit.rs"),
        "// mine\n"
    );
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_names_that_would_not_compile() {
    let (dir, output) = product_project("consumer_bad_names");

    for (name, expected) in [
        ("NotifyNewProduct", "snake_case"),
        ("match", "keyword"),
        ("bucles", "already uses"),
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
fn generate_consumer_is_refused_outside_an_events_project() {
    let dir = temp_dir("consumer_no_events");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("consumer-no-events", &dir).unwrap();
    scaffold::apply_db_to_new_project(&output).unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["greeting_created"]),
        &output,
    ));

    assert!(err.contains("--eventos"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_consumer_on_kafka_subscribes_to_the_topics_of_its_events() {
    let (dir, output) = product_project_on("consumer_kafka", "kafka");

    scaffold::run_generate_consumer(
        "notify_new_product",
        &on(&["product_created", "product_published"]),
        &output,
    )
    .unwrap();

    assert!(
        output
            .join("presentacion/worker/src/consumidores/notify_new_product.rs")
            .exists()
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains(".ensure_topics(&[\"product\"])"),
        "{registry}"
    );
    assert!(
        registry.contains("SuscripcionKafka::new(\n                &kafka,\n                \"notify_new_product\",\n                &[\"product\"],\n                &[PRODUCT_CREATED, PRODUCT_PUBLISHED],"),
        "{registry}"
    );
    assert!(
        registry.contains("DescartadosPostgres::new(pool.clone())"),
        "{registry}"
    );
    assert!(registry.contains("use crate::consumidores::notify_new_product::NotifyNewProduct;"));
    assert!(!registry.contains("SuscripcionPostgres"), "{registry}");
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_does_not_bring_back_a_removed_worker() {
    let (dir, output) = product_project("consumer_worker_removed");
    fs::remove_dir_all(output.join("presentacion/worker/src/generado")).unwrap();

    scaffold::run_bootstrap_command(&output, false).unwrap();

    assert!(!output.join("presentacion/worker/src/generado").exists());
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_regenerates_the_registry() {
    let (dir, output) = product_project("consumer_bootstrap");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::write(
        output.join("presentacion/worker/src/generado/registro.rs"),
        "// stale\n",
    )
    .unwrap();

    scaffold::run_bootstrap_command(&output, false).unwrap();

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains(".enrutar(\"audit\", &[PRODUCT_CREATED])"),
        "{registry}"
    );
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_refuses_when_a_declared_consumer_has_no_handler() {
    let (dir, output) = product_project("consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::remove_file(output.join("presentacion/worker/src/consumidores/audit.rs")).unwrap();

    let err = error_of(scaffold::run_bootstrap_command(&output, false));

    assert!(
        err.contains("presentacion/worker/src/consumidores/audit.rs is missing"),
        "{err}"
    );
    cleanup(&dir);
}

// ── puerto validar ─────────────────────────────────────────────────────

fn manifest_with_consumer(consumer: &str) -> String {
    format!(
        "[proyecto]\nnombre = \"shop\"\ndb = true\neventos = \"db\"\n\n[[entidad]]\nnombre = \"Product\"\n\
         casos_de_uso = [\"create_product\", \"get_product\", \"list_products\", \"update_product\", \
         \"delete_product\"]\ndb = true\neventos = [\"product_published\"]\n\n{consumer}"
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
        "[[consumidor]]\nnombre = \"notify_new_product\"\neventos = [\"product_created\", \"product_published\"]\n",
    );

    assert_eq!(validate_manifest("validate_consumer_ok", &manifest), Ok(()));
}

#[test]
fn validate_rejects_a_consumer_of_an_undeclared_event() {
    let manifest = manifest_with_consumer(
        "[[consumidor]]\nnombre = \"audit\"\neventos = [\"product_sold\"]\n",
    );

    let err = validate_manifest("validate_consumer_event", &manifest).unwrap_err();

    assert!(
        err.contains("event 'product_sold' is not declared"),
        "{err}"
    );
}

#[test]
fn validate_rejects_bad_duplicate_and_empty_consumers() {
    let manifest = manifest_with_consumer(
        "[[consumidor]]\nnombre = \"Audit\"\neventos = [\"product_created\"]\n\n\
         [[consumidor]]\nnombre = \"notify\"\neventos = [\"product_created\"]\n\n\
         [[consumidor]]\nnombre = \"notify\"\neventos = []\n",
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
    let manifest = "[proyecto]\nnombre = \"shop\"\ndb = true\n\n[[consumidor]]\nnombre = \"audit\"\neventos = [\"product_created\"]\n";

    let err = validate_manifest("validate_consumer_no_events", manifest).unwrap_err();

    assert!(err.contains("project.events is not set"), "{err}");
}

#[test]
fn validate_rejects_a_declared_consumer_whose_handler_is_missing() {
    let (dir, output) = product_project("validate_consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["product_created"]), &output).unwrap();
    fs::remove_file(output.join("presentacion/worker/src/consumidores/audit.rs")).unwrap();

    let err = validate::run_validate(&output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(
        err.contains("presentacion/worker/src/consumidores/audit.rs is missing"),
        "{err}"
    );
    cleanup(&dir);
}
