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

/// An events project with a CRUD `Producto` that declares `producto_published`.
fn producto_project(name: &str) -> (PathBuf, PathBuf) {
    producto_project_on(name, "db")
}

fn producto_project_on(name: &str, transport: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = events_project(name, transport);
    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Ask).unwrap();
    scaffold::run_generate_event("Producto", "producto_published", &output).unwrap();
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
    // A consumer is given the pool (Phase 20.3); `cableado` only once one uses it — an unused
    // dependency is a Cargo warning.
    assert!(!manifest.contains("\ncableado = "), "{manifest}");
    assert!(
        manifest.contains("sqlx = { workspace = true }"),
        "{manifest}"
    );
    let main = read(&output, "presentacion/worker/src/main.rs");
    assert!(main.contains("puerto_eventos::Worker::new(config)"));
    assert!(main.contains(".intentar_ejecutar(generado::registro::construir_worker)"));
    cleanup(&dir);
}

#[test]
fn new_with_db_events_relays_to_nobody_until_a_consumer_is_added() {
    let (dir, output) = events_project("worker_registry_vacio", "db");

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");

    assert!(registry.starts_with("// GENERADO"), "{registry}");
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
    assert!(makefile.contains("\neventos/descartados: "), "{makefile}");
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
        agents.contains("No edites nunca a mano `presentacion/worker/src/generado/registro.rs`"),
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
    assert!(registry.starts_with("// GENERADO"), "{registry}");
    assert!(
        registry.contains("ConfigKafka::from_env(\"worker_kafka\")"),
        "{registry}"
    );
    assert!(
        registry.contains(".relay(OutboxPostgres::new(pool), publicador)"),
        "{registry}"
    );
    assert!(!registry.contains("EntregasPostgres"), "{registry}");
    assert!(!registry.contains(".consumer("), "{registry}");
    let worker_agents = read(&output, "presentacion/worker/AGENTS.md");
    assert!(
        worker_agents.contains("## Transporte Kafka"),
        "{worker_agents}"
    );
    assert!(worker_agents.contains("KAFKA_BROKERS"), "{worker_agents}");
    cleanup(&dir);
}

#[test]
fn new_with_kafka_events_writes_no_event_entregas_in_the_outbox_migration() {
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
    assert!(!read(&output, "Makefile").contains("eventos/descartados"));
    cleanup(&dir);
}

// ── puerto generar consumidor ─────────────────────────────────────────────

#[test]
fn generate_consumer_writes_the_stub_the_toml_entry_and_the_registry_line() {
    let (dir, output) = producto_project("consumer_generate");

    scaffold::run_generate_consumer(
        "notify_new_producto",
        &on(&["producto_creado", "producto_published"]),
        &output,
    )
    .unwrap();

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(
        config.consumer,
        [puerto_toml::Consumer {
            name: "notify_new_producto".into(),
            events: on(&["producto_creado", "producto_published"]),
        }]
    );
    let stub = read(
        &output,
        "presentacion/worker/src/consumidores/notify_new_producto.rs",
    );
    assert!(
        stub.contains("impl ManejadorEvento for NotifyNewProducto"),
        "{stub}"
    );
    assert!(stub.contains("PRODUCTO_CREADO => match evento.decodificar::<SnapshotProducto>()"));
    assert!(stub.contains("PRODUCTO_PUBLISHED => match evento.decodificar::<SnapshotProducto>()"));
    assert!(stub.contains("ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, err.detalle)"));
    assert!(stub.contains("idempotent"), "{stub}");
    // canasta opened its own pool in a OnceCell: the registry hands each consumer its pool.
    assert!(stub.contains("use sqlx::PgPool;"), "{stub}");
    assert!(
        stub.contains("    pub fn construir(_pool: &PgPool) -> Self {\n        Self\n    }"),
        "{stub}"
    );
    for test in [
        "fn deberia_hacer_ack_cuando_el_evento_producto_creado_es_valido()",
        "fn deberia_hacer_ack_otra_vez_cuando_el_mismo_evento_llega_dos_veces()",
        "fn deberia_descartar_cuando_el_payload_esta_mal_formado()",
    ] {
        assert!(stub.contains(test), "{test}");
    }
    assert!(
        read(&output, "presentacion/worker/src/consumidores.rs")
            .contains("pub mod notify_new_producto;\n")
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry
            .contains(".enrutar(\"notify_new_producto\", &[PRODUCTO_CREADO, PRODUCTO_PUBLISHED])")
    );
    assert!(registry.contains("SuscripcionPostgres::new(pool.clone(), \"notify_new_producto\")"));
    assert!(
        registry.contains("NotifyNewProducto::construir(&pool),"),
        "{registry}"
    );
    assert!(registry.contains("use crate::consumidores::notify_new_producto::NotifyNewProducto;"));
    cleanup(&dir);
}

#[test]
fn generate_consumer_keeps_the_modules_sorted_and_shares_the_dead_letter_sink() {
    let (dir, output) = producto_project("consumer_two");

    scaffold::run_generate_consumer("notify_new_producto", &on(&["producto_creado"]), &output)
        .unwrap();
    scaffold::run_generate_consumer("audit", &on(&["producto_borrado"]), &output).unwrap();

    let modules = read(&output, "presentacion/worker/src/consumidores.rs");
    assert!(
        modules.ends_with("pub mod audit;\npub mod comun;\npub mod notify_new_producto;\n"),
        "{modules}"
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains("            descartados.clone(),\n"),
        "{registry}"
    );
    assert!(
        registry.contains("            descartados,\n"),
        "{registry}"
    );
    // Routed in declaration pedido.
    let notify = registry
        .find(".enrutar(\"notify_new_producto\"")
        .unwrap_or(usize::MAX);
    let audit = registry.find(".enrutar(\"audit\"").unwrap_or(0);
    assert!(notify < audit, "{registry}");
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_an_undeclared_event_and_writes_nothing() {
    let (dir, output) = producto_project("consumer_unknown_event");

    let err = error_of(scaffold::run_generate_consumer(
        "notify_new_producto",
        &on(&["producto_creado", "producto_sold"]),
        &output,
    ));

    assert!(err.contains("producto_sold"), "{err}");
    assert!(
        err.contains("producto_published"),
        "names the declared ones: {err}"
    );
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    assert!(
        !output
            .join("presentacion/worker/src/consumidores/notify_new_producto.rs")
            .exists()
    );
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_an_existing_name() {
    let (dir, output) = producto_project("consumer_duplicate");
    scaffold::run_generate_consumer("audit", &on(&["producto_creado"]), &output).unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["producto_borrado"]),
        &output,
    ));

    assert!(err.contains("ya está declarad"), "{err}");
    assert_eq!(puerto_toml::read(&output).unwrap().consumer.len(), 1);
    cleanup(&dir);
}

#[test]
fn generate_consumer_never_overwrites_a_handler_on_disk() {
    let (dir, output) = producto_project("consumer_overwrite");
    let path = output.join("presentacion/worker/src/consumidores/audit.rs");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "// mine\n").unwrap();

    let err = error_of(scaffold::run_generate_consumer(
        "audit",
        &on(&["producto_creado"]),
        &output,
    ));

    assert!(err.contains("nunca se sobrescribe"), "{err}");
    assert_eq!(
        read(&output, "presentacion/worker/src/consumidores/audit.rs"),
        "// mine\n"
    );
    assert!(puerto_toml::read(&output).unwrap().consumer.is_empty());
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_names_that_would_not_compile() {
    let (dir, output) = producto_project("consumer_bad_names");

    for (name, expected) in [
        ("NotifyNewProducto", "snake_case"),
        ("match", "palabra clave"),
        ("bucles", "ya usa"),
        ("evento_producto", "ya usa"),
        ("snapshot_producto", "ya usa"),
    ] {
        let err = error_of(scaffold::run_generate_consumer(
            name,
            &on(&["producto_creado"]),
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
        &on(&["saludo_creado"]),
        &output,
    ));

    assert!(err.contains("--eventos"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_consumer_on_kafka_subscribes_to_the_topics_of_its_events() {
    let (dir, output) = producto_project_on("consumer_kafka", "kafka");

    scaffold::run_generate_consumer(
        "notify_new_producto",
        &on(&["producto_creado", "producto_published"]),
        &output,
    )
    .unwrap();

    assert!(
        output
            .join("presentacion/worker/src/consumidores/notify_new_producto.rs")
            .exists()
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains(".asegurar_topics(&[\"producto\"])"),
        "{registry}"
    );
    assert!(
        registry.contains("SuscripcionKafka::new(\n                &kafka,\n                \"notify_new_producto\",\n                &[\"producto\"],\n                &[PRODUCTO_CREADO, PRODUCTO_PUBLISHED],"),
        "{registry}"
    );
    assert!(
        registry.contains("DescartadosPostgres::new(pool.clone())"),
        "{registry}"
    );
    assert!(registry.contains("use crate::consumidores::notify_new_producto::NotifyNewProducto;"));
    assert!(!registry.contains("SuscripcionPostgres"), "{registry}");
    assert!(
        registry.contains("NotifyNewProducto::construir(&pool),"),
        "{registry}"
    );
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_does_not_bring_back_a_removed_worker() {
    let (dir, output) = producto_project("consumer_worker_removed");
    fs::remove_dir_all(output.join("presentacion/worker/src/generado")).unwrap();

    scaffold::run_bootstrap_command(&output, false).unwrap();

    assert!(!output.join("presentacion/worker/src/generado").exists());
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_regenerates_the_registry() {
    let (dir, output) = producto_project("consumer_bootstrap");
    scaffold::run_generate_consumer("audit", &on(&["producto_creado"]), &output).unwrap();
    fs::write(
        output.join("presentacion/worker/src/generado/registro.rs"),
        "// stale\n",
    )
    .unwrap();

    scaffold::run_bootstrap_command(&output, false).unwrap();

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains(".enrutar(\"audit\", &[PRODUCTO_CREADO])"),
        "{registry}"
    );
    cleanup(&dir);
}

#[test]
fn generate_bootstrap_refuses_when_a_declared_consumer_has_no_handler() {
    let (dir, output) = producto_project("consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["producto_creado"]), &output).unwrap();
    fs::remove_file(output.join("presentacion/worker/src/consumidores/audit.rs")).unwrap();

    let err = error_of(scaffold::run_bootstrap_command(&output, false));

    assert!(
        err.contains("falta presentacion/worker/src/consumidores/audit.rs"),
        "{err}"
    );
    cleanup(&dir);
}

// ── puerto validar ─────────────────────────────────────────────────────

fn manifest_with_consumer(consumer: &str) -> String {
    format!(
        "[proyecto]\nnombre = \"shop\"\ndb = true\neventos = \"db\"\n\n[[entidad]]\nnombre = \"Producto\"\n\
         casos_de_uso = [\"crear_producto\", \"obtener_producto\", \"listar_productos\", \"actualizar_producto\", \
         \"borrar_producto\"]\ndb = true\neventos = [\"producto_published\"]\n\n{consumer}"
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
        "[[consumidor]]\nnombre = \"notify_new_producto\"\neventos = [\"producto_creado\", \"producto_published\"]\n",
    );

    assert_eq!(validate_manifest("validate_consumer_ok", &manifest), Ok(()));
}

#[test]
fn validate_rejects_a_consumer_of_an_undeclared_event() {
    let manifest = manifest_with_consumer(
        "[[consumidor]]\nnombre = \"audit\"\neventos = [\"producto_sold\"]\n",
    );

    let err = validate_manifest("validate_consumer_event", &manifest).unwrap_err();

    assert!(
        err.contains("ninguna entidad declara el evento 'producto_sold'"),
        "{err}"
    );
}

#[test]
fn validate_rejects_bad_duplicate_and_empty_consumers() {
    let manifest = manifest_with_consumer(
        "[[consumidor]]\nnombre = \"Audit\"\neventos = [\"producto_creado\"]\n\n\
         [[consumidor]]\nnombre = \"notify\"\neventos = [\"producto_creado\"]\n\n\
         [[consumidor]]\nnombre = \"notify\"\neventos = []\n",
    );

    let err = validate_manifest("validate_consumer_names", &manifest).unwrap_err();

    assert!(
        err.contains("nombre de consumidor inválido 'Audit'"),
        "{err}"
    );
    assert!(
        err.contains("el consumidor 'notify' está repetido"),
        "{err}"
    );
    assert!(
        err.contains("consumidor 'notify': no se suscribe a ningún evento"),
        "{err}"
    );
}

#[test]
fn validate_rejects_consumers_in_a_project_without_events() {
    let manifest = "[proyecto]\nnombre = \"shop\"\ndb = true\n\n[[consumidor]]\nnombre = \"audit\"\neventos = [\"producto_creado\"]\n";

    let err = validate_manifest("validate_consumer_no_events", manifest).unwrap_err();

    assert!(err.contains("proyecto.eventos no está puesto"), "{err}");
}

#[test]
fn validate_rejects_a_declared_consumer_whose_handler_is_missing() {
    let (dir, output) = producto_project("validate_consumer_drift");
    scaffold::run_generate_consumer("audit", &on(&["producto_creado"]), &output).unwrap();
    fs::remove_file(output.join("presentacion/worker/src/consumidores/audit.rs")).unwrap();

    let err = validate::run_validate(&output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(
        err.contains("falta presentacion/worker/src/consumidores/audit.rs"),
        "{err}"
    );
    cleanup(&dir);
}

// ── 27.14: what a consumer needs ───────────────────────────────────────────

const CONTROL_STOCK: &str = "presentacion/worker/src/consumidores/control_stock.rs";

#[test]
fn generate_consumer_with_use_cases_holds_builds_and_doubles_them() {
    let (dir, output) = producto_project("consumer_uses");
    scaffold::run_use_case_with("Producto", "agotar_producto", true, &output).unwrap();

    scaffold::run_generate_consumer_with(
        "control_stock",
        &on(&["producto_creado"]),
        &on(&["obtener_producto", "agotar_producto"]),
        &output,
    )
    .unwrap();

    let consumer = read(&output, CONTROL_STOCK);
    assert!(consumer.contains("pub struct ControlStock {"), "{consumer}");
    assert!(consumer.contains("pub obtener_producto: Arc<dyn ObtenerProducto>,"));
    assert!(consumer.contains("pub agotar_producto: Arc<dyn AgotarProducto>,"));
    assert!(consumer.contains("use aplicacion_producto::obtener_producto::ObtenerProducto;"));
    assert!(consumer.contains("let producto = cableado::producto::construir(pool);"));
    assert!(consumer.contains("obtener_producto: producto.obtener_producto,"));
    assert!(consumer.contains("obtener_producto: Arc::new(MockObtenerProducto::new()),"));
    assert!(
        !consumer.contains("Principal::sistema"),
        "no auth: {consumer}"
    );
    let manifest = read(&output, "presentacion/worker/Cargo.toml");
    assert!(
        manifest.contains("cableado = { workspace = true }"),
        "{manifest}"
    );
    assert!(manifest.contains("aplicacion_producto = { workspace = true }"));
    assert!(
        manifest.contains("aplicacion_producto = { workspace = true, features = [\"pruebas\"] }")
    );
    // The handler is the user's from now on: the manifest records only its subscriptions.
    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(config.consumer[0].events, on(&["producto_creado"]));
    cleanup(&dir);
}

#[test]
fn generate_consumer_refuses_a_use_case_no_entity_declares_and_writes_nothing() {
    let (dir, output) = producto_project("consumer_uses_unknown");
    let manifest = read(&output, "puerto.toml");

    let err = error_of(scaffold::run_generate_consumer_with(
        "control_stock",
        &on(&["producto_creado"]),
        &on(&["obtener_producto", "teletransportar_producto"]),
        &output,
    ));

    assert!(err.contains("teletransportar_producto"), "{err}");
    assert!(!output.join(CONTROL_STOCK).exists());
    assert_eq!(read(&output, "puerto.toml"), manifest);
    cleanup(&dir);
}

#[test]
fn a_consumer_of_an_auth_project_acts_as_the_system() {
    let (dir, output) = events_project("consumer_uses_auth", "db");
    crate::generators::auth::apply_auth_to_new_project(&output, "oidc").unwrap();
    let fields = [scaffold_test_field("name", "String")];
    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Ask).unwrap();

    scaffold::run_generate_consumer_with(
        "control_stock",
        &on(&["producto_creado"]),
        &on(&["listar_productos"]),
        &output,
    )
    .unwrap();

    let consumer = read(&output, CONTROL_STOCK);
    assert!(
        consumer.contains("Principal::sistema(\"control_stock\")"),
        "{consumer}"
    );
    assert!(consumer.contains("todas_las_paginas"), "a list: {consumer}");
    cleanup(&dir);
}

#[test]
fn the_worker_has_a_module_for_what_consumers_share_and_it_is_no_consumer() {
    let (dir, output) = producto_project("consumer_comun");
    let comun = read(&output, "presentacion/worker/src/consumidores/comun.rs");
    assert!(comun.starts_with("//!"), "{comun}");

    let err = error_of(scaffold::run_generate_consumer(
        "comun",
        &on(&["producto_creado"]),
        &output,
    ));
    assert!(err.contains("comun"), "{err}");

    scaffold::run_generate_consumer("notify_new_producto", &on(&["producto_creado"]), &output)
        .unwrap();
    let modules = read(&output, "presentacion/worker/src/consumidores.rs");
    assert!(
        modules.contains("pub mod comun;\npub mod notify_new_producto;\n"),
        "{modules}"
    );
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}
