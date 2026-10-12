//! Phase 15.2: a process that cannot start says why — no panic, no `expect` outside tests.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The code of a generated file before its `#[cfg(test)]` module.
fn non_test_code(src: &str) -> &str {
    src.find("#[cfg(test)]").map_or(src, |at| &src[..at])
}

fn project(name: &str, db: bool, events: Option<&str>) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some(name.replace('_', "-")), db, &dir).unwrap();
    if let Some(transport) = events {
        scaffold::apply_events_to_new_project(&output, transport).unwrap();
    }
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();
    (dir, output)
}

#[test]
fn main_lets_a_failed_build_end_the_process_with_its_message() {
    let main = crate::TEMPLATE_DIR
        .get_file("presentation/src/main.rs.liquid")
        .and_then(|f| f.contents_utf8())
        .unwrap();
    assert!(main.contains(".try_serve(generated::bootstrap::build_app)"));
}

#[test]
fn db_bootstrap_reports_a_database_it_cannot_use_instead_of_panicking() {
    let (dir, output) = project("startup_db", true, None);

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    let code = non_test_code(&bootstrap);
    assert!(code.contains(
        "pub async fn build_app(config: puerto_web::Config) -> Result<impl Endpoint, String> {"
    ));
    assert!(!code.contains(".expect("), "{code}");
    assert!(code.contains("set it in .env"));
    assert!(code.contains("is the database up? `make docker-compose/up`"));
    assert!(code.contains("Ok(crate::app::configure("));

    cleanup(&dir);
}

#[test]
fn inmemory_bootstrap_builds_the_app_the_same_way() {
    let (dir, output) = project("startup_inmemory", false, None);

    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(bootstrap.contains("-> Result<impl Endpoint, String> {"));
    assert!(bootstrap.contains("Ok(crate::app::configure("));
    assert!(!bootstrap.contains("DATABASE_URL"));

    cleanup(&dir);
}

#[test]
fn worker_reports_what_it_cannot_reach_instead_of_panicking() {
    for transport in ["db", "kafka"] {
        let (dir, output) = project(
            &format!("startup_worker_{transport}"),
            true,
            Some(transport),
        );

        let main = read(&output, "worker/src/main.rs");
        assert!(
            main.contains(".try_run(generated::registry::build_worker)"),
            "{transport}"
        );
        let registry = read(&output, "worker/src/generated/registry.rs");
        let code = non_test_code(&registry);
        assert!(
            code.contains(
                "pub async fn build_worker(config: WorkerConfig) -> Result<Loops, String> {"
            ),
            "{transport}: {code}"
        );
        assert!(!code.contains(".expect("), "{transport}: {code}");
        assert!(
            code.contains("is the database up? `make docker-compose/up`"),
            "{transport}"
        );

        cleanup(&dir);
    }
}
