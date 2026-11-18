// Phase 13.3 (ADR-0018): `puerto nuevo --db --eventos db` — units of work, `events.rs`, the outbox.
use super::*;

use crate::generators::naming::{crud_events, event_variant, is_crud_entity};
use crate::validation::{validate_event_name, validate_events_transport};

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A fresh `--db --events db` project (no web: nothing here concerns the frontend).
fn events_project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::apply_db_to_new_project(&output).unwrap();
    scaffold::apply_events_to_new_project(&output, "db").unwrap();
    scaffold::apply_no_web(&output).unwrap();
    (dir, output)
}

fn scaffold_producto(output: &Path) {
    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "sku:String!", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Producto", output, &fields, OverwritePolicy::Ask).unwrap();
}

fn write_manifest(name: &str, manifest: &str) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), manifest).unwrap();
    dir
}

// ── puerto nuevo ───────────────────────────────────────────────────────────

#[test]
fn new_with_events_but_no_database_is_refused_naming_db() {
    let dir = temp_dir("events_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let result = crate::commands::new::new_project(
        Some("no-db-events".into()),
        crate::commands::new::NewOptions {
            events: Some("db".into()),
            ..Default::default()
        },
        Some(dir.clone()),
    );

    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(err.contains("--db"), "{err}");
    assert!(!dir.join("no-db-events").exists(), "nothing is generated");
    cleanup(&dir);
}

#[test]
fn new_with_an_unknown_events_transport_is_refused() {
    let result = crate::commands::new::new_project(
        Some("nats-events".into()),
        crate::commands::new::NewOptions {
            db: true,
            events: Some("nats".into()),
            ..Default::default()
        },
        Some(temp_dir("events_nats")),
    );

    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(err.contains("db, kafka"), "{err}");
}

#[test]
fn new_with_events_writes_the_transport_the_outbox_migration_and_serde() {
    let (dir, output) = events_project("events_new");

    let manifest = read(&output, "puerto.toml");
    assert!(manifest.contains("eventos = \"db\""), "{manifest}");

    let migrations: Vec<String> = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let outbox = migrations
        .iter()
        .find(|m| m.ends_with("_outbox.sql"))
        .unwrap_or_else(|| panic!("no outbox migration in {migrations:?}"));
    let sql = read(&output, &format!("migraciones/{outbox}"));
    assert!(sql.ends_with(include_str!("../templates/infrastructure/outbox.sql")));

    // The snapshots derive serde: every domain crate of an events project names it, and the root
    // declares the versions once one does.
    scaffold_producto(&output);
    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains("serde = { version = \"1\", features = [\"derive\"] }"),
        "{root}"
    );
    assert!(root.contains("serde_json = \"1\""), "{root}");
    assert!(root.contains("features = [\"v4\", \"serde\"]"), "{root}");
    let domain = read(&output, "contextos/producto/dominio/Cargo.toml");
    assert!(domain.contains("serde = { workspace = true }"), "{domain}");
    assert!(
        domain.contains("serde_json = { workspace = true }"),
        "{domain}"
    );
    cleanup(&dir);
}

#[test]
fn new_with_events_tells_agents_that_events_rs_is_never_hand_edited() {
    let (dir, output) = events_project("events_agents");

    let agents = read(&output, "AGENTS.md");
    assert!(
        agents.contains("No edites nunca a mano `contextos/<c>/dominio/src/eventos.rs`"),
        "{agents}"
    );
    assert!(agents.contains("puerto generar evento"), "{agents}");
    // In front of the conventions, which stay.
    let events_at = agents.find("## Eventos de dominio").unwrap_or(usize::MAX);
    let conventions_at = agents.find("## Convenciones").unwrap_or(0);
    assert!(events_at < conventions_at, "{agents}");
    cleanup(&dir);
}

#[test]
fn events_need_a_database_project() {
    let dir = temp_dir("events_apply_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("apply-no-db", &dir).unwrap();

    let result = scaffold::apply_events_to_new_project(&output, "db");

    assert!(result.is_err());
    cleanup(&dir);
}

// ── scaffold ─────────────────────────────────────────────────────────────

#[test]
fn scaffold_in_an_events_project_writes_events_and_units_of_work() {
    let (dir, output) = events_project("events_scaffold");

    scaffold_producto(&output);

    let events = read(&output, "contextos/producto/dominio/src/eventos.rs");
    assert!(events.starts_with("// GENERADO"));
    assert!(events.contains("pub struct SnapshotProducto"));
    assert!(events.contains("#[serde(rename_all = \"camelCase\")]"));
    assert!(events.contains("pub name: String,"), "VOs are unwrapped");
    for variant in ["Creado", "Actualizado", "Borrado"] {
        assert!(events.contains(&format!("    {variant}(SnapshotProducto),")));
    }
    assert!(events.contains("pub const PRODUCTO_CREADO: &str = \"producto_creado\";"));
    assert!(events.contains("impl EventoDominio for EventoProducto"));

    let uow = read(&output, "contextos/producto/dominio/src/unit_of_work.rs");
    assert!(uow.contains("pub trait UnitOfWorkProducto: Send"));
    assert!(uow.contains("pub trait UnitOfWorkFactoryProducto: Send + Sync"));
    // ADR-0026: one call for the common write — begin, guardar, registrar, commit.
    assert!(
        uow.contains(
            "async fn guardar(&self, entidad: &Producto, evento: EventoProducto) -> Result<(), ErrorRepositorio> {"
        ),
        "{uow}"
    );
    assert!(uow.contains("pub struct UnitOfWorkFactoryGrabadora"));

    let pg_uow = read(
        &output,
        "contextos/producto/infraestructura/src/unit_of_work.rs",
    );
    assert!(pg_uow.contains("pub struct UnitOfWorkFactoryProductoPostgres"));
    assert!(pg_uow.contains("ON CONFLICT (id) DO UPDATE"));
    assert!(pg_uow.contains("use puerto_sqlx::uow::TransaccionPostgres;"));

    let lib = read(&output, "contextos/producto/dominio/src/lib.rs");
    assert!(lib.contains("pub mod eventos;"), "{lib}");
    assert!(lib.contains("pub mod unit_of_work;"), "{lib}");
    let infra_lib = read(&output, "contextos/producto/infraestructura/src/lib.rs");
    assert!(infra_lib.contains("pub mod unit_of_work;\n"), "{infra_lib}");
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_leaves_the_repository_read_only() {
    let (dir, output) = events_project("events_read_only");

    scaffold_producto(&output);

    let port = read(&output, "contextos/producto/dominio/src/repositorio.rs");
    assert!(!port.contains("async fn guardar"), "{port}");
    assert!(port.contains("async fn buscar_por_id"));
    let pg = read(
        &output,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(!pg.contains("async fn guardar"), "{pg}");
    assert!(
        !pg.contains("INSERT INTO"),
        "the upsert moved to the unit of work"
    );
    cleanup(&dir);
}

/// ADR-0026: the entity decides the fact and returns its event; an event nobody saves is a warning.
#[test]
fn scaffold_in_an_events_project_has_the_entity_emit_its_events() {
    let (dir, output) = events_project("events_entity_emits");

    scaffold_producto(&output);

    let model = read(&output, "contextos/producto/dominio/src/modelo.rs");
    for signature in [
        "pub fn crear(props: PropsProducto) -> Result<(Self, EventoProducto), ErrorProducto> {",
        "pub fn actualizar(&mut self, props: PropsProducto) -> Result<EventoProducto, ErrorProducto> {",
        "pub fn borrar(&mut self) -> EventoProducto {",
    ] {
        assert!(model.contains(signature), "{signature}\n{model}");
    }
    for event in ["Creado", "Actualizado", "Borrado"] {
        assert!(
            model.contains(&format!("EventoProducto::{event}(")),
            "{event}"
        );
    }
    let events = read(&output, "contextos/producto/dominio/src/eventos.rs");
    assert!(events.contains("#[must_use = "), "{events}");

    let create = read(
        &output,
        "contextos/producto/aplicacion/src/crear_producto.rs",
    );
    assert!(
        create.contains("let (entidad, evento) = Producto::crear("),
        "{create}"
    );
    let delete = read(
        &output,
        "contextos/producto/aplicacion/src/borrar_producto.rs",
    );
    assert!(
        delete.contains("let evento = entidad.borrar();"),
        "{delete}"
    );
    assert!(!delete.contains("borrado = true"), "{delete}");
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_writes_through_the_unit_of_work() {
    let (dir, output) = events_project("events_writes");

    scaffold_producto(&output);

    for (action, event) in [
        ("crear", "Creado"),
        ("actualizar", "Actualizado"),
        ("borrar", "Borrado"),
    ] {
        let uc = read(
            &output,
            &format!("contextos/producto/aplicacion/src/{action}_producto.rs"),
        );
        assert!(
            uc.contains("pub uow_factory: Arc<dyn UnitOfWorkFactoryProducto>,"),
            "{action}"
        );
        // ADR-0026: the entity emits the event; the factory's `guardar` opens, writes, records and
        // commits in one call.
        assert!(
            uc.contains("self.uow_factory.guardar(&entidad, evento)"),
            "{action}: {uc}"
        );
        assert!(
            !uc.contains(&format!("EventoProducto::{event}(")),
            "{action}: {uc}"
        );
        assert!(!uc.contains("begin()"), "{action}");
        assert!(!uc.contains("repository.guardar"), "{action}");
    }
    let create = read(
        &output,
        "contextos/producto/aplicacion/src/crear_producto.rs",
    );
    assert!(!create.contains("pub repository:"), "create only writes");
    assert!(create.contains(
        "fn deberia_registrar_el_evento_producto_creado_cuando_los_campos_son_validos()"
    ));
    assert!(create.contains("fn no_deberia_hacer_commit_cuando_el_almacen_rechaza_un_duplicado()"));
    let update = read(
        &output,
        "contextos/producto/aplicacion/src/actualizar_producto.rs",
    );
    assert!(update.contains("pub repositorio: Arc<dyn RepositorioProducto>,"));
    assert!(update.contains(
        "fn deberia_devolver_no_encontrado_sin_registrar_nada_cuando_producto_no_existe()"
    ));
    let get = read(
        &output,
        "contextos/producto/aplicacion/src/obtener_producto.rs",
    );
    assert!(
        !get.contains("uow_factory"),
        "reads do not open a unit of work"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_wires_the_unit_of_work_in_bootstrap() {
    let (dir, output) = events_project("events_bootstrap");

    scaffold_producto(&output);

    let bootstrap = wiring(&output);
    assert!(bootstrap.contains(
        "let uow_factory: Arc<dyn UnitOfWorkFactoryProducto> = Arc::new(UnitOfWorkFactoryProductoPostgres::new(pool.clone()));"
    ), "{bootstrap}");
    assert!(bootstrap.contains(
        "crear_producto: Arc::new(CrearProductoImpl { uow_factory: Arc::clone(&uow_factory) }),"
    ));
    assert!(bootstrap.contains(
        "obtener_producto: Arc::new(ObtenerProductoImpl { repositorio: Arc::clone(&repositorio) }),"
    ));
    assert!(bootstrap.contains(
        "actualizar_producto: Arc::new(ActualizarProductoImpl { repositorio: Arc::clone(&repositorio), uow_factory: Arc::clone(&uow_factory) }),"
    ));
    cleanup(&dir);
}

#[test]
fn scaffold_without_events_writes_no_events_files() {
    let dir = temp_dir("events_absent");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("events-absent", &dir).unwrap();
    scaffold::apply_db_to_new_project(&output).unwrap();

    scaffold_producto(&output);

    assert!(
        !output
            .join("contextos/producto/dominio/src/eventos.rs")
            .exists()
    );
    assert!(
        !output
            .join("contextos/producto/dominio/src/unit_of_work.rs")
            .exists()
    );
    assert!(
        !output
            .join("contextos/producto/infraestructura/src/unit_of_work.rs")
            .exists()
    );
    assert!(
        read(&output, "contextos/producto/dominio/src/repositorio.rs").contains("async fn guardar")
    );
    cleanup(&dir);
}

#[test]
fn the_demo_entity_keeps_its_repository_in_an_events_project() {
    // A simple (non-CRUD) entity has no writes to record: events are a CRUD-write concern.
    let (dir, output) = events_project("events_demo");

    assert!(
        !output
            .join("contextos/saludo/dominio/src/eventos.rs")
            .exists()
    );
    let bootstrap = wiring(&output);
    assert!(!bootstrap.contains("UnitOfWork"), "{bootstrap}");
    cleanup(&dir);
}

// ── generate use-case / generate event ──────────────────────────────────

#[test]
fn custom_use_case_in_an_events_project_gets_the_repository_and_the_unit_of_work() {
    let (dir, output) = events_project("events_use_case");
    scaffold_producto(&output);

    scaffold::run_use_case("Producto", "publish_producto", &output).unwrap();

    let uc = read(
        &output,
        "contextos/producto/aplicacion/src/publish_producto.rs",
    );
    assert!(uc.contains("pub repositorio: Arc<dyn RepositorioProducto>,"));
    assert!(uc.contains("pub uow_factory: Arc<dyn UnitOfWorkFactoryProducto>,"));
    assert!(uc.contains("Las escrituras pasan por la unidad de trabajo"));
    let bootstrap = wiring(&output);
    assert!(bootstrap.contains(
        "publish_producto: Arc::new(PublishProductoImpl { repositorio: Arc::clone(&repositorio), uow_factory: Arc::clone(&uow_factory) }),"
    ), "{bootstrap}");
    cleanup(&dir);
}

#[test]
fn generate_event_declares_it_and_regenerates_events_rs() {
    let (dir, output) = events_project("events_generate");
    scaffold_producto(&output);

    scaffold::run_generate_event("Producto", "producto_published", &output).unwrap();

    let config = puerto_toml::read(&output).unwrap();
    let producto = config.entity.iter().find(|e| e.name == "Producto").unwrap();
    assert_eq!(producto.events, ["producto_published"]);
    let events = read(&output, "contextos/producto/dominio/src/eventos.rs");
    assert!(events.contains("    Published(SnapshotProducto),"));
    assert!(events.contains("Self::Published(_) => PRODUCTO_PUBLISHED,"));
    // No use case records it until one is written to.
    let create = read(
        &output,
        "contextos/producto/aplicacion/src/crear_producto.rs",
    );
    assert!(!create.contains("Published"));
    cleanup(&dir);
}

#[test]
fn custom_events_survive_a_forced_rescaffold_and_a_bootstrap() {
    let (dir, output) = events_project("events_survive");
    scaffold_producto(&output);
    scaffold::run_generate_event("Producto", "producto_published", &output).unwrap();

    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "sku:String!", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Force).unwrap();
    scaffold::run_bootstrap_command(&output, false).unwrap();

    let events = read(&output, "contextos/producto/dominio/src/eventos.rs");
    assert!(events.contains("    Published(SnapshotProducto),"));
    cleanup(&dir);
}

#[test]
fn generate_event_refuses_a_crud_event_a_duplicate_and_a_bad_name() {
    let (dir, output) = events_project("events_refuse");
    scaffold_producto(&output);
    scaffold::run_generate_event("Producto", "producto_published", &output).unwrap();

    for (event, expected) in [
        ("producto_creado", "choca"),
        ("borrado", "choca"),
        ("producto_published", "ya está declarado"),
        ("published", "ya está declarado"),
        ("ProductoPublished", "snake_case"),
    ] {
        let err = scaffold::run_generate_event("Producto", event, &output)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains(expected), "{event}: {err}");
    }
    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(
        config
            .entity
            .iter()
            .find(|e| e.name == "Producto")
            .unwrap()
            .events
            .len(),
        1
    );
    cleanup(&dir);
}

#[test]
fn generate_event_is_refused_outside_an_events_project() {
    let dir = temp_dir("events_refused_project");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("plain-project", &dir).unwrap();
    scaffold_producto(&output);

    let err = scaffold::run_generate_event("Producto", "producto_published", &output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(err.contains("--eventos"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_event_is_refused_for_an_entity_that_is_not_crud() {
    let (dir, output) = events_project("events_refused_simple");

    let err = scaffold::run_generate_event("Saludo", "saludo_waved", &output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(err.contains("no es una entidad CRUD"), "{err}");
    cleanup(&dir);
}

// ── puerto validar ─────────────────────────────────────────────────────

#[test]
fn validate_rejects_events_without_a_database() {
    let dir = write_manifest(
        "validate_events_no_db",
        "[proyecto]\nnombre = \"shop\"\neventos = \"db\"\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

#[test]
fn validate_rejects_an_unknown_transport() {
    let dir = write_manifest(
        "validate_events_transport",
        "[proyecto]\nnombre = \"shop\"\ndb = true\neventos = \"rabbit\"\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

#[test]
fn validate_accepts_a_database_project_with_events_and_custom_events() {
    let dir = write_manifest(
        "validate_events_ok",
        "[proyecto]\nnombre = \"shop\"\ndb = true\neventos = \"kafka\"\n\n[[entidad]]\nnombre = \"Producto\"\ncasos_de_uso = [\"crear_producto\"]\neventos = [\"producto_published\"]\n",
    );

    assert!(validate::run_validate(&dir).is_ok());
    cleanup(&dir);
}

#[test]
fn validate_rejects_custom_events_that_clash_or_repeat() {
    for (name, events) in [
        ("validate_events_crud", "[\"producto_creado\"]"),
        (
            "validate_events_dup",
            "[\"producto_published\", \"published\"]",
        ),
    ] {
        let dir = write_manifest(
            name,
            &format!(
                "[proyecto]\nnombre = \"shop\"\ndb = true\neventos = \"db\"\n\n[[entidad]]\nnombre = \"Producto\"\ncasos_de_uso = [\"crear_producto\"]\neventos = {events}\n"
            ),
        );
        assert!(validate::run_validate(&dir).is_err(), "{events}");
        cleanup(&dir);
    }
}

#[test]
fn validate_rejects_entity_events_in_a_project_without_events() {
    let dir = write_manifest(
        "validate_events_undeclared",
        "[proyecto]\nnombre = \"shop\"\ndb = true\n\n[[entidad]]\nnombre = \"Producto\"\ncasos_de_uso = [\"crear_producto\"]\neventos = [\"producto_published\"]\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

// ── Pure rules ──────────────────────────────────────────────────────────

#[test]
fn event_variants_drop_the_entity_prefix() {
    assert_eq!(event_variant("producto", "producto_published"), "Published");
    assert_eq!(event_variant("producto", "price_changed"), "PriceChanged");
    assert_eq!(
        event_variant("linea_pedido", "linea_pedido_shipped"),
        "Shipped"
    );
    // A prefix that is the whole name, or a longer word, is not stripped.
    assert_eq!(event_variant("producto", "producto"), "Producto");
    assert_eq!(
        event_variant("producto", "productos_merged"),
        "ProductosMerged"
    );
    assert_eq!(event_variant("producto", "producto_"), "Producto");
}

#[test]
fn crud_events_are_past_tense_and_named_after_the_entity() {
    assert_eq!(
        crud_events("linea_pedido"),
        [
            ("Creado".to_string(), "linea_pedido_creado".to_string()),
            (
                "Actualizado".to_string(),
                "linea_pedido_actualizado".to_string()
            ),
            ("Borrado".to_string(), "linea_pedido_borrado".to_string()),
        ]
    );
}

#[test]
fn an_entity_is_crud_only_with_all_five_scaffolded_use_cases() {
    let crud: Vec<String> = crate::generators::naming::crud_use_cases("producto");
    assert!(is_crud_entity("producto", &crud));
    assert!(!is_crud_entity("producto", &crud[..4]));
    assert!(!is_crud_entity("producto", &["obtener_saludo".to_string()]));
}

#[test]
fn event_names_are_checked_against_crud_and_declared_ones() {
    let declared = ["producto_published".to_string()];
    assert_eq!(
        validate_event_name("producto", "producto_archived", &declared),
        Ok(())
    );
    assert_eq!(
        validate_event_name("producto", "price_changed", &[]),
        Ok(())
    );
    for bad in [
        "producto_creado",
        "producto_actualizado",
        "producto_borrado",
        "creado",
        "producto_published",
        "published",
        "Published",
        "1_published",
        "self",
    ] {
        assert!(
            validate_event_name("producto", bad, &declared).is_err(),
            "{bad} should be refused"
        );
    }
}

#[test]
fn only_db_and_kafka_are_transports() {
    assert_eq!(validate_events_transport("db"), Ok(()));
    assert_eq!(validate_events_transport("kafka"), Ok(()));
    assert!(validate_events_transport("nats").is_err());
    assert!(validate_events_transport("").is_err());
}
