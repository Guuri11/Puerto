use super::*;

// ── puerto generar migracion ─────────────────────────────────────────────

#[test]
fn migration_creates_migrations_dir_when_missing() {
    let dir = temp_dir("migration_no_dir");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    scaffold::run_migration("add_productos_table", &dir, None).unwrap();

    assert!(
        dir.join("migraciones").exists(),
        "expected migraciones to be created automatically"
    );

    cleanup(&dir);
}

#[test]
fn generate_migration_is_refused_in_a_project_without_database() {
    let dir = temp_dir("migration_no_db_project");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);

    let result = scaffold::run_generate_migration("add_stuff", &dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--db"));
    assert!(!dir.join("migraciones").exists());

    cleanup(&dir);
}

#[test]
fn migrations_created_in_the_same_second_get_distinct_versions() {
    // sqlx keys migrations by the timestamp prefix: two scaffolds within one second produced the
    // same version and `sqlx migrate run` refused with "previously applied but modified".
    let dir = temp_dir("migration_same_second");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    for name in ["crear_a_table", "crear_b_table", "crear_c_table"] {
        scaffold::run_migration(name, &dir, Some("SELECT 1;\n")).unwrap();
    }

    let mut versions: Vec<String> = fs::read_dir(dir.join("migraciones"))
        .unwrap()
        .flatten()
        .map(|e| {
            e.file_name()
                .to_string_lossy()
                .split('_')
                .next()
                .unwrap()
                .to_string()
        })
        .collect();
    versions.sort();
    versions.dedup();
    assert_eq!(versions.len(), 3, "versions must be unique: {versions:?}");
    assert!(
        versions
            .iter()
            .all(|v| v.len() == 14 && v.chars().all(|c| c.is_ascii_digit()))
    );

    cleanup(&dir);
}

#[test]
fn migration_file_carries_the_given_body() {
    let dir = temp_dir("migration_body");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    scaffold::run_migration("create_x_table", &dir, Some("CREATE TABLE xs ();\n")).unwrap();

    let entry = fs::read_dir(dir.join("migraciones"))
        .unwrap()
        .flatten()
        .next()
        .unwrap();
    assert!(
        entry
            .file_name()
            .to_string_lossy()
            .ends_with("_create_x_table.sql")
    );
    assert!(
        fs::read_to_string(entry.path())
            .unwrap()
            .contains("CREATE TABLE xs ();")
    );

    cleanup(&dir);
}

// ── T-13: real plurals for tables and REST paths ──────────────────────────

#[test]
fn pluralize_follows_spanish_rules_for_the_first_word() {
    use crate::generators::naming::pluralize;

    for (singular, plural) in [
        ("producto", "productos"),
        ("cliente", "clientes"),
        ("ciudad", "ciudades"),
        ("ley", "leyes"),
        ("lapiz", "lapices"),
        ("pais", "paises"),
        ("analisis", "analisis"),
        ("lunes", "lunes"),
        ("linea_pedido", "lineas_pedido"),
        ("estado_cuenta", "estados_cuenta"),
    ] {
        assert_eq!(pluralize(singular), plural, "{singular}");
    }
}

#[test]
fn db_scaffold_names_the_table_and_the_paths_with_the_real_plural() {
    let dir = temp_dir("t13_plural_table");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    scaffold::run_scaffold("Ciudad", &dir, &[], OverwritePolicy::Ask).unwrap();

    let migration = fs::read_dir(dir.join("migraciones"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(migration.contains("CREATE TABLE ciudades ("), "{migration}");

    let repo = fs::read_to_string(dir.join("contextos/ciudad/infraestructura/src/repositorio.rs"))
        .unwrap();
    assert!(repo.contains("FROM ciudades WHERE"), "{repo}");
    assert!(repo.contains("INSERT INTO ciudades ("), "{repo}");
    assert!(!repo.contains("ciudads"), "{repo}");

    let routes = fs::read_to_string(dir.join("contextos/ciudad/rest/src/rutas.rs")).unwrap();
    assert!(routes.contains(r#"path = "/ciudades""#), "{routes}");
    assert!(routes.contains(r#"path = "/ciudades/:id""#), "{routes}");
    assert!(
        routes.contains("/// Lista Ciudades, página a página"),
        "{routes}"
    );
    assert!(!routes.contains("ciudads"), "{routes}");

    cleanup(&dir);
}

// ── 17.0: a forced re-scaffold does not write a second CREATE TABLE ───────

fn table_migrations(dir: &Path, snake: &str) -> Vec<String> {
    let suffix = format!("_crear_tabla_{snake}.sql");
    fs::read_dir(dir.join("migraciones"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(&suffix))
        .collect()
}

#[test]
fn forced_rescaffold_keeps_the_one_migration_that_creates_the_table() {
    // `sqlx migrate run` failed on the second `CREATE TABLE categorias`: the table existed.
    let dir = temp_dir("rescaffold_one_create_table");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    scaffold::run_scaffold("Categoria", &dir, &[], OverwritePolicy::Ask).unwrap();
    let first = table_migrations(&dir, "categoria");
    let first_body = fs::read_to_string(dir.join("migraciones").join(&first[0])).unwrap();

    let fields = [scaffold_test_field("title", "String")];
    scaffold::run_scaffold("Categoria", &dir, &fields, OverwritePolicy::Force).unwrap();

    let after = table_migrations(&dir, "categoria");
    assert_eq!(after, first, "a second CREATE TABLE migration was written");
    let body = fs::read_to_string(dir.join("migraciones").join(&after[0])).unwrap();
    assert_eq!(body, first_body, "the existing migration was rewritten");
    cleanup(&dir);
}

#[test]
fn generate_repository_writes_no_create_table_when_one_exists() {
    let dir = temp_dir("repository_one_create_table");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);
    scaffold::run_scaffold("Categoria", &dir, &[], OverwritePolicy::Ask).unwrap();

    scaffold::run_generate_repository("Categoria", &dir).unwrap();

    assert_eq!(table_migrations(&dir, "categoria").len(), 1);
    cleanup(&dir);
}

// ── Phase 19.2: Postgres tests share the MIGRATOR ───────────────────────────
//
// `#[sqlx::test(migrations = "./migrations")]` copies every migration into each test (the SQL through
// `include_str!` and its checksums); `migrator = "…"` only refers to the static. In ant_backend the
// change took its tests' rebuild from 27 s to 7 s (ADR 0004 of digitalia).

#[test]
fn every_postgres_test_in_the_templates_uses_the_shared_migrator() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![src.join("snippets.rs")];
    let mut dirs = vec![src.join("templates")];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }

    let copying: Vec<_> = files
        .iter()
        .filter(|f| {
            fs::read_to_string(f)
                .unwrap()
                .contains("sqlx::test(migrations")
        })
        .collect();

    assert!(
        copying.is_empty(),
        "these still copy the migrations into each test: {copying:?}"
    );
}

#[test]
fn generated_postgres_repository_tests_use_the_shared_migrator() {
    let dir = temp_dir("migrator_shared_repository");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    fs::create_dir_all(dir.join("migraciones")).unwrap();
    scaffold::run("Producto", &dir, true, &[], &[]).unwrap();

    let repository =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/src/repositorio.rs"))
            .unwrap();

    assert!(repository.contains("#[sqlx::test(migrator = \"migrador::MIGRADOR\")]"));
    assert!(!repository.contains("sqlx::test(migrations"));
    cleanup(&dir);
}
