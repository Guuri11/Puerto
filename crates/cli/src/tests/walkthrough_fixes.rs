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
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();

    let container = read(
        &output,
        "web/src/presentation/features/product/containers/product-list-container.tsx",
    );
    assert!(container.contains("const offset = offsetFromSearch(useSearch(), PAGE_SIZE);"));
    assert!(container.contains("navigate(location + pageSearch(next, PAGE_SIZE))"));
    assert!(!container.contains("useState"));
    let test = read(
        &output,
        "web/src/presentation/features/product/containers/product-list-container.test.tsx",
    );
    assert!(test.contains("const path = \"/products?page=2\";"));

    cleanup(&dir);
}

// ── `puerto list` shows the whole manifest ─────────────────────────────────

#[test]
fn list_shows_custom_events_consumers_and_shared_value_objects() {
    let config: puerto_toml::PuertoConfig = toml::from_str(
        r#"
[project]
name = "shop"
db = true
events = "db"

[[value_object]]
name = "Email"
type = "String"

[[entity]]
name = "Product"
use_cases = ["create_product", "publish_product"]
db = true
events = ["product_published"]

[[consumer]]
name = "notify_new_product"
events = ["product_created", "product_published"]
"#,
    )
    .unwrap();

    let report = crate::commands::list::list_report(&config);

    assert!(report.contains("Events: db"), "{report}");
    assert!(report.contains("    · publish_product"), "{report}");
    assert!(report.contains("    events: product_published"), "{report}");
    assert!(
        report.contains("Shared value objects:\n  Email (String)"),
        "{report}"
    );
    assert!(
        report.contains("Consumers:\n  notify_new_product ← product_created, product_published"),
        "{report}"
    );
}

#[test]
fn list_of_a_plain_project_has_no_empty_sections() {
    let config: puerto_toml::PuertoConfig = toml::from_str(
        "[project]\nname = \"shop\"\n\n[[entity]]\nname = \"Product\"\nuse_cases = [\"create_product\"]\n",
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

    scaffold::run_use_case("Product", "archive_product", &dir).unwrap();

    let impl_rs = read(&dir, "business/src/application/product/archive_product.rs");
    // The params are user data (architecture.md): the log names the use case only.
    assert!(!impl_rs.contains("?params"), "{impl_rs}");
    assert!(impl_rs.contains("tracing::info!(\"archive_product: start\");"));
    // An empty test body passed: `cargo test` said the use case was tested.
    assert!(
        impl_rs.contains("#[ignore = \"write this test\"]"),
        "{impl_rs}"
    );
    assert!(!impl_rs.contains("TODO: implement test body"));

    cleanup(&dir);
}
