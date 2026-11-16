//! Phase 15.0: without `--db`, `InMemory{Entity}Repository` keeps what it is given, so the
//! default project's create → detail flow finds what it just created.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn scaffold_inmemory_producto(name: &str, args: &[&str]) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    let fields: Vec<puerto_toml::Field> = args
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run("Producto", &dir, false, &fields, &[]).unwrap();
    dir
}

#[test]
fn inmemory_repository_keeps_rows_in_a_map_keyed_by_id() {
    let dir = scaffold_inmemory_producto("inmemory_map", &[]);

    let repo = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repo.contains("filas: RwLock<HashMap<IdProducto, Producto>>,"));
    assert!(repo.contains("#[derive(Default)]\npub struct RepositorioProductoEnMemoria {"));
    assert!(repo.contains("pub fn new() -> Self {"));
    assert!(repo.contains("filas.insert(entidad.id, entidad.clone());"));
    assert!(repo.contains("se pierden cuando el proceso se para"));
    assert!(!repo.contains("stores nothing"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_skips_deleted_rows_and_pages_like_postgres() {
    let dir = scaffold_inmemory_producto("inmemory_paging", &[]);

    let repo = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repo.contains(".filter(|fila| !fila.borrado)"));
    assert!(repo.contains("b.creado_en"));
    assert!(repo.contains(".skip(pagina.desplazamiento() as usize)"));
    assert!(repo.contains(".take(pagina.limite() as usize)"));

    cleanup(&dir);
}

#[test]
fn inmemory_repository_checks_only_the_unique_fields() {
    let plain = scaffold_inmemory_producto("inmemory_no_unique", &["name:String"]);
    let repo = read(
        &plain,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(!repo.contains("ErrorRepositorio::Duplicado"));
    cleanup(&plain);

    let unique = scaffold_inmemory_producto("inmemory_unique", &["name:String", "sku:String!"]);
    let repo = read(
        &unique,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repo.contains("&& fila.sku == entidad.sku"));
    assert!(!repo.contains("row.name == entity.name"));
    assert!(repo.contains("return Err(ErrorRepositorio::Duplicado);"));
    assert!(repo.contains("async fn deberia_rechazar_un_duplicado_de_sku()"));
    cleanup(&unique);
}

#[test]
fn inmemory_repository_ships_its_unit_tests() {
    let dir = scaffold_inmemory_producto("inmemory_tests", &[]);

    let repo = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    for test in [
        "async fn deberia_encontrar_producto_por_id_tras_guardar()",
        "async fn deberia_actualizar_la_fila_cuando_se_guarda_de_nuevo()",
        "async fn no_deberia_encontrar_producto_tras_su_borrado()",
        "async fn deberia_devolver_la_pagina_pedida_de_una_lista_mayor()",
    ] {
        assert!(repo.contains(test), "{test}");
    }
    assert!(!repo.contains("should_reject_a_duplicate"));

    cleanup(&dir);
}

#[test]
fn infrastructure_crate_can_run_async_tests() {
    let dir = temp_dir("infra_async_tests");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let cargo =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/Cargo.toml")).unwrap();
    assert!(cargo.contains("[dev-dependencies]"), "{cargo}");
    assert!(cargo.contains("tokio = { workspace = true }"), "{cargo}");
    cleanup(&dir);
}

#[test]
fn make_run_says_an_inmemory_project_forgets_on_restart() {
    let dir = temp_dir("inmemory_banner");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let plain = new_project_non_interactive(Some("banner-mem".into()), false, &dir).unwrap();
    let db = new_project_non_interactive(Some("banner-db".into()), true, &dir).unwrap();

    let note = "en memoria: los datos se pierden al reiniciar";
    assert!(read(&plain, "Makefile").contains(note));
    assert!(!read(&db, "Makefile").contains(note));

    cleanup(&dir);
}
