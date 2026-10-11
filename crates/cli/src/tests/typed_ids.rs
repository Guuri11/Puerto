// Phase 11.0 (T-24): every CRUD entity's id is its own type.
use super::*;

/// Scaffolds a fieldless CRUD `Product` and returns the project directory.
fn scaffold_crud_product(name: &str, db: bool) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
    }
    scaffold::run("Product", &dir, db, true, &[], &[]).unwrap();
    dir
}

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

#[test]
fn model_declares_an_id_type_for_the_entity() {
    let dir = scaffold_crud_product("typed_id_model", false);

    let model = read(&dir, "business/src/domain/product/model.rs");
    assert!(model.contains("pub struct ProductId(Uuid);"));
    assert!(model.contains("pub fn generate() -> Self"));
    assert!(model.contains("pub fn from_uuid(value: Uuid) -> Self"));
    assert!(model.contains("pub fn value(&self) -> Uuid"));
    assert!(model.contains("impl std::fmt::Display for ProductId"));
    assert!(model.contains("pub id: ProductId,"));
    assert!(model.contains("id: ProductId::generate(),"));

    cleanup(&dir);
}

#[test]
fn use_case_params_and_repository_take_the_id_type() {
    let dir = scaffold_crud_product("typed_id_ports", false);

    for uc in ["get_product", "update_product", "delete_product"] {
        let content = read(
            &dir,
            &format!("business/src/domain/product/use_cases/{uc}.rs"),
        );
        assert!(content.contains("pub id: ProductId,"), "{uc}");
        assert!(!content.contains("Uuid"), "{uc} still names Uuid");
    }
    let repository = read(&dir, "business/src/domain/product/repository.rs");
    assert!(repository.contains("async fn find_by_id(&self, id: ProductId)"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn application_tests_ask_for_a_missing_id_with_the_id_type() {
    let dir = scaffold_crud_product("typed_id_application", false);

    for uc in ["get_product", "update_product", "delete_product"] {
        let content = read(&dir, &format!("business/src/application/product/{uc}.rs"));
        assert!(content.contains("ProductId::generate()"), "{uc}");
        assert!(!content.contains("Uuid"), "{uc} still names Uuid");
    }

    cleanup(&dir);
}

#[test]
fn routes_and_dto_convert_at_the_edge() {
    let dir = scaffold_crud_product("typed_id_presentation", false);

    let routes = read(&dir, "presentation/src/api/product/routes.rs");
    assert!(routes.contains("ProductId::from_uuid(id.0)"));
    assert!(!routes.contains("{ id: id.0 }"));
    let dto = read(&dir, "presentation/src/api/product/dto.rs");
    assert!(dto.contains("pub id: Uuid,"));
    assert!(dto.contains("id: entity.id.value(),"));

    cleanup(&dir);
}

#[test]
fn postgres_row_keeps_a_uuid_and_converts() {
    let dir = scaffold_crud_product("typed_id_postgres", true);

    let entity = read(&dir, "infrastructure/src/product/entity.rs");
    assert!(entity.contains("pub id: Uuid,"));
    assert!(entity.contains("id: ProductId::from_uuid(row.id),"));
    assert!(entity.contains("id: entity.id.value(),"));
    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("async fn find_by_id(&self, id: ProductId)"));
    assert!(repository.contains("id.value()"));
    assert!(repository.contains("find_by_id(ProductId::generate())"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_takes_the_id_type() {
    let dir = scaffold_crud_product("typed_id_inmemory", false);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("async fn find_by_id(&self, id: ProductId)"));
    assert!(!repository.contains("Uuid"));

    cleanup(&dir);
}

#[test]
fn a_uuid_field_still_imports_uuid_next_to_the_id_type() {
    let dir = temp_dir("typed_id_uuid_field");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let fields = vec![scaffold_test_field("owner", "Uuid")];
    scaffold::run("Product", &dir, false, true, &fields, &[]).unwrap();

    let update = read(
        &dir,
        "business/src/domain/product/use_cases/update_product.rs",
    );
    assert!(update.contains("pub id: ProductId,"));
    assert!(update.contains("pub owner: Uuid,"));
    assert_eq!(update.matches("use uuid::Uuid;").count(), 1);

    cleanup(&dir);
}
