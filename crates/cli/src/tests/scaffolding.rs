use super::*;

// ── puerto generar scaffold ──────────────────────────────────────────────

#[test]
fn scaffold_creates_all_files_for_single_word_entity() {
    let dir = temp_dir("scaffold_single_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    assert!(dir.join("contextos/product/dominio/src/modelo.rs").exists());
    assert!(
        dir.join("contextos/product/dominio/src/errores.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/dominio/src/repositorio.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/infraestructura/src/repositorio.rs")
            .exists()
    );
    for layer in ["dominio", "aplicacion", "infraestructura", "rest"] {
        assert!(
            dir.join(format!("contextos/product/{layer}/Cargo.toml"))
                .exists(),
            "{layer}"
        );
        assert!(
            dir.join(format!("contextos/product/{layer}/src/lib.rs"))
                .exists(),
            "{layer}"
        );
    }
    assert!(dir.join("contextos/product/rest/src/dto.rs").exists());
    assert!(
        dir.join("contextos/product/rest/src/respuestas.rs")
            .exists()
    );
    assert!(dir.join("contextos/product/rest/src/errores.rs").exists());
    assert!(dir.join("contextos/product/rest/src/rutas.rs").exists());

    cleanup(&dir);
}

#[test]
fn scaffold_creates_all_files_for_multi_word_entity() {
    let dir = temp_dir("scaffold_multi_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("OrderItem", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/order_item/dominio/src/modelo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/order_item/aplicacion/src/create_order_item.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/order_item/aplicacion/src/create_order_item.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/order_item/infraestructura/src/repositorio.rs")
            .exists()
    );
    assert!(dir.join("contextos/order_item/rest/src/rutas.rs").exists());

    cleanup(&dir);
}

#[test]
fn scaffold_normalizes_lowercase_input() {
    let dir = temp_dir("scaffold_lowercase");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("product", &dir, false, &[], &[]).unwrap();

    assert!(dir.join("contextos/product/dominio/src/modelo.rs").exists());
    assert!(
        dir.join("contextos/product/aplicacion/src/create_product.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_model() {
    let dir = temp_dir("scaffold_model_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/product/dominio/src/modelo.rs")).unwrap();
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/product/dominio/src/errores.rs")).unwrap();
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/product/aplicacion/src/create_product.rs")).unwrap();
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    for uc in &[
        "get_product",
        "list_products",
        "update_product",
        "delete_product",
    ] {
        let path = dir.join(format!("contextos/product/aplicacion/src/{uc}.rs"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("modelo::{Product, ProductProps}")
                || content.contains("modelo::{Product, ProductId, ProductProps}"),
            "{uc}.rs test block missing the `modelo::{{Product, …ProductProps}}` import"
        );
    }

    cleanup(&dir);
}

// ── lib.rs auto-patching ──────────────────────────────────────────────────

/// The workspace skeleton the generators patch: the root `Cargo.toml` (members and
/// `[workspace.dependencies]`), `nucleo/`, the `api` binary's manifest — and a `puerto.toml` with
/// `project`.
pub(super) fn setup_workspace_stubs(base: &Path, project: &str) {
    fs::create_dir_all(base.join("nucleo/src")).unwrap();
    fs::write(
        base.join("Cargo.toml"),
        "[workspace]\nmembers = [\"nucleo\", \"contextos/*/*\", \"presentacion/*\"]\nresolver = \"3\"\n\n[workspace.dependencies]\npuerto-nucleo = \"0.20\"\nnucleo = { path = \"nucleo\" }\n",
    )
    .unwrap();
    fs::create_dir_all(base.join("presentacion/api")).unwrap();
    fs::write(
        base.join("presentacion/api/Cargo.toml"),
        "[package]\nname = \"api\"\n\n[dependencies]\ncableado = { workspace = true }\n",
    )
    .unwrap();
    fs::write(base.join("puerto.toml"), project).unwrap();
}

pub(super) fn setup_puerto_stubs(base: &Path) {
    setup_workspace_stubs(
        base,
        "[proyecto]\nnombre = \"test-app\"\n\n[[entidad]]\nnombre = \"Greeting\"\ncasos_de_uso = [\"get_greeting\"]\n",
    );
}

#[test]
fn scaffold_writes_a_flat_lib_rs_for_each_crate() {
    let dir = temp_dir("scaffold_flat_libs");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let lib = |layer: &str| {
        fs::read_to_string(dir.join(format!("contextos/product/{layer}/src/lib.rs"))).unwrap()
    };
    assert_eq!(
        lib("dominio"),
        "pub mod errores;\npub mod modelo;\npub mod repositorio;\n\n\
         #[cfg(any(test, feature = \"pruebas\"))]\npub mod pruebas {\n    pub mod madres;\n}\n"
    );
    assert_eq!(
        lib("aplicacion"),
        "pub mod create_product;\npub mod delete_product;\npub mod get_product;\n\
         pub mod list_products;\npub mod update_product;\n"
    );
    assert_eq!(lib("infraestructura"), "pub mod repositorio;\n");
    assert_eq!(
        lib("rest"),
        "pub mod dto;\npub mod errores;\npub mod respuestas;\npub mod rutas;\n"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_declares_the_context_crates_in_the_workspace() {
    let dir = temp_dir("scaffold_workspace_deps");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    for (name, path) in [
        ("dominio_product", "contextos/product/dominio"),
        ("aplicacion_product", "contextos/product/aplicacion"),
        ("infra_product", "contextos/product/infraestructura"),
        ("rest_product", "contextos/product/rest"),
    ] {
        assert!(
            manifest.contains(&format!("{name} = {{ path = \"{path}\" }}")),
            "{manifest}"
        );
    }

    cleanup(&dir);
}

#[test]
fn scaffold_gives_the_api_binary_the_context_rest_crate() {
    let dir = temp_dir("scaffold_api_deps");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let manifest = fs::read_to_string(dir.join("presentacion/api/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("rest_product = { workspace = true }"),
        "{manifest}"
    );
    assert!(
        manifest.contains("aplicacion_product = { workspace = true, features = [\"pruebas\"] }"),
        "the spec export test builds the API from the use cases' doubles:\n{manifest}"
    );
    let cableado = fs::read_to_string(dir.join("presentacion/cableado/Cargo.toml")).unwrap();
    assert!(
        cableado.contains("infra_product = { workspace = true }"),
        "{cableado}"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_updates_puerto_toml_and_regenerates_bootstrap() {
    let dir = temp_dir("scaffold_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let toml = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(toml.contains("nombre = \"Product\""));
    assert!(toml.contains("create_product"));
    assert!(toml.contains("nombre = \"Greeting\"")); // original still present

    let bootstrap = wiring(&dir);
    assert!(bootstrap.contains("CreateProductUseCaseImpl"));
    assert!(bootstrap.contains("InMemoryProductRepository"));
    assert!(bootstrap.contains("ProductApi"));
    assert!(bootstrap.contains("GetGreetingUseCaseImpl"));
    assert!(bootstrap.contains("InMemoryGreetingRepository"));
    assert!(bootstrap.contains("GreetingApi"));

    cleanup(&dir);
}

// ── puerto generar scaffold --db ─────────────────────────────────────────

fn setup_db_puerto_stubs(base: &Path) {
    setup_puerto_stubs(base);
    fs::create_dir_all(base.join("migraciones")).unwrap();
}

#[test]
fn scaffold_db_creates_entity_rs() {
    let dir = temp_dir("scaffold_db_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, &[], &[]).unwrap();
    assert!(
        dir.join("contextos/product/infraestructura/src/fila.rs")
            .exists()
    );
    cleanup(&dir);
}

#[test]
fn scaffold_db_repository_uses_pgpool() {
    let dir = temp_dir("scaffold_db_repo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Product", &dir, true, &[], &[]).unwrap();
    let content =
        fs::read_to_string(dir.join("contextos/product/infraestructura/src/repositorio.rs"))
            .unwrap();
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
    scaffold::run("Product", &dir, true, &[], &[]).unwrap();
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
    scaffold::run("Product", &dir, true, &[], &[]).unwrap();
    let content = wiring(&dir);
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();
    let content =
        fs::read_to_string(dir.join("contextos/product/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(content.contains("InMemoryProductRepository"));
    assert!(!content.contains("PgPool"));
    assert!(
        !dir.join("contextos/product/infraestructura/src/fila.rs")
            .exists()
    );
    cleanup(&dir);
}

// ── puerto generar scaffold --crud ──────────────────────────────────────

fn setup_puerto_stubs_for_crud(base: &Path) {
    setup_puerto_stubs(base);
}

#[test]
fn scaffold_crud_creates_all_domain_use_case_files() {
    let dir = temp_dir("scaffold_crud_domain_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/product/aplicacion/src/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/get_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/list_products.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/update_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/delete_product.rs")
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/product/aplicacion/src/create_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/get_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/list_products.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/update_product.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/product/aplicacion/src/delete_product.rs")
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/product/dominio/src/repositorio.rs")).unwrap();
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
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/product/rest/src/rutas.rs")).unwrap();
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
fn scaffold_crud_bootstrap_wires_all_use_cases() {
    let dir = temp_dir("scaffold_crud_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    fs::create_dir_all(dir.join("presentacion/api/src/generado")).unwrap();
    fs::write(
        dir.join("presentacion/api/src/generado/bootstrap.rs"),
        "// placeholder\n",
    )
    .unwrap();
    scaffold::run("Product", &dir, false, &[], &[]).unwrap();

    let content = wiring(&dir);
    assert!(content.contains("CreateProductUseCaseImpl"));
    assert!(content.contains("GetProductUseCaseImpl"));
    assert!(content.contains("ListProductsUseCaseImpl"));
    assert!(content.contains("UpdateProductUseCaseImpl"));
    assert!(content.contains("DeleteProductUseCaseImpl"));

    cleanup(&dir);
}

// ── Phase 7.4: scaffold infers db + always CRUD ───────────────────────────

pub(super) fn setup_puerto_stubs_with_project_db(base: &Path) {
    setup_workspace_stubs(
        base,
        "[proyecto]\nnombre = \"test-app\"\ndb = true\n\n[[entidad]]\nnombre = \"Greeting\"\ncasos_de_uso = [\"get_greeting\"]\n",
    );
}

#[test]
fn scaffold_7_4_infers_db_from_project_toml() {
    let dir = temp_dir("scaffold_74_infer_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    fs::create_dir_all(dir.join("migraciones")).unwrap();
    scaffold::run_scaffold("Team", &dir, &[], OverwritePolicy::Ask).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/team/infraestructura/src/repositorio.rs")).unwrap();
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

    let content =
        fs::read_to_string(dir.join("contextos/team/infraestructura/src/repositorio.rs")).unwrap();
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
        dir.join("contextos/team/aplicacion/src/create_team.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/team/aplicacion/src/get_team.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/team/aplicacion/src/list_teams.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/team/aplicacion/src/update_team.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/team/aplicacion/src/delete_team.rs")
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
        dir.join("migraciones").is_dir(),
        "run_migration should create migraciones/ when project.db = true"
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
        !dir.join("migraciones").exists(),
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

    let model_path = output.join("contextos/product/dominio/src/modelo.rs");
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
        err.contains("--forzar"),
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

    let model = fs::read_to_string(output.join("contextos/product/dominio/src/modelo.rs")).unwrap();
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
fn should_drop_the_value_objects_module_when_a_forced_rescaffold_has_no_local_value_object() {
    // Arrange: `Customer` first had a local `Email` VO — `value_objects.rs` names
    // `CustomerError::InvalidEmail`. Re-scaffolded without it, `errors.rs` loses the variant: a
    // `value_objects.rs` left behind (and still declared in lib.rs) no longer compiled.
    let dir = temp_dir("rescaffold_drops_vos");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("rescaffold-drops-vos", &dir).unwrap();
    let email = puerto_toml::parse_field_arg("email:Email:String").unwrap();
    scaffold::run_scaffold("Customer", &output, &[email], OverwritePolicy::Ask).unwrap();
    let value_objects = output.join("contextos/customer/dominio/src/valores.rs");
    assert!(value_objects.exists());

    // Act
    scaffold::run_scaffold(
        "Customer",
        &output,
        &[scaffold_test_field("full_name", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();

    // Assert
    assert!(
        !value_objects.exists(),
        "stale value_objects.rs left behind"
    );
    let lib = fs::read_to_string(output.join("contextos/customer/dominio/src/lib.rs")).unwrap();
    assert!(
        !lib.contains("pub mod valores;"),
        "lib.rs still declares the module:\n{lib}"
    );

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
    let src = dir.join("contextos");
    fs::create_dir_all(&src).unwrap();
    fs::set_permissions(&src, fs::Permissions::from_mode(0o555)).unwrap();

    // Act
    let result = scaffold::run_scaffold("Widget", &dir, &[], OverwritePolicy::Force);

    // Assert
    fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();
    let err = result
        .expect_err("a read-only project cannot be scaffolded")
        .to_string();
    assert!(
        err.contains("contextos/widget/dominio"),
        "error names the path: {err}"
    );
    cleanup(&dir);
}

#[test]
fn should_fail_naming_the_manifest_when_it_cannot_be_patched() {
    // `try_patch_libs` used to return a bool nobody read: a missing lib.rs still ended in
    // "✓ Done. Zero manual wiring." and a project whose new modules were never declared. The
    // manifests are what a scaffold patches now.

    // Arrange
    let dir = temp_dir("t11_missing_lib");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    fs::remove_file(dir.join("presentacion/api/Cargo.toml")).unwrap();

    // Act
    let result = scaffold::run_scaffold("Widget", &dir, &[], OverwritePolicy::Force);

    // Assert
    let err = result
        .expect_err("an unpatchable manifest is an error")
        .to_string();
    assert!(
        err.contains("presentacion/api/Cargo.toml"),
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
        "contextos/order_item/aplicacion/src/list_order_items.rs",
        "contextos/order_item/aplicacion/src/list_order_items.rs",
    ] {
        assert!(output.join(path).exists(), "{path}");
    }
    assert!(
        !output
            .join("contextos/order_item/aplicacion/src/list_order_item.rs")
            .exists()
    );

    let lib =
        fs::read_to_string(output.join("contextos/order_item/aplicacion/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("pub mod list_order_items;").count(), 1, "{lib}");

    let trait_src =
        fs::read_to_string(output.join("contextos/order_item/aplicacion/src/list_order_items.rs"))
            .unwrap();
    assert!(
        trait_src.contains("pub struct ListOrderItemsParams {"),
        "{trait_src}"
    );
    assert!(
        trait_src.contains("pub trait ListOrderItemsUseCaseTrait"),
        "{trait_src}"
    );

    let routes = fs::read_to_string(output.join("contextos/order_item/rest/src/rutas.rs")).unwrap();
    assert!(
        routes.contains("pub list_order_items: Arc<dyn ListOrderItemsUseCaseTrait>,"),
        "{routes}"
    );
    assert!(routes.contains("ListOrderItemsResponse"), "{routes}");

    let bootstrap = wiring(&output);
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
