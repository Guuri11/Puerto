// Phase 13.3 (ADR-0018): `puerto new --db --events db` — units of work, `events.rs`, the outbox.
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

fn scaffold_product(output: &Path) {
    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "sku:String!", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Product", output, &fields, OverwritePolicy::Ask).unwrap();
}

fn write_manifest(name: &str, manifest: &str) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), manifest).unwrap();
    dir
}

// ── puerto new ───────────────────────────────────────────────────────────

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
    assert!(manifest.contains("events = \"db\""), "{manifest}");

    let migrations: Vec<String> = fs::read_dir(output.join("infrastructure/migrations"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let outbox = migrations
        .iter()
        .find(|m| m.ends_with("_outbox.sql"))
        .unwrap_or_else(|| panic!("no outbox migration in {migrations:?}"));
    let sql = read(&output, &format!("infrastructure/migrations/{outbox}"));
    assert!(sql.ends_with(include_str!("../templates/infrastructure/outbox.sql")));

    let business = read(&output, "business/Cargo.toml");
    assert!(business.contains("serde = { version = \"1\", features = [\"derive\"] }"));
    assert!(business.contains("serde_json = \"1\""));
    assert!(business.contains("features = [\"v4\", \"serde\"]"));
    cleanup(&dir);
}

#[test]
fn new_with_events_tells_agents_that_events_rs_is_never_hand_edited() {
    let (dir, output) = events_project("events_agents");

    let agents = read(&output, "AGENTS.md");
    assert!(
        agents.contains("Never hand-edit `business/src/domain/<entity>/events.rs`"),
        "{agents}"
    );
    assert!(agents.contains("puerto generate event"), "{agents}");
    // In front of the conventions, which stay.
    let events_at = agents.find("## Domain events").unwrap_or(usize::MAX);
    let conventions_at = agents.find("## Conventions").unwrap_or(0);
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

    scaffold_product(&output);

    let events = read(&output, "business/src/domain/product/events.rs");
    assert!(events.starts_with("// AUTO-GENERATED"));
    assert!(events.contains("pub struct ProductSnapshot"));
    assert!(events.contains("#[serde(rename_all = \"camelCase\")]"));
    assert!(events.contains("pub name: String,"), "VOs are unwrapped");
    for variant in ["Created", "Updated", "Deleted"] {
        assert!(events.contains(&format!("    {variant}(ProductSnapshot),")));
    }
    assert!(events.contains("pub const PRODUCT_CREATED: &str = \"product_created\";"));
    assert!(events.contains("impl DomainEvent for ProductEvent"));

    let uow = read(&output, "business/src/domain/product/unit_of_work.rs");
    assert!(uow.contains("pub trait ProductUnitOfWork: Send"));
    assert!(uow.contains("pub trait ProductUnitOfWorkFactory: Send + Sync"));
    assert!(uow.contains("pub struct RecordingUnitOfWorkFactory"));

    let pg_uow = read(&output, "infrastructure/src/product/unit_of_work.rs");
    assert!(pg_uow.contains("pub struct PgProductUnitOfWorkFactory"));
    assert!(pg_uow.contains("ON CONFLICT (id) DO UPDATE"));
    assert!(pg_uow.contains("use puerto_sqlx::uow::PgTransaction;"));

    let lib = read(&output, "business/src/lib.rs");
    assert!(lib.contains("pub mod events;"));
    assert!(lib.contains("pub mod unit_of_work;"));
    let infra_lib = read(&output, "infrastructure/src/lib.rs");
    assert!(
        infra_lib.contains("    pub mod unit_of_work;\n"),
        "{infra_lib}"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_leaves_the_repository_read_only() {
    let (dir, output) = events_project("events_read_only");

    scaffold_product(&output);

    let port = read(&output, "business/src/domain/product/repository.rs");
    assert!(!port.contains("async fn save"), "{port}");
    assert!(port.contains("async fn find_by_id"));
    let pg = read(&output, "infrastructure/src/product/repository.rs");
    assert!(!pg.contains("async fn save"), "{pg}");
    assert!(
        !pg.contains("INSERT INTO"),
        "the upsert moved to the unit of work"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_writes_through_the_unit_of_work() {
    let (dir, output) = events_project("events_writes");

    scaffold_product(&output);

    for (action, event) in [
        ("create", "Created"),
        ("update", "Updated"),
        ("delete", "Deleted"),
    ] {
        let uc = read(
            &output,
            &format!("business/src/application/product/{action}_product.rs"),
        );
        assert!(
            uc.contains("pub uow_factory: Arc<dyn ProductUnitOfWorkFactory>,"),
            "{action}"
        );
        assert!(uc.contains("self.uow_factory.begin()"), "{action}");
        assert!(
            uc.contains(&format!("uow.record(ProductEvent::{event}(")),
            "{action}"
        );
        assert!(uc.contains("uow.commit()"), "{action}");
        assert!(!uc.contains("repository.save"), "{action}");
    }
    let create = read(
        &output,
        "business/src/application/product/create_product.rs",
    );
    assert!(!create.contains("pub repository:"), "create only writes");
    assert!(create.contains("fn should_record_product_created_when_fields_are_valid()"));
    assert!(create.contains("fn should_not_commit_when_the_store_rejects_a_duplicate()"));
    let update = read(
        &output,
        "business/src/application/product/update_product.rs",
    );
    assert!(update.contains("pub repository: Arc<dyn ProductRepositoryTrait>,"));
    assert!(update.contains(
        "fn should_return_not_found_and_record_nothing_when_the_product_does_not_exist()"
    ));
    let get = read(&output, "business/src/application/product/get_product.rs");
    assert!(
        !get.contains("uow_factory"),
        "reads do not open a unit of work"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_events_project_wires_the_unit_of_work_in_bootstrap() {
    let (dir, output) = events_project("events_bootstrap");

    scaffold_product(&output);

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(bootstrap.contains(
        "let product_uow: Arc<dyn ProductUnitOfWorkFactory> = Arc::new(PgProductUnitOfWorkFactory::new(pool.clone()));"
    ));
    assert!(bootstrap.contains(
        "let create_product = Arc::new(CreateProductUseCaseImpl { uow_factory: Arc::clone(&product_uow) });"
    ));
    assert!(bootstrap.contains(
        "let get_product = Arc::new(GetProductUseCaseImpl { repository: Arc::clone(&product_repo) });"
    ));
    assert!(bootstrap.contains(
        "let update_product = Arc::new(UpdateProductUseCaseImpl { repository: Arc::clone(&product_repo), uow_factory: Arc::clone(&product_uow) });"
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

    scaffold_product(&output);

    assert!(
        !output
            .join("business/src/domain/product/events.rs")
            .exists()
    );
    assert!(
        !output
            .join("business/src/domain/product/unit_of_work.rs")
            .exists()
    );
    assert!(
        !output
            .join("infrastructure/src/product/unit_of_work.rs")
            .exists()
    );
    assert!(read(&output, "business/src/domain/product/repository.rs").contains("async fn save"));
    cleanup(&dir);
}

#[test]
fn the_demo_entity_keeps_its_repository_in_an_events_project() {
    // A simple (non-CRUD) entity has no writes to record: events are a CRUD-write concern.
    let (dir, output) = events_project("events_demo");

    assert!(
        !output
            .join("business/src/domain/greeting/events.rs")
            .exists()
    );
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(!bootstrap.contains("UnitOfWork"), "{bootstrap}");
    cleanup(&dir);
}

// ── generate use-case / generate event ──────────────────────────────────

#[test]
fn custom_use_case_in_an_events_project_gets_the_repository_and_the_unit_of_work() {
    let (dir, output) = events_project("events_use_case");
    scaffold_product(&output);

    scaffold::run_use_case("Product", "publish_product", &output).unwrap();

    let uc = read(
        &output,
        "business/src/application/product/publish_product.rs",
    );
    assert!(uc.contains("pub repository: Arc<dyn ProductRepositoryTrait>,"));
    assert!(uc.contains("pub uow_factory: Arc<dyn ProductUnitOfWorkFactory>,"));
    assert!(uc.contains("Writes go through the unit of work"));
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(bootstrap.contains(
        "let publish_product = Arc::new(PublishProductUseCaseImpl { repository: Arc::clone(&product_repo), uow_factory: Arc::clone(&product_uow) });"
    ));
    cleanup(&dir);
}

#[test]
fn generate_event_declares_it_and_regenerates_events_rs() {
    let (dir, output) = events_project("events_generate");
    scaffold_product(&output);

    scaffold::run_generate_event("Product", "product_published", &output).unwrap();

    let config = puerto_toml::read(&output).unwrap();
    let product = config.entity.iter().find(|e| e.name == "Product").unwrap();
    assert_eq!(product.events, ["product_published"]);
    let events = read(&output, "business/src/domain/product/events.rs");
    assert!(events.contains("    Published(ProductSnapshot),"));
    assert!(events.contains("Self::Published(_) => PRODUCT_PUBLISHED,"));
    // No use case records it until one is written to.
    let create = read(
        &output,
        "business/src/application/product/create_product.rs",
    );
    assert!(!create.contains("Published"));
    cleanup(&dir);
}

#[test]
fn custom_events_survive_a_forced_rescaffold_and_a_bootstrap() {
    let (dir, output) = events_project("events_survive");
    scaffold_product(&output);
    scaffold::run_generate_event("Product", "product_published", &output).unwrap();

    let fields: Vec<puerto_toml::Field> = ["name:Name:String", "sku:String!", "price:i64"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold("Product", &output, &fields, OverwritePolicy::Force).unwrap();
    scaffold::run_bootstrap_command(&output, false).unwrap();

    let events = read(&output, "business/src/domain/product/events.rs");
    assert!(events.contains("    Published(ProductSnapshot),"));
    cleanup(&dir);
}

#[test]
fn generate_event_refuses_a_crud_event_a_duplicate_and_a_bad_name() {
    let (dir, output) = events_project("events_refuse");
    scaffold_product(&output);
    scaffold::run_generate_event("Product", "product_published", &output).unwrap();

    for (event, expected) in [
        ("product_created", "clashes"),
        ("deleted", "clashes"),
        ("product_published", "already declared"),
        ("published", "already declared"),
        ("ProductPublished", "snake_case"),
    ] {
        let err = scaffold::run_generate_event("Product", event, &output)
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
            .find(|e| e.name == "Product")
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
    scaffold_product(&output);

    let err = scaffold::run_generate_event("Product", "product_published", &output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(err.contains("--events"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_event_is_refused_for_an_entity_that_is_not_crud() {
    let (dir, output) = events_project("events_refused_simple");

    let err = scaffold::run_generate_event("Greeting", "greeting_waved", &output)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    assert!(err.contains("not a CRUD entity"), "{err}");
    cleanup(&dir);
}

// ── puerto validate ─────────────────────────────────────────────────────

#[test]
fn validate_rejects_events_without_a_database() {
    let dir = write_manifest(
        "validate_events_no_db",
        "[project]\nname = \"shop\"\nevents = \"db\"\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

#[test]
fn validate_rejects_an_unknown_transport() {
    let dir = write_manifest(
        "validate_events_transport",
        "[project]\nname = \"shop\"\ndb = true\nevents = \"rabbit\"\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

#[test]
fn validate_accepts_a_database_project_with_events_and_custom_events() {
    let dir = write_manifest(
        "validate_events_ok",
        "[project]\nname = \"shop\"\ndb = true\nevents = \"kafka\"\n\n[[entity]]\nname = \"Product\"\nuse_cases = [\"create_product\"]\nevents = [\"product_published\"]\n",
    );

    assert!(validate::run_validate(&dir).is_ok());
    cleanup(&dir);
}

#[test]
fn validate_rejects_custom_events_that_clash_or_repeat() {
    for (name, events) in [
        ("validate_events_crud", "[\"product_created\"]"),
        (
            "validate_events_dup",
            "[\"product_published\", \"published\"]",
        ),
    ] {
        let dir = write_manifest(
            name,
            &format!(
                "[project]\nname = \"shop\"\ndb = true\nevents = \"db\"\n\n[[entity]]\nname = \"Product\"\nuse_cases = [\"create_product\"]\nevents = {events}\n"
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
        "[project]\nname = \"shop\"\ndb = true\n\n[[entity]]\nname = \"Product\"\nuse_cases = [\"create_product\"]\nevents = [\"product_published\"]\n",
    );

    assert!(validate::run_validate(&dir).is_err());
    cleanup(&dir);
}

// ── Pure rules ──────────────────────────────────────────────────────────

#[test]
fn event_variants_drop_the_entity_prefix() {
    assert_eq!(event_variant("product", "product_published"), "Published");
    assert_eq!(event_variant("product", "price_changed"), "PriceChanged");
    assert_eq!(event_variant("order_item", "order_item_shipped"), "Shipped");
    // A prefix that is the whole name, or a longer word, is not stripped.
    assert_eq!(event_variant("product", "product"), "Product");
    assert_eq!(
        event_variant("product", "products_merged"),
        "ProductsMerged"
    );
    assert_eq!(event_variant("product", "product_"), "Product");
}

#[test]
fn crud_events_are_past_tense_and_named_after_the_entity() {
    assert_eq!(
        crud_events("order_item"),
        [
            ("Created".to_string(), "order_item_created".to_string()),
            ("Updated".to_string(), "order_item_updated".to_string()),
            ("Deleted".to_string(), "order_item_deleted".to_string()),
        ]
    );
}

#[test]
fn an_entity_is_crud_only_with_all_five_scaffolded_use_cases() {
    let crud: Vec<String> = crate::generators::naming::crud_use_cases("product");
    assert!(is_crud_entity("product", &crud));
    assert!(!is_crud_entity("product", &crud[..4]));
    assert!(!is_crud_entity("product", &["get_greeting".to_string()]));
}

#[test]
fn event_names_are_checked_against_crud_and_declared_ones() {
    let declared = ["product_published".to_string()];
    assert_eq!(
        validate_event_name("product", "product_archived", &declared),
        Ok(())
    );
    assert_eq!(validate_event_name("product", "price_changed", &[]), Ok(()));
    for bad in [
        "product_created",
        "product_updated",
        "product_deleted",
        "created",
        "product_published",
        "published",
        "Published",
        "1_published",
        "self",
    ] {
        assert!(
            validate_event_name("product", bad, &declared).is_err(),
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
