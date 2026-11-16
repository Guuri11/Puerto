//! Phase 15.4: the smaller things the 2026-10-12 walkthrough found.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// ── The list's page lives in the URL ───────────────────────────────────────

#[test]
fn list_screen_reads_and_writes_its_page_in_the_url() {
    let dir = temp_dir("walkthrough_list_page");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("walkthrough-list-page", &dir).unwrap();
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();

    let container = read(
        &output,
        "web/src/presentation/features/producto/containers/producto-list-container.tsx",
    );
    assert!(container.contains("const offset = offsetFromSearch(useSearch(), PAGE_SIZE);"));
    assert!(container.contains("navigate(location + pageSearch(next, PAGE_SIZE))"));
    assert!(!container.contains("useState"));
    let test = read(
        &output,
        "web/src/presentation/features/producto/containers/producto-list-container.test.tsx",
    );
    assert!(test.contains("const path = \"/productos?page=2\";"));

    cleanup(&dir);
}

// ── `puerto listar` shows the whole manifest ─────────────────────────────────

#[test]
fn list_shows_custom_events_consumers_and_shared_value_objects() {
    let config: puerto_toml::PuertoConfig = toml::from_str(
        r#"
[proyecto]
nombre = "shop"
db = true
eventos = "db"

[[objeto_valor]]
nombre = "Email"
tipo = "String"

[[entidad]]
nombre = "Producto"
casos_de_uso = ["crear_producto", "publish_producto"]
db = true
eventos = ["producto_published"]

[[consumidor]]
nombre = "notify_new_producto"
eventos = ["producto_creado", "producto_published"]
"#,
    )
    .unwrap();

    let report = crate::commands::list::list_report(&config);

    assert!(report.contains("Events: db"), "{report}");
    assert!(report.contains("    · publish_producto"), "{report}");
    assert!(
        report.contains("    events: producto_published"),
        "{report}"
    );
    assert!(
        report.contains("Shared value objects:\n  Email (String)"),
        "{report}"
    );
    assert!(
        report.contains("Consumers:\n  notify_new_producto ← producto_creado, producto_published"),
        "{report}"
    );
}

#[test]
fn list_of_a_plain_project_has_no_empty_sections() {
    let config: puerto_toml::PuertoConfig = toml::from_str(
        "[proyecto]\nnombre = \"shop\"\n\n[[entidad]]\nnombre = \"Producto\"\ncasos_de_uso = [\"crear_producto\"]\n",
    )
    .unwrap();

    let report = crate::commands::list::list_report(&config);

    assert!(!report.contains("Consumers"), "{report}");
    assert!(!report.contains("Shared value objects"), "{report}");
    assert!(!report.contains("events:"), "{report}");
}

// ── `generate use-case` ─────────────────────────────────────────────────────

#[test]
fn generated_use_case_logs_no_payload_and_its_test_is_marked_to_write() {
    let dir = temp_dir("walkthrough_use_case");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);

    scaffold::run_use_case("Producto", "archive_producto", &dir).unwrap();

    let impl_rs = read(
        &dir,
        "contextos/producto/aplicacion/src/archive_producto.rs",
    );
    // The params are user data (architecture.md): the log names the use case only.
    assert!(!impl_rs.contains("?params"), "{impl_rs}");
    assert!(impl_rs.contains("tracing::info!(\"archive_producto: inicio\");"));
    // An empty test body passed: `cargo test` said the use case was tested.
    assert!(
        impl_rs.contains("#[ignore = \"escribe este test\"]"),
        "{impl_rs}"
    );
    assert!(!impl_rs.contains("TODO: implement test body"));

    cleanup(&dir);
}
