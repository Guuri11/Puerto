// Phase 11.0 (T-24): every CRUD entity's id is its own type.
use super::*;

/// Scaffolds a fieldless CRUD `Producto` and returns the project directory.
fn scaffold_crud_producto(name: &str, db: bool) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("migraciones")).unwrap();
    }
    scaffold::run("Producto", &dir, db, &[], &[]).unwrap();
    dir
}

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

#[test]
fn model_declares_an_id_type_for_the_entity() {
    let dir = scaffold_crud_producto("typed_id_model", false);

    let model = read(&dir, "contextos/producto/dominio/src/modelo.rs");
    assert!(model.contains("pub struct IdProducto(Uuid);"));
    assert!(model.contains("pub fn generar() -> Self"));
    assert!(model.contains("pub fn from_uuid(valor: Uuid) -> Self"));
    assert!(model.contains("pub fn valor(&self) -> Uuid"));
    assert!(model.contains("impl std::fmt::Display for IdProducto"));
    assert!(model.contains("pub id: IdProducto,"));
    assert!(model.contains("id: IdProducto::generar(),"));

    cleanup(&dir);
}

#[test]
fn use_case_params_and_repository_take_the_id_type() {
    let dir = scaffold_crud_producto("typed_id_ports", false);

    for uc in ["obtener_producto", "actualizar_producto", "borrar_producto"] {
        let content = read(&dir, &format!("contextos/producto/aplicacion/src/{uc}.rs"));
        assert!(content.contains("pub id: IdProducto,"), "{uc}");
        assert!(!content.contains("Uuid"), "{uc} still names Uuid");
    }
    let repository = read(&dir, "contextos/producto/dominio/src/repositorio.rs");
    assert!(repository.contains("async fn buscar_por_id(&self, id: IdProducto)"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn application_tests_ask_for_a_missing_id_with_the_id_type() {
    let dir = scaffold_crud_producto("typed_id_application", false);

    for uc in ["obtener_producto", "actualizar_producto", "borrar_producto"] {
        let content = read(&dir, &format!("contextos/producto/aplicacion/src/{uc}.rs"));
        assert!(content.contains("IdProducto::generar()"), "{uc}");
        assert!(!content.contains("Uuid"), "{uc} still names Uuid");
    }

    cleanup(&dir);
}

#[test]
fn routes_and_dto_convert_at_the_edge() {
    let dir = scaffold_crud_producto("typed_id_presentation", false);

    let routes = read(&dir, "contextos/producto/rest/src/rutas.rs");
    assert!(routes.contains("IdProducto::from_uuid(id.0)"));
    assert!(!routes.contains("{ id: id.0 }"));
    let dto = read(&dir, "contextos/producto/rest/src/dto.rs");
    assert!(dto.contains("pub id: Uuid,"));
    assert!(dto.contains("id: entidad.id.valor(),"));

    cleanup(&dir);
}

#[test]
fn postgres_row_keeps_a_uuid_and_converts() {
    let dir = scaffold_crud_producto("typed_id_postgres", true);

    let entity = read(&dir, "contextos/producto/infraestructura/src/fila.rs");
    assert!(entity.contains("pub id: Uuid,"));
    assert!(entity.contains("id: IdProducto::from_uuid(fila.id),"));
    assert!(entity.contains("id: entidad.id.valor(),"));
    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("async fn buscar_por_id(&self, id: IdProducto)"));
    assert!(repository.contains("id.valor()"));
    assert!(repository.contains("buscar_por_id(IdProducto::generar())"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_takes_the_id_type() {
    let dir = scaffold_crud_producto("typed_id_inmemory", false);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("async fn buscar_por_id(&self, id: IdProducto)"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn a_uuid_field_still_imports_uuid_next_to_the_id_type() {
    let dir = temp_dir("typed_id_uuid_field");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let fields = vec![scaffold_test_field("owner", "Uuid")];
    scaffold::run("Producto", &dir, false, &fields, &[]).unwrap();

    let update = read(
        &dir,
        "contextos/producto/aplicacion/src/actualizar_producto.rs",
    );
    assert!(update.contains("pub id: IdProducto,"));
    assert!(update.contains("pub owner: Uuid,"));
    let top = &update[..update.find("#[cfg(test)]\nmod tests").unwrap()];
    assert_eq!(top.matches("use uuid::Uuid;").count(), 1, "{top}");

    cleanup(&dir);
}
