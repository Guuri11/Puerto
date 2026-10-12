// Phase 11.1 (T-28): repositories speak one error, `RepositoryError`, whatever the entity.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

fn scaffold_product(name: &str, db: bool, crud: bool, fields: &[puerto_toml::Field]) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
    }
    scaffold::run("Product", &dir, db, crud, fields, &[]).unwrap();
    dir
}

#[test]
fn greeting_repository_returns_the_repository_error() {
    let dir = temp_dir("repo_error_greeting");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("repo-error-greeting", &dir).unwrap();

    let port = read(&output, "business/src/domain/greeting/repository.rs");
    assert!(port.contains("Result<Option<Greeting>, RepositoryError>"));
    let errors = read(&output, "business/src/domain/greeting/errors.rs");
    assert!(errors.contains("impl From<RepositoryError> for GreetingError"));

    cleanup(&dir);
}

#[test]
fn crud_repository_port_returns_the_repository_error() {
    let dir = scaffold_product("repo_error_crud_port", false, true, &[]);

    let port = read(&dir, "business/src/domain/product/repository.rs");
    assert!(port.contains("use puerto_core::{Page, PageRequest, RepositoryError};"));
    assert!(
        port.contains("async fn save(&self, entity: &Product) -> Result<(), RepositoryError>;")
    );
    assert!(port.contains("Result<Option<Product>, RepositoryError>"));
    assert!(!port.contains("ProductError"));

    cleanup(&dir);
}

#[test]
fn non_crud_repository_port_returns_the_repository_error() {
    let dir = scaffold_product("repo_error_simple_port", false, false, &[]);

    let port = read(&dir, "business/src/domain/product/repository.rs");
    assert!(port.contains("Result<Option<Product>, RepositoryError>"));
    assert!(!port.contains("ProductError"));

    cleanup(&dir);
}

#[test]
fn entity_error_converts_the_repository_error_and_tests_each_arm() {
    let dir = scaffold_product("repo_error_from", false, true, &[]);

    let errors = read(&dir, "business/src/domain/product/errors.rs");
    assert!(errors.contains("impl From<RepositoryError> for ProductError"));
    assert!(errors.contains("RepositoryError::Duplicate => Self::Duplicate,"));
    assert!(errors.contains(
        "RepositoryError::InvalidData | RepositoryError::Unavailable => Self::RepositoryError,"
    ));
    // `RepositoryError` is `#[non_exhaustive]` in puerto-core: a failure a later version adds is
    // the store's problem (500) until the project gives it a meaning.
    assert!(errors.contains("_ => Self::RepositoryError,"));
    assert!(errors.contains("fn should_convert_a_duplicate_into_the_entity_duplicate()"));
    assert!(errors.contains("fn should_convert_invalid_data_into_a_repository_error()"));
    assert!(errors.contains("fn should_convert_an_unavailable_store_into_a_repository_error()"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_returns_the_repository_error() {
    let dir = scaffold_product("repo_error_inmemory", false, true, &[]);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("RepositoryError"));
    assert!(!repository.contains("ProductError"));

    cleanup(&dir);
}

#[test]
fn postgres_repositories_map_driver_errors_in_one_place() {
    for (name, crud) in [
        ("repo_error_pg_crud", true),
        ("repo_error_pg_simple", false),
    ] {
        let dir = scaffold_product(name, true, crud, &[]);

        let repository = read(&dir, "infrastructure/src/product/repository.rs");
        assert!(repository.contains("map_sqlx_error"), "{name}");
        assert!(!repository.contains("23505"), "{name}");
        assert!(
            repository.contains("RepositoryError::InvalidData"),
            "{name}"
        );
        assert!(
            !repository.contains("ProductError::RepositoryError"),
            "{name}"
        );

        cleanup(&dir);
    }
}

#[test]
fn postgres_test_rejects_a_duplicate_when_a_field_is_unique() {
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;
    let dir = scaffold_product("repo_error_pg_unique", true, true, &[sku]);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("async fn should_reject_a_duplicate_sku(pool: PgPool)"));
    assert!(repository.contains("RepositoryError::Duplicate"));

    cleanup(&dir);
}

#[test]
fn postgres_test_has_no_duplicate_case_without_a_unique_field() {
    let dir = scaffold_product("repo_error_pg_no_unique", true, true, &[]);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(!repository.contains("should_reject_a_duplicate"));

    cleanup(&dir);
}

#[test]
fn shared_value_objects_are_declared_in_shared_rs_without_mod_rs() {
    let dir = temp_dir("repo_error_shared_vo");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let email = puerto_toml::Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    };
    let vos = [ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
    }];
    // Since 12.0 a project ships no `shared.rs`: the first shared VO creates it.
    assert!(!dir.join("business/src/domain/shared.rs").exists());

    scaffold::run(
        "User",
        &dir,
        false,
        true,
        std::slice::from_ref(&email),
        &vos,
    )
    .unwrap();
    scaffold::run("Account", &dir, false, true, &[email], &vos).unwrap();

    assert!(!dir.join("business/src/domain/shared/mod.rs").exists());
    let shared = read(&dir, "business/src/domain/shared.rs");
    assert!(!shared.contains("repository_error"));
    assert_eq!(shared.matches("pub mod errors;").count(), 1);
    assert_eq!(shared.matches("pub mod value_objects;").count(), 1);
    assert!(
        dir.join("business/src/domain/shared/value_objects.rs")
            .exists()
    );
    let lib = read(&dir, "business/src/lib.rs");
    assert_eq!(lib.matches("pub mod shared;").count(), 1);

    cleanup(&dir);
}
