//! Phase 15.0: without `--db`, `InMemory{Entity}Repository` keeps what it is given, so the
//! default project's create → detail flow finds what it just created.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn scaffold_inmemory_product(name: &str, args: &[&str]) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let fields: Vec<puerto_toml::Field> = args
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run("Product", &dir, false, true, &fields, &[]).unwrap();
    dir
}

#[test]
fn inmemory_repository_keeps_rows_in_a_map_keyed_by_id() {
    let dir = scaffold_inmemory_product("inmemory_map", &[]);

    let repo = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repo.contains("rows: RwLock<HashMap<ProductId, Product>>,"));
    assert!(repo.contains("#[derive(Default)]\npub struct InMemoryProductRepository {"));
    assert!(repo.contains("pub fn new() -> Self {"));
    assert!(repo.contains("rows.insert(entity.id, entity.clone());"));
    assert!(repo.contains("lost when the process stops"));
    assert!(!repo.contains("stores nothing"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_skips_deleted_rows_and_pages_like_postgres() {
    let dir = scaffold_inmemory_product("inmemory_paging", &[]);

    let repo = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repo.contains(".filter(|row| !row.deleted)"));
    assert!(repo.contains("b.created_at"));
    assert!(repo.contains(".skip(page.offset() as usize)"));
    assert!(repo.contains(".take(page.limit() as usize)"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_checks_only_the_unique_fields() {
    let plain = scaffold_inmemory_product("inmemory_no_unique", &["name:String"]);
    let repo = read(&plain, "infrastructure/src/product/repository.rs");
    assert!(!repo.contains("RepositoryError::Duplicate"));
    cleanup(&plain);

    let unique = scaffold_inmemory_product("inmemory_unique", &["name:String", "sku:String!"]);
    let repo = read(&unique, "infrastructure/src/product/repository.rs");
    assert!(repo.contains("&& row.sku == entity.sku"));
    assert!(!repo.contains("row.name == entity.name"));
    assert!(repo.contains("return Err(RepositoryError::Duplicate);"));
    assert!(repo.contains("async fn should_reject_a_duplicate_sku()"));
    cleanup(&unique);
}

#[test]
fn inmemory_repository_ships_its_unit_tests() {
    let dir = scaffold_inmemory_product("inmemory_tests", &[]);

    let repo = read(&dir, "infrastructure/src/product/repository.rs");
    for test in [
        "async fn should_find_a_saved_product_by_id()",
        "async fn should_update_the_row_when_saved_again()",
        "async fn should_not_find_a_deleted_product()",
        "async fn should_return_the_requested_page_of_a_larger_list()",
    ] {
        assert!(repo.contains(test), "{test}");
    }
    assert!(!repo.contains("should_reject_a_duplicate"));

    cleanup(&dir);
}

#[test]
fn infrastructure_crate_can_run_async_tests() {
    let cargo = crate::TEMPLATE_DIR
        .get_file("infrastructure/Cargo.toml.liquid")
        .and_then(|f| f.contents_utf8())
        .unwrap();
    assert!(cargo.contains(
        "[dev-dependencies]\ntokio = { version = \"1\", features = [\"rt\", \"macros\"] }"
    ));
}

#[test]
fn make_run_says_an_inmemory_project_forgets_on_restart() {
    let dir = temp_dir("inmemory_banner");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let plain = new_project_non_interactive(Some("banner-mem".into()), false, &dir).unwrap();
    let db = new_project_non_interactive(Some("banner-db".into()), true, &dir).unwrap();

    let note = "in memory: data is lost on restart";
    assert!(read(&plain, "Makefile").contains(note));
    assert!(!read(&db, "Makefile").contains(note));

    cleanup(&dir);
}
