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
fn db_module_takes_the_repository_error_from_puerto_core() {
    let dir = temp_dir("runtime_db_rs");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("runtime-db".into()), true, &dir).unwrap();

    let db = read(&output, "infrastructure/src/db.rs");
    assert!(db.contains("use puerto_core::RepositoryError;"));
    assert!(!db.contains("domain::shared"));

    cleanup(&dir);
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
