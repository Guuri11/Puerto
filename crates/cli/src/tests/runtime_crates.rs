// Phase 12.0 (ADR-0017): what every project shares lives in `puerto-core`, not in its template.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

/// The `major.minor` of the CLI under test: what a new project asks of `puerto-core`.
fn cli_minor() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut parts = version.split('.');
    format!("{}.{}", parts.next().unwrap(), parts.next().unwrap())
}

fn new_project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    (dir, output)
}

#[test]
fn new_project_depends_on_puerto_core_at_the_cli_minor() {
    let (dir, output) = new_project("runtime-deps");

    let root = read(&output, "Cargo.toml");
    assert!(root.contains("[workspace.dependencies]"));
    assert!(
        root.contains(&format!("puerto-core = \"{}\"", cli_minor())),
        "{root}"
    );
    assert!(!root.contains(crate::runtime::VERSION_MARKER));
    for member in ["business", "infrastructure", "presentation"] {
        let manifest = read(&output, &format!("{member}/Cargo.toml"));
        assert!(
            manifest.contains("puerto-core = { workspace = true }"),
            "{member}"
        );
    }

    cleanup(&dir);
}

#[test]
fn new_project_ships_no_copy_of_the_runtime_types() {
    let (dir, output) = new_project("runtime-no-shared");

    for path in [
        "business/src/domain/shared.rs",
        "business/src/domain/shared/pagination.rs",
        "business/src/domain/shared/repository_error.rs",
    ] {
        assert!(!output.join(path).exists(), "{path}");
    }
    assert!(!read(&output, "business/src/lib.rs").contains("pub mod shared;"));

    cleanup(&dir);
}

#[test]
fn no_demo_project_declares_no_shared_module() {
    let (dir, output) = new_project("runtime-no-demo");
    scaffold::apply_no_demo(&output).unwrap();

    assert_eq!(
        read(&output, "business/src/lib.rs"),
        "pub mod domain {\n}\npub mod application {\n}\n"
    );
    assert!(!output.join("business/src/domain/shared.rs").exists());

    cleanup(&dir);
}

#[test]
fn greeting_takes_the_repository_error_from_puerto_core() {
    let (dir, output) = new_project("runtime-greeting");

    for path in [
        "business/src/domain/greeting/errors.rs",
        "business/src/domain/greeting/repository.rs",
        "infrastructure/src/greeting/repository.rs",
    ] {
        let source = read(&output, path);
        assert!(
            source.contains("use puerto_core::RepositoryError;"),
            "{path}"
        );
        assert!(!source.contains("domain::shared"), "{path}");
    }

    cleanup(&dir);
}

#[test]
fn scaffold_imports_the_runtime_types_from_puerto_core() {
    for (name, db) in [
        ("runtime_scaffold_mem", false),
        ("runtime_scaffold_pg", true),
    ] {
        let dir = temp_dir(name);
        cleanup(&dir);
        setup_puerto_stubs(&dir);
        if db {
            fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
        }
        scaffold::run("Product", &dir, db, true, &[], &[]).unwrap();

        for (path, import) in [
            (
                "business/src/domain/product/errors.rs",
                "use puerto_core::RepositoryError;",
            ),
            (
                "business/src/domain/product/repository.rs",
                "use puerto_core::{Page, PageRequest, RepositoryError};",
            ),
            (
                "business/src/domain/product/use_cases/list_products.rs",
                "use puerto_core::{Page, PageRequest};",
            ),
            (
                "business/src/application/product/list_products.rs",
                "use puerto_core::Page;",
            ),
            (
                "infrastructure/src/product/repository.rs",
                "use puerto_core::{Page, PageRequest, RepositoryError};",
            ),
            (
                "presentation/src/api/product/routes.rs",
                "use puerto_core::PageRequest;",
            ),
            (
                "presentation/src/api/product/dto.rs",
                "use puerto_core::Page;",
            ),
        ] {
            let source = read(&dir, path);
            assert!(source.contains(import), "{name}: {path}");
            assert!(!source.contains("shared::pagination"), "{name}: {path}");
            assert!(
                !source.contains("shared::repository_error"),
                "{name}: {path}"
            );
        }

        cleanup(&dir);
    }
}

#[test]
fn non_crud_scaffold_imports_the_repository_error_from_puerto_core() {
    for (name, db) in [("runtime_simple_mem", false), ("runtime_simple_pg", true)] {
        let dir = temp_dir(name);
        cleanup(&dir);
        setup_puerto_stubs(&dir);
        if db {
            fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
        }
        scaffold::run("Product", &dir, db, false, &[], &[]).unwrap();

        for path in [
            "business/src/domain/product/repository.rs",
            "infrastructure/src/product/repository.rs",
        ] {
            let source = read(&dir, path);
            assert!(
                source.contains("use puerto_core::RepositoryError;"),
                "{name}: {path}"
            );
            assert!(!source.contains("domain::shared"), "{name}: {path}");
        }

        cleanup(&dir);
    }
}

#[test]
fn snippets_import_the_runtime_types_from_puerto_core() {
    let snippets = format!(
        "{}{}",
        crate::snippets::SNIPPETS_JSON,
        crate::snippets::SQL_SNIPPETS_JSON
    );
    assert!(!snippets.contains("shared::pagination"));
    assert!(!snippets.contains("shared::repository_error"));
    assert!(snippets.contains("use puerto_core::RepositoryError;"));
    assert!(snippets.contains("use puerto_core::{Page, PageRequest, RepositoryError};"));
}

// ── The rendered requirement and the version check ────────────────────────

#[test]
fn should_require_the_cli_minor_when_rendering_the_runtime_version() {
    assert_eq!(crate::runtime::requirement("0.14.0"), "0.14");
    assert_eq!(crate::runtime::requirement("0.14.3"), "0.14");
    assert_eq!(crate::runtime::requirement("1.2.0"), "1.2");
}

#[test]
fn should_replace_the_marker_with_the_requirement_when_rendering_the_manifest() {
    let manifest = format!(
        "[workspace.dependencies]\npuerto-core = \"{}\"\n",
        crate::runtime::VERSION_MARKER
    );
    assert_eq!(
        crate::runtime::render_manifest(&manifest, "0.14.2"),
        "[workspace.dependencies]\npuerto-core = \"0.14\"\n"
    );
}

#[test]
fn should_warn_when_a_runtime_crate_is_on_another_minor() {
    let manifest = "[workspace.dependencies]\npuerto-core = \"0.13\"\n";
    assert_eq!(
        crate::runtime::version_warnings(manifest, "0.14.0"),
        vec!["puerto-core 0.13 but puerto 0.14 — see the CHANGELOG migration guide".to_string()]
    );
}

#[test]
fn should_stay_silent_when_only_the_patch_differs() {
    let manifest = "[workspace.dependencies]\npuerto-core = \"0.14.1\"\n";
    assert!(crate::runtime::version_warnings(manifest, "0.14.3").is_empty());
}

#[test]
fn should_read_the_version_from_a_table_and_from_member_dependencies() {
    let manifest = "[dependencies]\npuerto-core = { version = \"0.12\" }\n";
    assert_eq!(
        crate::runtime::version_warnings(manifest, "0.14.0").len(),
        1
    );
}

#[test]
fn should_ignore_a_manifest_without_runtime_crates() {
    let manifest = "[workspace]\nmembers = [\"business\"]\n";
    assert!(crate::runtime::version_warnings(manifest, "0.14.0").is_empty());
}

#[test]
fn should_ignore_a_workspace_inherited_runtime_dependency() {
    let manifest = "[dependencies]\npuerto-core = { workspace = true }\n";
    assert!(crate::runtime::version_warnings(manifest, "0.14.0").is_empty());
}

#[test]
fn validate_passes_on_a_new_project() {
    let (dir, output) = new_project("runtime-validate");

    assert!(validate::run_validate(&output).is_ok());

    cleanup(&dir);
}

// ── Phase 12.1: `puerto-web` ──────────────────────────────────────────────

#[test]
fn new_project_depends_on_puerto_web_from_presentation_only() {
    let (dir, output) = new_project("runtime-web-deps");

    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains(&format!("puerto-web = \"{}\"", cli_minor())),
        "{root}"
    );
    let presentation = read(&output, "presentation/Cargo.toml");
    assert!(presentation.contains("puerto-web = { workspace = true }"));
    // The server, `.env` and the subscriber are the runtime's now.
    assert!(!presentation.contains("dotenvy"), "{presentation}");
    assert!(
        !presentation.contains("tracing-subscriber"),
        "{presentation}"
    );
    for member in ["business", "infrastructure"] {
        assert!(!read(&output, &format!("{member}/Cargo.toml")).contains("puerto-web"));
    }

    cleanup(&dir);
}

#[test]
fn new_project_main_hands_the_server_to_puerto_web() {
    let (dir, output) = new_project("runtime-web-main");

    let main = read(&output, "presentation/src/main.rs");
    assert!(main.contains("puerto_web::Config::from_env()?"), "{main}");
    assert!(main.contains("puerto_web::App::new(config)"), "{main}");
    assert!(
        main.contains(".serve(generated::bootstrap::build_app)"),
        "{main}"
    );
    assert!(main.contains("mod app;"), "{main}");
    for gone in [
        "run_with_graceful_shutdown",
        "tracing_subscriber",
        "SERVICE_PORT",
        "mod health;",
    ] {
        assert!(!main.contains(gone), "{gone}: {main}");
    }

    cleanup(&dir);
}

#[test]
fn new_project_ships_no_copy_of_the_http_shell() {
    let (dir, output) = new_project("runtime-web-no-copy");

    assert!(!output.join("presentation/src/health.rs").exists());
    // `IntoErrorResponse` stays: an `error_mapper.rs` implements it for an error of `business`,
    // which the orphan rule only allows for a trait of `presentation` itself.
    let error = read(&output, "presentation/src/api/error.rs");
    assert!(error.contains("pub trait IntoErrorResponse"), "{error}");
    assert!(error.contains("use puerto_web::ErrorResponse;"), "{error}");
    assert!(!error.contains("struct ErrorResponse"), "{error}");
    let greeting = format!(
        "{}{}",
        read(&output, "presentation/src/api/greeting/error_mapper.rs"),
        read(&output, "presentation/src/api/greeting/responses.rs")
    );
    assert!(!greeting.contains("error::ErrorResponse"), "{greeting}");
    assert!(greeting.contains("use crate::api::error::IntoErrorResponse;"));
    assert!(greeting.contains("use puerto_web::ErrorResponse;"));

    cleanup(&dir);
}

#[test]
fn bootstrap_builds_the_routes_with_puerto_web_and_hands_them_to_the_app_hook() {
    let (dir, output) = new_project("runtime-web-bootstrap");

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(
        bootstrap.contains("pub async fn build_app(config: puerto_web::Config) -> impl Endpoint"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("crate::app::configure(puerto_web::routes("),
        "{bootstrap}"
    );
    assert!(bootstrap.contains("puerto_web::health::always_ready()"));
    for gone in ["Cors", "std::env::var(\"SERVICE_PORT\")", "crate::health"] {
        assert!(!bootstrap.contains(gone), "{gone}: {bootstrap}");
    }

    cleanup(&dir);
}

#[test]
fn db_bootstrap_probes_the_database_through_puerto_web() {
    let dir = temp_dir("runtime_web_db_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("runtime-web-db".into()), true, &dir).unwrap();
    // The readiness check pings the database once an entity is stored in it.
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(
        bootstrap.contains("puerto_web::health::probe(move ||"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("infrastructure::db::ping(&pool)"),
        "{bootstrap}"
    );
    assert!(!bootstrap.contains("always_ready"), "{bootstrap}");

    cleanup(&dir);
}

#[test]
fn new_project_has_a_hand_written_app_hook() {
    let (dir, output) = new_project("runtime-web-app-hook");

    let app = read(&output, "presentation/src/app.rs");
    assert!(
        app.contains("pub fn configure(routes: Route) -> impl Endpoint"),
        "{app}"
    );
    assert!(!app.contains("AUTO-GENERATED"), "{app}");

    cleanup(&dir);
}

#[test]
fn app_hook_survives_scaffold_force_and_bootstrap_byte_for_byte() {
    let (dir, output) = new_project("runtime-web-app-survives");
    let hook = output.join("presentation/src/app.rs");
    let custom = "use poem::{Endpoint, EndpointExt, Route, middleware::SetHeader};\n\npub fn configure(routes: Route) -> impl Endpoint {\n    routes.with(SetHeader::new().overriding(\"x-team\", \"orders\"))\n}\n";
    fs::write(&hook, custom).unwrap();

    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Force).unwrap();
    crate::generators::bootstrap::run_bootstrap_command(&output, false).unwrap();

    assert_eq!(fs::read_to_string(&hook).unwrap(), custom);

    cleanup(&dir);
}

#[test]
fn scaffold_takes_the_error_body_from_puerto_web() {
    for (name, crud) in [("runtime_web_crud", true), ("runtime_web_simple", false)] {
        let dir = temp_dir(name);
        cleanup(&dir);
        setup_puerto_stubs(&dir);
        scaffold::run("Product", &dir, false, crud, &[], &[]).unwrap();

        for (path, import) in [
            (
                "presentation/src/api/product/error_mapper.rs",
                "use puerto_web::ErrorResponse;",
            ),
            (
                "presentation/src/api/product/responses.rs",
                "use puerto_web::ErrorResponse;",
            ),
        ] {
            let source = read(&dir, path);
            assert!(source.contains(import), "{name}: {path}\n{source}");
            assert!(!source.contains("error::ErrorResponse"), "{name}: {path}");
        }

        cleanup(&dir);
    }
}

#[test]
fn snippets_take_the_error_body_from_puerto_web() {
    let snippets = crate::snippets::SNIPPETS_JSON;
    assert!(!snippets.contains("error::ErrorResponse"));
    assert!(!snippets.contains("error::{ErrorResponse"));
    assert!(snippets.contains("use puerto_web::ErrorResponse;"));
}

#[test]
fn every_runtime_crate_is_patched_in_the_compile_matrix() {
    assert_eq!(
        crate::runtime::RUNTIME_CRATES,
        ["puerto-core", "puerto-web", "puerto-sqlx"]
    );
}

// ── Phase 12.2: `puerto-sqlx` ─────────────────────────────────────────────

fn new_db_project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some(name.into()), true, &dir).unwrap();
    (dir, output)
}

#[test]
fn db_project_depends_on_puerto_sqlx_from_infrastructure_only() {
    let (dir, output) = new_db_project("runtime-sqlx-deps");

    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains(&format!("puerto-sqlx = \"{}\"", cli_minor())),
        "{root}"
    );
    let infrastructure = read(&output, "infrastructure/Cargo.toml");
    assert!(
        infrastructure.contains("puerto-sqlx = { workspace = true }"),
        "{infrastructure}"
    );
    for member in ["business", "presentation"] {
        assert!(!read(&output, &format!("{member}/Cargo.toml")).contains("puerto-sqlx"));
    }

    cleanup(&dir);
}

#[test]
fn project_without_db_does_not_depend_on_puerto_sqlx() {
    let (dir, output) = new_project("runtime-sqlx-no-db");

    for path in [
        "Cargo.toml",
        "business/Cargo.toml",
        "infrastructure/Cargo.toml",
        "presentation/Cargo.toml",
    ] {
        assert!(!read(&output, path).contains("puerto-sqlx"), "{path}");
    }

    cleanup(&dir);
}

#[test]
fn db_module_keeps_only_the_project_migrations_and_re_exports_puerto_sqlx() {
    let (dir, output) = new_db_project("runtime-sqlx-db-rs");

    let db = read(&output, "infrastructure/src/db.rs");
    assert!(db.contains("pub use puerto_sqlx::*;"), "{db}");
    assert!(
        db.contains(
            "pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!(\"./migrations\");"
        ),
        "{db}"
    );
    // The pool, the probe and the error mapping are the runtime's — no code of their own here.
    for gone in ["fn ", "23505", "PgPoolOptions", "DatabaseError"] {
        assert!(!db.contains(gone), "{gone}: {db}");
    }

    cleanup(&dir);
}

#[test]
fn db_bootstrap_runs_the_project_migrations_through_puerto_sqlx() {
    let (dir, output) = new_db_project("runtime-sqlx-bootstrap");
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(
        bootstrap
            .contains("infrastructure::db::run_migrations(&pool, &infrastructure::db::MIGRATOR)"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("infrastructure::db::create_postgres_pool(&database_url)"),
        "{bootstrap}"
    );

    cleanup(&dir);
}

#[test]
fn compile_matrix_patches_only_the_runtime_crates_a_project_names() {
    let (dir, output) = new_project("runtime-sqlx-patch");
    super::compile_matrix::patch_runtime_crates(&output);

    let root = read(&output, "Cargo.toml");
    let patch = &root[root.find("[patch.crates-io]").unwrap()..];
    assert!(patch.contains("puerto-core = { path"), "{patch}");
    assert!(patch.contains("puerto-web = { path"), "{patch}");
    // An unused `[patch]` entry is a cargo warning in every build of a project without a database.
    assert!(!patch.contains("puerto-sqlx"), "{patch}");

    cleanup(&dir);
}

// Phase 13.2 (ADR-0018): the outbox tables `puerto new --events` writes are the ones
// `puerto-sqlx`'s own tests run against (`OUTBOX_MIGRATION` embeds that file).
#[test]
fn outbox_migration_template_is_exactly_the_one_puerto_sqlx_is_tested_with() {
    let template = include_str!("../templates/infrastructure/outbox.sql");
    let runtime = include_str!("../../../sqlx/migrations/outbox.sql");
    let runtime_lib = include_str!("../../../sqlx/src/lib.rs");

    assert_eq!(template, runtime);
    assert!(runtime_lib.contains(r#"include_str!("../migrations/outbox.sql")"#));
}
