use super::*;

// ── puerto generate migration ─────────────────────────────────────────────

#[test]
fn migration_creates_migrations_dir_when_missing() {
    let dir = temp_dir("migration_no_dir");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    scaffold::run_migration("add_products_table", &dir, None).unwrap();

    assert!(
        dir.join("infrastructure/migrations").exists(),
        "expected infrastructure/migrations to be created automatically"
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
    assert!(!dir.join("infrastructure/migrations").exists());

    cleanup(&dir);
}

#[test]
fn migrations_created_in_the_same_second_get_distinct_versions() {
    // sqlx keys migrations by the timestamp prefix: two scaffolds within one second produced the
    // same version and `sqlx migrate run` refused with "previously applied but modified".
    let dir = temp_dir("migration_same_second");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    for name in ["create_a_table", "create_b_table", "create_c_table"] {
        scaffold::run_migration(name, &dir, Some("SELECT 1;\n")).unwrap();
    }

    let mut versions: Vec<String> = fs::read_dir(dir.join("infrastructure/migrations"))
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

    let entry = fs::read_dir(dir.join("infrastructure/migrations"))
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
