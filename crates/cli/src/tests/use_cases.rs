use super::*;

// ── puerto generar caso-de-uso ──────────────────────────────────────────────

pub(super) fn setup_use_case_stubs(base: &Path) {
    super::scaffolding::setup_workspace_stubs(
        base,
        "[proyecto]\nnombre = \"test-app\"\n\n[[entidad]]\nnombre = \"Producto\"\ncasos_de_uso = [\"crear_producto\"]\n",
    );
    fs::create_dir_all(base.join("contextos/producto/aplicacion/src")).unwrap();
    fs::write(
        base.join("contextos/producto/aplicacion/src/lib.rs"),
        "pub mod crear_producto;\n",
    )
    .unwrap();
}

#[test]
fn use_case_creates_trait_file() {
    let dir = temp_dir("uc_trait");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    assert!(
        dir.join("contextos/producto/aplicacion/src/borrar_producto.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn use_case_creates_impl_file() {
    let dir = temp_dir("uc_impl");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    assert!(
        dir.join("contextos/producto/aplicacion/src/borrar_producto.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn use_case_trait_file_has_correct_content() {
    let dir = temp_dir("uc_trait_content");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/borrar_producto.rs"))
            .unwrap();
    assert!(content.contains("ParamsBorrarProducto"));
    assert!(content.contains("BorrarProducto"));

    cleanup(&dir);
}

#[test]
fn use_case_impl_file_has_correct_content() {
    let dir = temp_dir("uc_impl_content");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/borrar_producto.rs"))
            .unwrap();
    assert!(content.contains("BorrarProductoImpl"));
    assert!(content.contains("RepositorioProducto"));
    assert!(content.contains("BorrarProducto"));

    cleanup(&dir);
}

#[test]
fn use_case_adds_one_line_to_the_application_lib_rs() {
    let dir = temp_dir("uc_patch_lib");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content = fs::read_to_string(dir.join("contextos/producto/aplicacion/src/lib.rs")).unwrap();
    assert_eq!(
        content,
        "pub mod crear_producto;\npub mod borrar_producto;\n"
    );

    cleanup(&dir);
}

#[test]
fn use_case_updates_puerto_toml() {
    let dir = temp_dir("uc_puerto_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(content.contains("borrar_producto"));
    assert!(content.contains("crear_producto")); // existing preserved

    cleanup(&dir);
}

#[test]
fn use_case_regenerates_bootstrap() {
    let dir = temp_dir("uc_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content = wiring(&dir);
    assert!(content.contains("BorrarProductoImpl"));
    assert!(content.contains("CrearProductoImpl"));

    cleanup(&dir);
}

#[test]
fn use_case_normalizes_entity_casing() {
    let dir = temp_dir("uc_normalize");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("producto", "borrar_producto", &dir).unwrap();

    assert!(
        dir.join("contextos/producto/aplicacion/src/borrar_producto.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn use_case_errors_when_entity_not_in_puerto_toml() {
    let dir = temp_dir("uc_missing_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);

    let result = scaffold::run_use_case("NonExistent", "do_something", &dir);
    assert!(result.is_err());

    cleanup(&dir);
}

#[test]
fn use_case_rerun_is_refused_and_leaves_files_untouched() {
    // Running the same `generate use-case` twice used to rewrite both files with the `todo!()`
    // stub and still print "added" — silently discarding whatever had been implemented since.
    let dir = temp_dir("uc_rerun_refused");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();
    let impl_path = dir.join("contextos/producto/aplicacion/src/borrar_producto.rs");
    fs::write(&impl_path, "// implemented by hand\n").unwrap();

    let result = scaffold::run_use_case("Producto", "borrar_producto", &dir);

    assert!(result.is_err(), "a second run must be refused");
    assert!(result.unwrap_err().to_string().contains("already exists"));
    assert_eq!(
        fs::read_to_string(&impl_path).unwrap(),
        "// implemented by hand\n"
    );
    let toml = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert_eq!(toml.matches("borrar_producto").count(), 1);

    cleanup(&dir);
}

// ── T-13: review fixes — generate use-case ────────────────────────────────

#[test]
fn use_case_rejects_an_action_that_is_not_snake_case() {
    let dir = temp_dir("uc_not_snake");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);

    let result = scaffold::run_use_case("Producto", "ArchiveProducto", &dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("snake_case"));
    assert!(
        !dir.join("contextos/producto/aplicacion/src/ArchiveProducto.rs")
            .exists()
    );
    assert!(
        !fs::read_to_string(dir.join("puerto.toml"))
            .unwrap()
            .contains("ArchiveProducto")
    );

    cleanup(&dir);
}

#[test]
fn use_case_refuses_to_replace_a_scaffolded_crud_use_case() {
    // `generate use-case Item create_item` replaced the working CRUD impl with a `todo!()` stub.
    let dir = temp_dir("uc_crud_clash");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("uc-crud-clash", &dir).unwrap();
    scaffold::run_scaffold(
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();
    let impl_path = output.join("contextos/producto/aplicacion/src/crear_producto.rs");
    let before = fs::read_to_string(&impl_path).unwrap();

    let result = scaffold::run_use_case("Producto", "crear_producto", &output);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
    assert_eq!(fs::read_to_string(&impl_path).unwrap(), before);

    cleanup(&dir);
}

#[test]
fn use_case_adds_its_field_to_the_entity_api_struct() {
    let dir = temp_dir("uc_routes_field");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("uc-routes-field", &dir).unwrap();
    scaffold::run_scaffold(
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();

    scaffold::run_use_case("Producto", "archive_producto", &output).unwrap();

    let routes = fs::read_to_string(output.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(routes.contains("pub archive_producto: Arc<dyn ArchiveProducto>,"));
    assert!(routes.contains("use aplicacion_producto::archive_producto::ArchiveProducto;"));
    assert!(!routes.contains("Impl {"), "{routes}");

    cleanup(&dir);
}

#[test]
fn rescaffold_with_force_keeps_custom_use_case_in_the_api_struct() {
    // The custom use case survives in puerto.toml (and so in bootstrap.rs); routes.rs is
    // rewritten from the CRUD template — it must get the field back or bootstrap stops compiling.
    let dir = temp_dir("uc_routes_force");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("uc-routes-force", &dir).unwrap();
    let fields = [scaffold_test_field("title", "String")];
    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Ask).unwrap();
    scaffold::run_use_case("Producto", "archive_producto", &output).unwrap();

    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Force).unwrap();

    let routes = fs::read_to_string(output.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert_eq!(
        routes
            .matches("pub archive_producto: Arc<dyn ArchiveProducto>,")
            .count(),
        1
    );

    cleanup(&dir);
}
