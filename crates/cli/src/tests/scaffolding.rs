use super::*;

// ── puerto generate scaffold ──────────────────────────────────────────────

#[test]
fn scaffold_creates_all_files_for_single_word_entity() {
    let dir = temp_dir("scaffold_single_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    assert!(dir.join("business/src/domain/product/model.rs").exists());
    assert!(dir.join("business/src/domain/product/errors.rs").exists());
    assert!(
        dir.join("business/src/domain/product/repository.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/product/use_cases/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/product/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("infrastructure/src/product/repository.rs")
            .exists()
    );
    assert!(dir.join("presentation/src/api/product.rs").exists());
    assert!(dir.join("presentation/src/api/product/dto.rs").exists());
    assert!(
        dir.join("presentation/src/api/product/responses.rs")
            .exists()
    );
    assert!(
        dir.join("presentation/src/api/product/error_mapper.rs")
            .exists()
    );
    assert!(dir.join("presentation/src/api/product/routes.rs").exists());

    cleanup(&dir);
}

#[test]
fn scaffold_creates_all_files_for_multi_word_entity() {
    let dir = temp_dir("scaffold_multi_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("OrderItem", &dir, false, false, &[], &[]).unwrap();

    assert!(dir.join("business/src/domain/order_item/model.rs").exists());
    assert!(
        dir.join("business/src/domain/order_item/use_cases/create_order_item.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/order_item/create_order_item.rs")
            .exists()
    );
    assert!(
        dir.join("infrastructure/src/order_item/repository.rs")
            .exists()
    );
    assert!(
        dir.join("presentation/src/api/order_item/routes.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_normalizes_lowercase_input() {
    let dir = temp_dir("scaffold_lowercase");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("product", &dir, false, false, &[], &[]).unwrap();

    assert!(dir.join("business/src/domain/product/model.rs").exists());
    assert!(
        dir.join("business/src/application/product/create_product.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_model() {
    let dir = temp_dir("scaffold_model_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/domain/product/model.rs")).unwrap();
    assert!(content.contains("pub struct ProductProps"));
    assert!(content.contains("pub struct Product {"));
    assert!(content.contains("ProductError::ValidationError"));
    assert!(content.contains("product.validation_error.name_empty"));

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_errors() {
    let dir = temp_dir("scaffold_errors_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/domain/product/errors.rs")).unwrap();
    assert!(content.contains("pub enum ProductError"));
    assert!(content.contains("product.not_found"));
    assert!(content.contains("product.repository_error"));

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_use_case_impl() {
    let dir = temp_dir("scaffold_impl_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("business/src/application/product/create_product.rs")).unwrap();
    assert!(!content.contains("Create{Pascal}UseCaseImpl"));
    assert!(content.contains("CreateProductUseCaseImpl"));
    assert!(content.contains("ProductRepositoryTrait"));
    assert!(content.contains("product.validation_error.name_empty"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_impls_import_model_struct_in_tests() {
    let dir = temp_dir("scaffold_model_import");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    for uc in &[
        "get_product",
        "list_products",
        "update_product",
        "delete_product",
    ] {
        let path = dir.join(format!("business/src/application/product/{uc}.rs"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("model::{Product, ProductProps}")
                || content.contains("model::{Product, ProductId, ProductProps}"),
            "{uc}.rs test block missing the `model::{{Product, …ProductProps}}` import"
        );
    }

    cleanup(&dir);
}

// ── lib.rs auto-patching ──────────────────────────────────────────────────

pub(super) fn setup_puerto_stubs(base: &Path) {
    fs::create_dir_all(base.join("business/src")).unwrap();
    fs::write(
        base.join("business/src/lib.rs"),
        "pub mod domain {\n  pub mod greeting {\n    pub mod errors;\n    pub mod model;\n    pub mod repository;\n    pub mod use_cases {\n      pub mod get_greeting;\n    }\n  }\n}\npub mod application {\n  pub mod greeting {\n    pub mod get_greeting;\n  }\n}\n",
    )
    .unwrap();

    fs::create_dir_all(base.join("infrastructure/src")).unwrap();
    fs::write(
        base.join("infrastructure/src/lib.rs"),
        "pub mod greeting {\n    pub mod repository;\n}\n",
    )
    .unwrap();

    fs::create_dir_all(base.join("presentation/src")).unwrap();
    fs::write(
        base.join("presentation/src/api.rs"),
        "pub mod error;\npub mod greeting;\n",
    )
    .unwrap();

    fs::write(
        base.join("puerto.toml"),
        "[project]\nname = \"test-app\"\n\n[[entity]]\nname = \"Greeting\"\nuse_cases = [\"get_greeting\"]\n",
    )
    .unwrap();
}

#[test]
fn scaffold_patches_business_lib_rs() {
    let dir = temp_dir("scaffold_patch_business");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
    assert!(content.contains("pub mod product {"));
    assert!(content.contains("pub mod errors;"));
    assert!(content.contains("pub mod use_cases {"));
    assert!(!content.contains("pub mod use_cases;")); // never the bare form
    assert!(content.contains("pub mod create_product;"));
    assert!(content.contains("pub mod greeting {"));

    cleanup(&dir);
}

#[test]
fn scaffold_patches_infra_lib_rs() {
    let dir = temp_dir("scaffold_patch_infra");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("infrastructure/src/lib.rs")).unwrap();
    assert!(content.contains("pub mod product {"));
    assert!(content.contains("pub mod repository;"));
    assert!(content.contains("pub mod greeting {"));

    cleanup(&dir);
}

#[test]
fn scaffold_patches_api_rs() {
    let dir = temp_dir("scaffold_patch_api");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("presentation/src/api.rs")).unwrap();
    assert!(content.contains("pub mod product;"));
    assert!(content.contains("pub mod error;"));
    assert!(content.contains("pub mod greeting;"));

    cleanup(&dir);
}

#[test]
fn scaffold_updates_puerto_toml_and_regenerates_bootstrap() {
    let dir = temp_dir("scaffold_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let toml = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(toml.contains("name = \"Product\""));
    assert!(toml.contains("create_product"));
    assert!(toml.contains("name = \"Greeting\"")); // original still present

    let bootstrap =
        fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(bootstrap.contains("CreateProductUseCaseImpl"));
    assert!(bootstrap.contains("InMemoryProductRepository"));
    assert!(bootstrap.contains("ProductApi"));
    assert!(bootstrap.contains("GetGreetingUseCaseImpl"));
    assert!(bootstrap.contains("InMemoryGreetingRepository"));
    assert!(bootstrap.contains("GreetingApi"));

    cleanup(&dir);
}

// ── puerto generate scaffold --db ─────────────────────────────────────────

fn setup_db_puerto_stubs(base: &Path) {
    setup_puerto_stubs(base);
    fs::create_dir_all(base.join("infrastructure/migrations")).unwrap();
}

#[test]
fn scaffold_db_creates_entity_rs() {
    let dir = temp_dir("scaffold_db_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, false, &[], &[]).unwrap();
    assert!(dir.join("infrastructure/src/product/entity.rs").exists());
    cleanup(&dir);
}

#[test]
fn scaffold_db_repository_uses_pgpool() {
    let dir = temp_dir("scaffold_db_repo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, false, &[], &[]).unwrap();
    let content = fs::read_to_string(dir.join("infrastructure/src/product/repository.rs")).unwrap();
    assert!(content.contains("PgPool"));
    assert!(content.contains("PgProductRepository"));
    assert!(!content.contains("InMemoryProductRepository"));
    cleanup(&dir);
}

#[test]
fn scaffold_db_puerto_toml_has_db_true() {
    let dir = temp_dir("scaffold_db_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, false, &[], &[]).unwrap();
    let content = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(content.contains("db = true"));
    cleanup(&dir);
}

#[test]
fn scaffold_db_bootstrap_uses_pg_repo() {
    let dir = temp_dir("scaffold_db_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, false, &[], &[]).unwrap();
    let content = fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(content.contains("PgProductRepository"));
    assert!(!content.contains("InMemoryProductRepository"));
    assert!(content.contains("InMemoryGreetingRepository"));
    cleanup(&dir);
}

#[test]
fn scaffold_without_db_still_uses_inmemory() {
    let dir = temp_dir("scaffold_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();
    let content = fs::read_to_string(dir.join("infrastructure/src/product/repository.rs")).unwrap();
    assert!(content.contains("InMemoryProductRepository"));
    assert!(!content.contains("PgPool"));
    assert!(!dir.join("infrastructure/src/product/entity.rs").exists());
    cleanup(&dir);
}

// ── puerto generate scaffold --crud ──────────────────────────────────────

fn setup_puerto_stubs_for_crud(base: &Path) {
    setup_puerto_stubs(base);
}

#[test]
fn scaffold_crud_creates_all_domain_use_case_files() {
    let dir = temp_dir("scaffold_crud_domain_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    assert!(
        dir.join("business/src/domain/product/use_cases/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/product/use_cases/get_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/product/use_cases/list_products.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/product/use_cases/update_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/product/use_cases/delete_product.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_crud_creates_all_application_use_case_files() {
    let dir = temp_dir("scaffold_crud_app_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    assert!(
        dir.join("business/src/application/product/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/product/get_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/product/list_products.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/product/update_product.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/application/product/delete_product.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_crud_repository_has_find_all() {
    let dir = temp_dir("scaffold_crud_repo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("business/src/domain/product/repository.rs")).unwrap();
    assert!(content.contains("async fn find_all"));
    assert!(content.contains("async fn find_by_id"));
    assert!(content.contains("async fn save"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_routes_has_all_http_methods() {
    let dir = temp_dir("scaffold_crud_routes");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap();
    assert!(content.contains("method = \"post\""));
    assert!(content.contains("method = \"get\""));
    assert!(content.contains("method = \"put\""));
    assert!(content.contains("method = \"delete\""));
    assert!(content.contains("pub struct ProductApi"));
    assert!(content.contains("pub create_product"));
    assert!(content.contains("pub get_product"));
    assert!(content.contains("pub list_products"));
    assert!(content.contains("pub update_product"));
    assert!(content.contains("pub delete_product"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_patches_business_lib_with_all_use_cases() {
    let dir = temp_dir("scaffold_crud_lib");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();

    // Verify domain use_cases block exists
    assert!(content.contains("pub mod use_cases {"));
    assert!(content.contains("pub mod create_product;"));
    assert!(content.contains("pub mod get_product;"));
    assert!(content.contains("pub mod list_products;"));
    assert!(content.contains("pub mod update_product;"));
    assert!(content.contains("pub mod delete_product;"));

    // Regression: verify modules appear inside the application block, not only in domain.
    // A location-agnostic contains() check passes even when the idempotency guard fires too
    // early and leaves the application block empty.
    let app_start = content
        .find("pub mod application {")
        .expect("application block missing");
    let app_block = &content[app_start..];
    assert!(
        app_block.contains("pub mod product {"),
        "application block missing pub mod product"
    );
    let product_in_app = app_block.find("pub mod product {").unwrap();
    let product_block = &app_block[product_in_app..];
    assert!(
        product_block.contains("pub mod create_product;"),
        "application.product missing create_product"
    );
    assert!(
        product_block.contains("pub mod get_product;"),
        "application.product missing get_product"
    );
    assert!(
        product_block.contains("pub mod list_products;"),
        "application.product missing list_products"
    );
    assert!(
        product_block.contains("pub mod update_product;"),
        "application.product missing update_product"
    );
    assert!(
        product_block.contains("pub mod delete_product;"),
        "application.product missing delete_product"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_crud_bootstrap_wires_all_use_cases() {
    let dir = temp_dir("scaffold_crud_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    fs::create_dir_all(dir.join("presentation/src/generated")).unwrap();
    fs::write(
        dir.join("presentation/src/generated/bootstrap.rs"),
        "// placeholder\n",
    )
    .unwrap();
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(content.contains("CreateProductUseCaseImpl"));
    assert!(content.contains("GetProductUseCaseImpl"));
    assert!(content.contains("ListProductsUseCaseImpl"));
    assert!(content.contains("UpdateProductUseCaseImpl"));
    assert!(content.contains("DeleteProductUseCaseImpl"));

    cleanup(&dir);
}

// ── Phase 7.4: scaffold infers db + always CRUD ───────────────────────────

pub(super) fn setup_puerto_stubs_with_project_db(base: &Path) {
    fs::create_dir_all(base.join("business/src")).unwrap();
    fs::write(
        base.join("business/src/lib.rs"),
        "pub mod domain {\n  pub mod greeting {\n    pub mod errors;\n    pub mod model;\n    pub mod repository;\n    pub mod use_cases {\n      pub mod get_greeting;\n    }\n  }\n}\npub mod application {\n  pub mod greeting {\n    pub mod get_greeting;\n  }\n}\n",
    ).unwrap();
    fs::create_dir_all(base.join("infrastructure/src")).unwrap();
    fs::write(
        base.join("infrastructure/src/lib.rs"),
        "pub mod greeting {\n    pub mod repository;\n}\n",
    )
    .unwrap();
    fs::create_dir_all(base.join("presentation/src")).unwrap();
    fs::write(
        base.join("presentation/src/api.rs"),
        "pub mod error;\npub mod greeting;\n",
    )
    .unwrap();
    fs::write(
        base.join("puerto.toml"),
        "[project]\nname = \"test-app\"\ndb = true\n\n[[entity]]\nname = \"Greeting\"\nuse_cases = [\"get_greeting\"]\n",
    ).unwrap();
}

#[test]
fn scaffold_7_4_infers_db_from_project_toml() {
    let dir = temp_dir("scaffold_74_infer_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    let content = fs::read_to_string(dir.join("infrastructure/src/team/repository.rs")).unwrap();
    assert!(content.contains("PgTeamRepository"));
    assert!(!content.contains("InMemoryTeamRepository"));

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_non_db_project_uses_inmemory() {
    let dir = temp_dir("scaffold_74_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    let content = fs::read_to_string(dir.join("infrastructure/src/team/repository.rs")).unwrap();
    assert!(content.contains("InMemoryTeamRepository"));
    assert!(!content.contains("PgTeamRepository"));

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_always_generates_5_use_cases() {
    let dir = temp_dir("scaffold_74_5_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    assert!(
        dir.join("business/src/domain/team/use_cases/create_team.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/team/use_cases/get_team.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/team/use_cases/list_teams.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/team/use_cases/update_team.rs")
            .exists()
    );
    assert!(
        dir.join("business/src/domain/team/use_cases/delete_team.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_db_project_auto_creates_migration() {
    let dir = temp_dir("scaffold_74_migration");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    assert!(
        dir.join("infrastructure/migrations").is_dir(),
        "run_migration should create infrastructure/migrations/ when project.db = true"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_non_db_project_does_not_create_migrations_dir() {
    let dir = temp_dir("scaffold_74_no_migration");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    assert!(
        !dir.join("infrastructure/migrations").exists(),
        "run_migration should NOT be called when project.db = false"
    );

    cleanup(&dir);
}

// ── T-04: scaffold does not silently overwrite ────────────────────────────

pub(super) fn scaffold_test_field(name: &str, ty: &str) -> puerto_toml::Field {
    puerto_toml::Field {
        name: name.to_string(),
        field_type: ty.to_string(),
        ..Default::default()
    }
}

#[test]
fn rescaffold_refuses_to_overwrite_and_writes_nothing() {
    let dir = temp_dir("rescaffold_refuses");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("rescaffold-refuses", &dir).unwrap();

    scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();

    let model_path = output.join("business/src/domain/product/model.rs");
    let hand_written = format!(
        "{}\n// hand-written business rule\n",
        fs::read_to_string(&model_path).unwrap()
    );
    fs::write(&model_path, &hand_written).unwrap();

    // Non-interactive re-scaffold must fail rather than silently discard the edit.
    let result = scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("other", "i64")],
        OverwritePolicy::Ask,
    );

    assert!(result.is_err(), "re-scaffold should refuse without --force");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("--force"),
        "error should point at --force: {err}"
    );
    assert_eq!(
        fs::read_to_string(&model_path).unwrap(),
        hand_written,
        "no file may be touched when the command is refused"
    );

    cleanup(&dir);
}

#[test]
fn rescaffold_with_force_overwrites_and_syncs_puerto_toml() {
    let dir = temp_dir("rescaffold_force");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("rescaffold-force", &dir).unwrap();

    scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();

    scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("headline", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();

    let model = fs::read_to_string(output.join("business/src/domain/product/model.rs")).unwrap();
    assert!(model.contains("pub headline: String"));
    assert!(!model.contains("pub title: String"));

    // The manifest must describe the code that is actually on disk.
    let config = puerto_toml::read(&output).unwrap();
    let product = config
        .entity
        .iter()
        .find(|e| e.name == "Product")
        .expect("Product in puerto.toml");
    let field_names: Vec<&str> = product.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["headline"]);

    cleanup(&dir);
}

#[test]
fn rescaffold_preserves_use_cases_added_by_generate_use_case() {
    let dir = temp_dir("rescaffold_keeps_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("rescaffold-keeps-uc", &dir).unwrap();

    scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();
    scaffold::run_use_case("Product", "archive_product", &output).unwrap();

    scaffold::run_scaffold(
        "Product",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();

    let config = puerto_toml::read(&output).unwrap();
    let product = config
        .entity
        .iter()
        .find(|e| e.name == "Product")
        .expect("Product in puerto.toml");
    assert!(
        product.use_cases.contains(&"archive_product".to_string()),
        "a custom use case must survive a re-scaffold: {:?}",
        product.use_cases
    );

    cleanup(&dir);
}

// ── T-11: generation failures surface, never a silent `✓ Done` ────────────

#[test]
fn should_fail_naming_the_file_when_the_project_is_read_only() {
    use std::os::unix::fs::PermissionsExt;

    // Arrange
    let dir = temp_dir("t11_read_only");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let src = dir.join("business/src");
    fs::set_permissions(&src, fs::Permissions::from_mode(0o555)).unwrap();

    // Act
    let result = scaffold::run_scaffold("Widget", &dir, &[], OverwritePolicy::Force);

    // Assert
    fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();
    let err = result
        .expect_err("a read-only project cannot be scaffolded")
        .to_string();
    assert!(
        err.contains("business/src/domain/widget"),
        "error names the path: {err}"
    );
    cleanup(&dir);
}

#[test]
fn should_fail_naming_the_lib_rs_when_it_cannot_be_patched() {
    // `try_patch_libs` used to return a bool nobody read: a missing lib.rs still ended in
    // "✓ Done. Zero manual wiring." and a project whose new modules were never declared.

    // Arrange
    let dir = temp_dir("t11_missing_lib");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    fs::remove_file(dir.join("infrastructure/src/lib.rs")).unwrap();

    // Act
    let result = scaffold::run_scaffold("Widget", &dir, &[], OverwritePolicy::Force);

    // Assert
    let err = result
        .expect_err("an unpatchable lib.rs is an error")
        .to_string();
    assert!(
        err.contains("infrastructure/src/lib.rs"),
        "error names the file: {err}"
    );
    cleanup(&dir);
}

#[test]
fn should_name_the_template_and_the_cause_when_a_render_fails() {
    // Arrange — model.tera needs `pascal`, which an empty context lacks.
    let ctx = tera::Context::new();

    // Act
    let err = crate::generators::render::render("domain/model.tera", &ctx)
        .expect_err("missing variables fail the render")
        .to_string();

    // Assert
    assert!(
        err.contains("domain/model.tera"),
        "names the template: {err}"
    );
    assert!(
        err.contains("not found in context"),
        "carries tera's cause: {err}"
    );
}

// ── T-20: the list use case is named after the plural ─────────────────────

#[test]
fn scaffold_names_the_list_use_case_after_the_plural_in_every_layer() {
    let dir = temp_dir("t20_list_plural");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("t20-list-plural", &dir).unwrap();
    scaffold::run_scaffold("OrderItem", &output, &[], OverwritePolicy::Ask).unwrap();

    let manifest = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(manifest.contains("\"list_order_items\""), "{manifest}");
    assert!(!manifest.contains("\"list_order_item\""), "{manifest}");
    for path in [
        "business/src/domain/order_item/use_cases/list_order_items.rs",
        "business/src/application/order_item/list_order_items.rs",
    ] {
        assert!(output.join(path).exists(), "{path}");
    }
    assert!(
        !output
            .join("business/src/domain/order_item/use_cases/list_order_item.rs")
            .exists()
    );

    let lib = fs::read_to_string(output.join("business/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod list_order_items;").count(), 2, "{lib}");

    let trait_src = fs::read_to_string(
        output.join("business/src/domain/order_item/use_cases/list_order_items.rs"),
    )
    .unwrap();
    assert!(
        trait_src.contains("pub struct ListOrderItemsParams {"),
        "{trait_src}"
    );
    assert!(
        trait_src.contains("pub trait ListOrderItemsUseCaseTrait"),
        "{trait_src}"
    );

    let routes =
        fs::read_to_string(output.join("presentation/src/api/order_item/routes.rs")).unwrap();
    assert!(
        routes.contains("pub list_order_items: Arc<dyn ListOrderItemsUseCaseTrait>,"),
        "{routes}"
    );
    assert!(routes.contains("ListOrderItemsResponse"), "{routes}");

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(
        bootstrap.contains("ListOrderItemsUseCaseImpl"),
        "{bootstrap}"
    );
    assert!(
        !bootstrap.contains("ListOrderItemUseCaseImpl"),
        "{bootstrap}"
    );

    cleanup(&dir);
}
