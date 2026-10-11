use super::*;

// ── Phase 5: logger ───────────────────────────────────────────────────────

#[test]
fn new_project_bootstrap_injects_no_logger() {
    // Logging goes through `tracing` directly; the subscriber is set up once in main.rs.
    let dir = temp_dir("logger_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("logger-test", &dir).unwrap();

    let content =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(!content.contains("logger"), "{content}");
    let main = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();
    assert!(main.contains("tracing_subscriber::fmt()"), "{main}");

    cleanup(&dir);
}

#[test]
fn scaffold_use_case_impl_logs_with_tracing() {
    let dir = temp_dir("logger_scaffold_impl");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("business/src/application/product/create_product.rs")).unwrap();
    assert!(content.contains("tracing::info!"), "{content}");
    assert!(!content.contains("logger"), "{content}");

    cleanup(&dir);
}

#[test]
fn use_case_generator_impl_logs_with_tracing() {
    let dir = temp_dir("logger_uc_impl");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_use_case_stubs(&dir);
    scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

    let content =
        fs::read_to_string(dir.join("business/src/application/product/delete_product.rs")).unwrap();
    assert!(content.contains("tracing::info!"), "{content}");
    assert!(!content.contains("logger"), "{content}");

    cleanup(&dir);
}

#[test]
fn scaffold_bootstrap_wires_use_cases_without_a_logger() {
    let dir = temp_dir("logger_bootstrap_scaffold");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(content.contains(
        "let create_product = Arc::new(CreateProductUseCaseImpl { repository: Arc::new(InMemoryProductRepository::new()) as Arc<dyn ProductRepositoryTrait> });"
    ), "{content}");
    assert!(!content.contains("logger"), "{content}");

    cleanup(&dir);
}

// ── T-14: document-api conventions (T-31…T-38, logger removal) ────────────

/// Every file under `base` (skipping build output), as `(relative path, content)`.
fn project_files(base: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "target" || name == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(&path, base, out);
            } else if let Ok(content) = fs::read_to_string(&path) {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                out.push((rel, content));
            }
        }
    }
    let mut out = vec![];
    walk(base, base, &mut out);
    out
}

/// A generated project with the Greeting demo plus a scaffolded entity carrying a VO.
fn project_with_product(name: &str, db: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(&name.replace('_', "-"), &dir).unwrap();
    crate::generators::project::apply_base_env(&output).unwrap();
    if db {
        crate::generators::project::apply_db_to_new_project(&output).unwrap();
    }
    let fields = [
        scaffold_test_field("title", "String"),
        puerto_toml::parse_field_arg("status:Status:enum:Active/Archived").unwrap(),
    ];
    scaffold::run_scaffold("Product", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

#[test]
fn generated_project_logs_with_tracing_and_has_no_logger_port() {
    // document-api's convention: `tracing` is already a facade, so an injected `LoggerTrait` only
    // added a field to every use case and repository and a mock helper to every test module.
    let (dir, output) = project_with_product("no_logger_port", true);
    scaffold::run_use_case("Product", "archive_product", &output).unwrap();

    for (path, content) in project_files(&output) {
        if !path.ends_with(".rs") {
            continue;
        }
        assert!(
            !content.contains("LoggerTrait"),
            "{path} still names LoggerTrait"
        );
        assert!(
            !content.contains("MockLogger"),
            "{path} still names MockLogger"
        );
        assert!(
            !content.contains("logger:"),
            "{path} still has a logger field"
        );
    }
    assert!(!output.join("business/src/domain/logger.rs").exists());
    assert!(!output.join("infrastructure/src/logger.rs").exists());
    let create =
        fs::read_to_string(output.join("business/src/application/product/create_product.rs"))
            .unwrap();
    assert!(create.contains("tracing::info!"), "{create}");
    let business_toml = fs::read_to_string(output.join("business/Cargo.toml")).unwrap();
    assert!(business_toml.contains("tracing"), "{business_toml}");

    cleanup(&dir);
}

#[test]
fn no_demo_project_has_no_logger_module() {
    let dir = temp_dir("no_demo_no_logger");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("no-demo-no-logger", &dir).unwrap();

    scaffold::apply_no_demo(&output).unwrap();

    let lib = fs::read_to_string(output.join("business/src/lib.rs")).unwrap();
    assert!(!lib.contains("logger"), "{lib}");
    let infra = fs::read_to_string(output.join("infrastructure/src/lib.rs")).unwrap();
    assert!(!infra.contains("logger"), "{infra}");
    cleanup(&dir);
}

// ── T-37: error responses ─────────────────────────────────────────────────

#[test]
fn error_mapper_names_the_code_and_maps_validation_to_422() {
    let fields = [
        scaffold_test_field("title", "String"),
        puerto_toml::parse_field_arg("status:Status:enum:Active/Archived").unwrap(),
    ];
    let ctx =
        crate::generators::presentation::build_error_mapper_ctx("Product", "product", &fields);

    let mapper = crate::generators::render::render("presentation/error_mapper.tera", &ctx).unwrap();

    assert!(mapper.contains("ErrorResponse::from_code"), "{mapper}");
    assert!(!mapper.contains("\"product_error\""), "{mapper}");
    assert!(
        mapper.contains("ProductError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY")
    );
    assert!(mapper.contains("ProductError::InvalidStatus => StatusCode::UNPROCESSABLE_ENTITY"));
    // One test per arm, so a status can't drift unnoticed.
    for test in [
        "should_map_validation_error_to_422_with_its_code",
        "should_map_not_found_to_404_with_its_code",
        "should_map_duplicate_to_409_with_its_code",
        "should_map_repository_error_to_500_with_its_code",
        "should_map_invalid_status_to_422_with_its_code",
    ] {
        assert!(mapper.contains(test), "missing {test}:\n{mapper}");
    }
}

#[test]
fn responses_offer_422_and_no_longer_map_400() {
    let ctx = crate::generators::presentation::build_simple_ctx("Product", "product");

    let responses =
        crate::generators::render::render("presentation/responses_crud.tera", &ctx).unwrap();

    assert!(responses.contains("#[oai(status = 422)]"));
    assert!(
        responses.contains("StatusCode::UNPROCESSABLE_ENTITY => Self::UnprocessableEntity(error)")
    );
    assert!(!responses.contains("BAD_REQUEST"), "{responses}");
}

#[test]
fn shared_error_response_is_built_from_the_code() {
    let dir = temp_dir("error_rs_from_code");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("error-rs-from-code", &dir).unwrap();

    let error_rs = fs::read_to_string(output.join("presentation/src/api/error.rs")).unwrap();

    assert!(error_rs.contains("pub fn from_code("), "{error_rs}");
    cleanup(&dir);
}

// ── T-38: health probes + graceful shutdown ───────────────────────────────

#[test]
fn generated_project_serves_liveness_and_readiness_and_shuts_down_gracefully() {
    let (dir, output) = project_with_product("health_no_db", false);

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    let main = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();

    assert!(
        bootstrap.contains(".at(\"/health\", get(crate::health::liveness))"),
        "{bootstrap}"
    );
    assert!(bootstrap.contains("\"/health/ready\""), "{bootstrap}");
    assert!(main.contains("run_with_graceful_shutdown"), "{main}");
    assert!(main.contains("mod health;"), "{main}");
    assert!(output.join("presentation/src/health.rs").exists());
    cleanup(&dir);
}

#[test]
fn readiness_pings_the_database_in_a_db_project() {
    let (dir, output) = project_with_product("health_db", true);

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    let db_rs = fs::read_to_string(output.join("infrastructure/src/db.rs")).unwrap();

    assert!(
        bootstrap.contains("infrastructure::db::ping(&pool)"),
        "{bootstrap}"
    );
    assert!(db_rs.contains("pub async fn ping("), "{db_rs}");
    cleanup(&dir);
}

// ── T-35: toolchain + edition ─────────────────────────────────────────────

#[test]
fn generated_project_pins_its_toolchain_and_uses_edition_2024() {
    // The clippy lints the generated `make lint` enforces change between Rust versions: without a
    // pin, whether a fresh project passes its own lint depends on the machine.
    let dir = temp_dir("toolchain_pin");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("toolchain-pin", &dir).unwrap();

    let toolchain = fs::read_to_string(output.join("rust-toolchain.toml")).unwrap();
    assert!(toolchain.contains("channel = \"1.95.0\""), "{toolchain}");
    assert!(toolchain.contains("clippy") && toolchain.contains("rustfmt"));
    for krate in ["business", "infrastructure", "presentation"] {
        let manifest = fs::read_to_string(output.join(format!("{krate}/Cargo.toml"))).unwrap();
        assert!(
            manifest.contains("edition = \"2024\""),
            "{krate}: {manifest}"
        );
    }
    let workspace = fs::read_to_string(output.join("Cargo.toml")).unwrap();
    assert!(workspace.contains("resolver = \"3\""), "{workspace}");
    cleanup(&dir);
}

// ── T-31 / T-32 / T-33: AI rules shipped to generated projects ────────────

#[test]
fn generated_agent_rules_never_describe_mod_rs_files() {
    // The per-crate AGENTS.md drew `mod.rs` trees — the one thing the module convention forbids —
    // and nothing tested them. An agent in a generated project was told to create them.
    let dir = temp_dir("agents_no_mod_rs");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("agents-no-mod-rs", &dir).unwrap();

    for (path, content) in project_files(&output) {
        if !path.ends_with("AGENTS.md") {
            continue;
        }
        for line in content.lines() {
            if line.contains("mod.rs") {
                assert!(
                    line.to_lowercase().contains("no `mod.rs`"),
                    "{path} describes a mod.rs file: {line}"
                );
            }
        }
        assert!(
            !content.contains("LoggerTrait"),
            "{path} documents the removed logger port"
        );
    }
    cleanup(&dir);
}

#[test]
fn generated_project_has_a_root_entry_point_for_agents() {
    let dir = temp_dir("agents_root");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("agents-root", &dir).unwrap();

    let claude = fs::read_to_string(output.join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(output.join("AGENTS.md")).unwrap();

    assert_eq!(claude.trim(), "@AGENTS.md");
    for must in [
        "puerto generate",
        "generated/bootstrap.rs",
        "Tier 1",
        "make lint",
        "one commit",
        "business/AGENTS.md",
    ] {
        assert!(agents.contains(must), "root AGENTS.md misses '{must}'");
    }
    cleanup(&dir);
}

#[test]
fn template_bootstrap_is_exactly_what_the_generator_writes() {
    // `puerto new` copies the template's bootstrap.rs verbatim — it is not regenerated. When the
    // generator changed (the logger removal), the copy kept wiring a `TracingLogger` that no longer
    // existed, and every fresh project failed to compile.
    let dir = temp_dir("template_bootstrap_drift");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("template-bootstrap-drift", &dir).unwrap();
    let config = puerto_toml::read(&output).unwrap();

    let shipped =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    let tags = fs::read_to_string(output.join("presentation/src/api/tags.rs")).unwrap();

    assert_eq!(
        shipped,
        crate::generators::bootstrap::generate_bootstrap_content(&config.entity).unwrap(),
        "template/presentation/src/generated/bootstrap.rs is stale — regenerate it"
    );
    assert_eq!(
        tags,
        crate::generators::bootstrap::generate_tags_content(&config.entity).unwrap(),
        "template/presentation/src/api/tags.rs is stale — regenerate it"
    );
    cleanup(&dir);
}
