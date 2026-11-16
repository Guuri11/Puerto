// Phase 11.1 (T-28): repositories speak one error, `ErrorRepositorio`, whatever the entity.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

fn scaffold_producto(name: &str, db: bool, _crud: bool, fields: &[puerto_toml::Field]) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("migraciones")).unwrap();
    }
    scaffold::run("Producto", &dir, db, fields, &[]).unwrap();
    dir
}

#[test]
fn saludo_repository_returns_the_repository_error() {
    let dir = temp_dir("repo_error_saludo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("repo-error-saludo", &dir).unwrap();

    let port = read(&output, "contextos/saludo/dominio/src/repositorio.rs");
    assert!(port.contains("Result<Option<Saludo>, ErrorRepositorio>"));
    let errors = read(&output, "contextos/saludo/dominio/src/errores.rs");
    assert!(errors.contains("impl From<ErrorRepositorio> for ErrorSaludo"));

    cleanup(&dir);
}

#[test]
fn crud_repository_port_returns_the_repository_error() {
    let dir = scaffold_producto("repo_error_crud_port", false, true, &[]);

    let port = read(&dir, "contextos/producto/dominio/src/repositorio.rs");
    assert!(port.contains("use puerto_nucleo::{Pagina, PeticionPagina, ErrorRepositorio};"));
    assert!(
        port.contains(
            "async fn guardar(&self, entidad: &Producto) -> Result<(), ErrorRepositorio>;"
        )
    );
    assert!(port.contains("Result<Option<Producto>, ErrorRepositorio>"));
    assert!(!port.contains("ErrorProducto"));

    cleanup(&dir);
}

#[test]
fn non_crud_repository_port_returns_the_repository_error() {
    let dir = scaffold_producto("repo_error_simple_port", false, false, &[]);

    let port = read(&dir, "contextos/producto/dominio/src/repositorio.rs");
    assert!(port.contains("Result<Option<Producto>, ErrorRepositorio>"));
    assert!(!port.contains("ErrorProducto"));

    cleanup(&dir);
}

#[test]
fn entity_error_converts_the_repository_error_and_tests_each_arm() {
    let dir = scaffold_producto("repo_error_from", false, true, &[]);

    let errors = read(&dir, "contextos/producto/dominio/src/errores.rs");
    assert!(errors.contains("impl From<ErrorRepositorio> for ErrorProducto"));
    assert!(errors.contains("ErrorRepositorio::Duplicado => Self::Duplicado,"));
    assert!(errors.contains(
        "ErrorRepositorio::DatosInvalidos | ErrorRepositorio::NoDisponible => Self::Repositorio,"
    ));
    // `ErrorRepositorio` is `#[non_exhaustive]` in puerto-nucleo: a failure a later version adds is
    // the store's problem (500) until the project gives it a meaning.
    assert!(errors.contains("_ => Self::Repositorio,"));
    assert!(errors.contains("fn deberia_convertir_un_duplicado_en_el_duplicado_de_la_entidad()"));
    assert!(errors.contains("fn deberia_convertir_datos_invalidos_en_un_error_de_repositorio()"));
    assert!(
        errors
            .contains("fn deberia_convertir_un_almacen_no_disponible_en_un_error_de_repositorio()")
    );

    cleanup(&dir);
}

#[test]
fn inmemory_repository_returns_the_repository_error() {
    let dir = scaffold_producto("repo_error_inmemory", false, true, &[]);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("ErrorRepositorio"));
    assert!(!repository.contains("ErrorProducto"));

    cleanup(&dir);
}

#[test]
fn postgres_repositories_map_driver_errors_in_one_place() {
    for (name, crud) in [
        ("repo_error_pg_crud", true),
        ("repo_error_pg_simple", false),
    ] {
        let dir = scaffold_producto(name, true, crud, &[]);

        let repository = read(
            &dir,
            "contextos/producto/infraestructura/src/repositorio.rs",
        );
        assert!(repository.contains("mapear_error_sqlx"), "{name}");
        assert!(!repository.contains("23505"), "{name}");
        assert!(
            repository.contains("ErrorRepositorio::DatosInvalidos"),
            "{name}"
        );
        assert!(!repository.contains("ErrorProducto::Repositorio"), "{name}");

        cleanup(&dir);
    }
}

#[test]
fn postgres_test_rejects_a_duplicate_when_a_field_is_unique() {
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;
    let dir = scaffold_producto("repo_error_pg_unique", true, true, &[sku]);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("async fn deberia_rechazar_un_duplicado_de_sku(pool: PgPool)"));
    assert!(repository.contains("ErrorRepositorio::Duplicado"));

    cleanup(&dir);
}

#[test]
fn postgres_test_has_no_duplicate_case_without_a_unique_field() {
    let dir = scaffold_producto("repo_error_pg_no_unique", true, true, &[]);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
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
    assert!(!dir.join("nucleo/src/lib.rs").exists());

    scaffold::run("User", &dir, false, std::slice::from_ref(&email), &vos).unwrap();
    scaffold::run("Account", &dir, false, &[email], &vos).unwrap();

    assert!(!dir.join("contextos/shared/dominio/src/mod.rs").exists());
    let shared = read(&dir, "nucleo/src/lib.rs");
    assert!(!shared.contains("repository_error"));
    assert_eq!(shared.matches("pub mod errores;").count(), 1);
    assert_eq!(shared.matches("pub mod valores;").count(), 1);
    assert!(dir.join("nucleo/src/valores.rs").exists());
    let manifest = read(&dir, "contextos/user/dominio/Cargo.toml");
    assert!(
        manifest.contains("nucleo = { workspace = true }"),
        "{manifest}"
    );

    cleanup(&dir);
}
