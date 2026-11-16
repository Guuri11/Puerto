use super::*;

// ── puerto generar scaffold ──────────────────────────────────────────────

#[test]
fn scaffold_creates_all_files_for_single_word_entity() {
    let dir = temp_dir("scaffold_single_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/producto/dominio/src/modelo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/dominio/src/errores.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/dominio/src/repositorio.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/crear_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/crear_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/infraestructura/src/repositorio.rs")
            .exists()
    );
    for layer in ["dominio", "aplicacion", "infraestructura", "rest"] {
        assert!(
            dir.join(format!("contextos/producto/{layer}/Cargo.toml"))
                .exists(),
            "{layer}"
        );
        assert!(
            dir.join(format!("contextos/producto/{layer}/src/lib.rs"))
                .exists(),
            "{layer}"
        );
    }
    assert!(dir.join("contextos/producto/rest/src/dto.rs").exists());
    assert!(
        dir.join("contextos/producto/rest/src/respuestas.rs")
            .exists()
    );
    assert!(dir.join("contextos/producto/rest/src/errores.rs").exists());
    assert!(dir.join("contextos/producto/rest/src/rutas.rs").exists());

    cleanup(&dir);
}

#[test]
fn scaffold_creates_all_files_for_multi_word_entity() {
    let dir = temp_dir("scaffold_multi_word");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("LineaPedido", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/linea_pedido/dominio/src/modelo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/linea_pedido/aplicacion/src/crear_linea_pedido.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/linea_pedido/aplicacion/src/crear_linea_pedido.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/linea_pedido/infraestructura/src/repositorio.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/linea_pedido/rest/src/rutas.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_normalizes_lowercase_input() {
    let dir = temp_dir("scaffold_lowercase");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("producto", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/producto/dominio/src/modelo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/crear_producto.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_model() {
    let dir = temp_dir("scaffold_model_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/producto/dominio/src/modelo.rs")).unwrap();
    assert!(content.contains("pub struct PropsProducto"));
    assert!(content.contains("pub struct Producto {"));
    assert!(content.contains("ErrorProducto::Validacion"));
    assert!(content.contains("producto.validacion.nombre_vacio"));

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_errors() {
    let dir = temp_dir("scaffold_errors_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/dominio/src/errores.rs")).unwrap();
    assert!(content.contains("pub enum ErrorProducto"));
    assert!(content.contains("producto.no_encontrado"));
    assert!(content.contains("producto.repositorio"));

    cleanup(&dir);
}

#[test]
fn scaffold_substitutes_pascal_name_in_use_case_impl() {
    let dir = temp_dir("scaffold_impl_content");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/crear_producto.rs"))
            .unwrap();
    assert!(!content.contains("Crear{Pascal}Impl"));
    assert!(content.contains("CrearProductoImpl"));
    assert!(content.contains("RepositorioProducto"));
    assert!(content.contains("producto.validacion.nombre_vacio"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_impls_import_model_struct_in_tests() {
    let dir = temp_dir("scaffold_model_import");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    for uc in &[
        "obtener_producto",
        "listar_productos",
        "actualizar_producto",
        "borrar_producto",
    ] {
        let path = dir.join(format!("contextos/producto/aplicacion/src/{uc}.rs"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("modelo::{Producto, PropsProducto}")
                || content.contains("modelo::{Producto, IdProducto, PropsProducto}"),
            "{uc}.rs test block missing the `modelo::{{Producto, …PropsProducto}}` import"
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
        "[proyecto]\nnombre = \"test-app\"\n\n[[entidad]]\nnombre = \"Saludo\"\ncasos_de_uso = [\"obtener_saludo\"]\n",
    );
}

#[test]
fn scaffold_writes_a_flat_lib_rs_for_each_crate() {
    let dir = temp_dir("scaffold_flat_libs");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let lib = |layer: &str| {
        fs::read_to_string(dir.join(format!("contextos/producto/{layer}/src/lib.rs"))).unwrap()
    };
    assert_eq!(
        lib("dominio"),
        "pub mod errores;\npub mod modelo;\npub mod repositorio;\n\n\
         #[cfg(any(test, feature = \"pruebas\"))]\npub mod pruebas {\n    pub mod madres;\n}\n"
    );
    assert_eq!(
        lib("aplicacion"),
        "pub mod actualizar_producto;\npub mod borrar_producto;\npub mod crear_producto;\n\
         pub mod listar_productos;\npub mod obtener_producto;\n"
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    for (name, path) in [
        ("dominio_producto", "contextos/producto/dominio"),
        ("aplicacion_producto", "contextos/producto/aplicacion"),
        ("infra_producto", "contextos/producto/infraestructura"),
        ("rest_producto", "contextos/producto/rest"),
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let manifest = fs::read_to_string(dir.join("presentacion/api/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("rest_producto = { workspace = true }"),
        "{manifest}"
    );
    assert!(
        manifest.contains("aplicacion_producto = { workspace = true, features = [\"pruebas\"] }"),
        "the spec export test builds the API from the use cases' doubles:\n{manifest}"
    );
    let cableado = fs::read_to_string(dir.join("presentacion/cableado/Cargo.toml")).unwrap();
    assert!(
        cableado.contains("infra_producto = { workspace = true }"),
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let toml = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(toml.contains("nombre = \"Producto\""));
    assert!(toml.contains("crear_producto"));
    assert!(toml.contains("nombre = \"Saludo\"")); // original still present

    let bootstrap = wiring(&dir);
    assert!(bootstrap.contains("CrearProductoImpl"));
    assert!(bootstrap.contains("RepositorioProductoEnMemoria"));
    assert!(bootstrap.contains("ApiProducto"));
    assert!(bootstrap.contains("ObtenerSaludoImpl"));
    assert!(bootstrap.contains("RepositorioSaludoEnMemoria"));
    assert!(bootstrap.contains("ApiSaludo"));

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
    scaffold::run("Producto", &dir, true, &[], &[]).unwrap();
    assert!(
        dir.join("contextos/producto/infraestructura/src/fila.rs")
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
    scaffold::run("Producto", &dir, true, &[], &[]).unwrap();
    let content =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(content.contains("PgPool"));
    assert!(content.contains("RepositorioProductoPostgres"));
    assert!(!content.contains("RepositorioProductoEnMemoria"));
    cleanup(&dir);
}

#[test]
fn scaffold_db_puerto_toml_has_db_true() {
    let dir = temp_dir("scaffold_db_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_db_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, true, &[], &[]).unwrap();
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
    scaffold::run("Producto", &dir, true, &[], &[]).unwrap();
    let content = wiring(&dir);
    assert!(content.contains("RepositorioProductoPostgres"));
    assert!(!content.contains("RepositorioProductoEnMemoria"));
    assert!(content.contains("RepositorioSaludoEnMemoria"));
    cleanup(&dir);
}

#[test]
fn scaffold_without_db_still_uses_inmemory() {
    let dir = temp_dir("scaffold_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();
    let content =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(content.contains("RepositorioProductoEnMemoria"));
    assert!(!content.contains("PgPool"));
    assert!(
        !dir.join("contextos/producto/infraestructura/src/fila.rs")
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/producto/aplicacion/src/crear_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/obtener_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/listar_productos.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/actualizar_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/borrar_producto.rs")
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    assert!(
        dir.join("contextos/producto/aplicacion/src/crear_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/obtener_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/listar_productos.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/actualizar_producto.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/producto/aplicacion/src/borrar_producto.rs")
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/dominio/src/repositorio.rs")).unwrap();
    assert!(content.contains("async fn buscar_todos"));
    assert!(content.contains("async fn buscar_por_id"));
    assert!(content.contains("async fn guardar"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_routes_has_all_http_methods() {
    let dir = temp_dir("scaffold_crud_routes");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_for_crud(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(content.contains("method = \"post\""));
    assert!(content.contains("method = \"get\""));
    assert!(content.contains("method = \"put\""));
    assert!(content.contains("method = \"delete\""));
    assert!(content.contains("pub struct ApiProducto"));
    assert!(content.contains("pub crear_producto"));
    assert!(content.contains("pub obtener_producto"));
    assert!(content.contains("pub listar_productos"));
    assert!(content.contains("pub actualizar_producto"));
    assert!(content.contains("pub borrar_producto"));

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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = wiring(&dir);
    assert!(content.contains("CrearProductoImpl"));
    assert!(content.contains("ObtenerProductoImpl"));
    assert!(content.contains("ListarProductosImpl"));
    assert!(content.contains("ActualizarProductoImpl"));
    assert!(content.contains("BorrarProductoImpl"));

    cleanup(&dir);
}

// ── Phase 7.4: scaffold infers db + always CRUD ───────────────────────────

pub(super) fn setup_puerto_stubs_with_project_db(base: &Path) {
    setup_workspace_stubs(
        base,
        "[proyecto]\nnombre = \"test-app\"\ndb = true\n\n[[entidad]]\nnombre = \"Saludo\"\ncasos_de_uso = [\"obtener_saludo\"]\n",
    );
}

#[test]
fn scaffold_7_4_infers_db_from_project_toml() {
    let dir = temp_dir("scaffold_74_infer_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    fs::create_dir_all(dir.join("migraciones")).unwrap();
    scaffold::run_scaffold("Equipo", &dir, &[], OverwritePolicy::Ask).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/equipo/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(content.contains("RepositorioEquipoPostgres"));
    assert!(!content.contains("RepositorioEquipoEnMemoria"));

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_non_db_project_uses_inmemory() {
    let dir = temp_dir("scaffold_74_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run_scaffold("Equipo", &dir, &[], OverwritePolicy::Ask).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/equipo/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(content.contains("RepositorioEquipoEnMemoria"));
    assert!(!content.contains("RepositorioEquipoPostgres"));

    cleanup(&dir);
}

#[test]
fn scaffold_7_4_always_generates_5_use_cases() {
    let dir = temp_dir("scaffold_74_5_uc");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run_scaffold("Equipo", &dir, &[], OverwritePolicy::Ask).unwrap();

    assert!(
        dir.join("contextos/equipo/aplicacion/src/crear_equipo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/equipo/aplicacion/src/obtener_equipo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/equipo/aplicacion/src/listar_equipos.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/equipo/aplicacion/src/actualizar_equipo.rs")
            .exists()
    );
    assert!(
        dir.join("contextos/equipo/aplicacion/src/borrar_equipo.rs")
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
    scaffold::run_scaffold("Equipo", &dir, &[], OverwritePolicy::Ask).unwrap();

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
    scaffold::run_scaffold("Equipo", &dir, &[], OverwritePolicy::Ask).unwrap();

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
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();

    let model_path = output.join("contextos/producto/dominio/src/modelo.rs");
    let hand_written = format!(
        "{}\n// hand-written business rule\n",
        fs::read_to_string(&model_path).unwrap()
    );
    fs::write(&model_path, &hand_written).unwrap();

    // Non-interactive re-scaffold must fail rather than silently discard the edit.
    let result = scaffold::run_scaffold(
        "Producto",
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
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();

    scaffold::run_scaffold(
        "Producto",
        &output,
        &[scaffold_test_field("headline", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();

    let model =
        fs::read_to_string(output.join("contextos/producto/dominio/src/modelo.rs")).unwrap();
    assert!(model.contains("pub headline: String"));
    assert!(!model.contains("pub title: String"));

    // The manifest must describe the code that is actually on disk.
    let config = puerto_toml::read(&output).unwrap();
    let producto = config
        .entity
        .iter()
        .find(|e| e.name == "Producto")
        .expect("Producto in puerto.toml");
    let field_names: Vec<&str> = producto.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["headline"]);

    cleanup(&dir);
}

#[test]
fn should_drop_the_value_objects_module_when_a_forced_rescaffold_has_no_local_value_object() {
    // Arrange: `Cliente` first had a local `Email` VO — `value_objects.rs` names
    // `ClienteError::InvalidEmail`. Re-scaffolded without it, `errors.rs` loses the variant: a
    // `value_objects.rs` left behind (and still declared in lib.rs) no longer compiled.
    let dir = temp_dir("rescaffold_drops_vos");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("rescaffold-drops-vos", &dir).unwrap();
    let email = puerto_toml::parse_field_arg("email:Email:String").unwrap();
    scaffold::run_scaffold("Cliente", &output, &[email], OverwritePolicy::Ask).unwrap();
    let value_objects = output.join("contextos/cliente/dominio/src/valores.rs");
    assert!(value_objects.exists());

    // Act
    scaffold::run_scaffold(
        "Cliente",
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
    let lib = fs::read_to_string(output.join("contextos/cliente/dominio/src/lib.rs")).unwrap();
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
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();
    scaffold::run_use_case("Producto", "archive_producto", &output).unwrap();

    scaffold::run_scaffold(
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();

    let config = puerto_toml::read(&output).unwrap();
    let producto = config
        .entity
        .iter()
        .find(|e| e.name == "Producto")
        .expect("Producto in puerto.toml");
    assert!(
        producto.use_cases.contains(&"archive_producto".to_string()),
        "a custom use case must survive a re-scaffold: {:?}",
        producto.use_cases
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
    let result = scaffold::run_scaffold("Artilugio", &dir, &[], OverwritePolicy::Force);

    // Assert
    fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();
    let err = result
        .expect_err("a read-only project cannot be scaffolded")
        .to_string();
    assert!(
        err.contains("contextos/artilugio/dominio"),
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
    let result = scaffold::run_scaffold("Artilugio", &dir, &[], OverwritePolicy::Force);

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
    scaffold::run_scaffold("LineaPedido", &output, &[], OverwritePolicy::Ask).unwrap();

    let manifest = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(manifest.contains("\"listar_lineas_pedido\""), "{manifest}");
    assert!(!manifest.contains("\"listar_linea_pedido\""), "{manifest}");
    for path in [
        "contextos/linea_pedido/aplicacion/src/listar_lineas_pedido.rs",
        "contextos/linea_pedido/aplicacion/src/listar_lineas_pedido.rs",
    ] {
        assert!(output.join(path).exists(), "{path}");
    }
    assert!(
        !output
            .join("contextos/linea_pedido/aplicacion/src/listar_linea_pedido.rs")
            .exists()
    );

    let lib =
        fs::read_to_string(output.join("contextos/linea_pedido/aplicacion/src/lib.rs")).unwrap();
    assert_eq!(
        lib.matches("pub mod listar_lineas_pedido;").count(),
        1,
        "{lib}"
    );

    let trait_src = fs::read_to_string(
        output.join("contextos/linea_pedido/aplicacion/src/listar_lineas_pedido.rs"),
    )
    .unwrap();
    assert!(
        trait_src.contains("pub struct ParamsListarLineasPedido {"),
        "{trait_src}"
    );
    assert!(
        trait_src.contains("pub trait ListarLineasPedido"),
        "{trait_src}"
    );

    let routes =
        fs::read_to_string(output.join("contextos/linea_pedido/rest/src/rutas.rs")).unwrap();
    assert!(
        routes.contains("pub listar_lineas_pedido: Arc<dyn ListarLineasPedido>,"),
        "{routes}"
    );
    assert!(routes.contains("RespuestaListarLineasPedido"), "{routes}");

    let bootstrap = wiring(&output);
    assert!(bootstrap.contains("ListarLineasPedidoImpl"), "{bootstrap}");
    assert!(!bootstrap.contains("ListarLineaPedidoImpl"), "{bootstrap}");

    cleanup(&dir);
}
