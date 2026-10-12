use super::*;

// ── puerto new (non-interactive / flag-driven) ────────────────────────────

#[test]
fn new_name_flag_sets_project_name() {
    let dir = temp_dir("new_name_flag");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("explicit-name".into()), false, &dir).unwrap();

    let content = fs::read_to_string(output.join("presentation/Cargo.toml")).unwrap();
    assert!(content.contains("name = \"explicit-name\""));

    cleanup(&dir);
}

#[test]
fn new_db_flag_creates_db_files() {
    let dir = temp_dir("new_db_flag");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("db-project".into()), true, &dir).unwrap();

    assert!(output.join("docker-compose.yml").exists());
    assert!(output.join(".env").exists());
    assert!(output.join(".env.example").exists());
    assert!(output.join(".cargo/config.toml").exists());
    assert!(output.join("infrastructure/migrations").is_dir());
    assert!(output.join("infrastructure/src/db.rs").exists());

    cleanup(&dir);
}

#[test]
fn new_without_db_flag_has_no_db_files() {
    let dir = temp_dir("new_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("plain-project".into()), false, &dir).unwrap();

    assert!(!output.join(".env.example").exists());
    assert!(!output.join(".cargo/config.toml").exists());
    assert!(!output.join("infrastructure/src/db.rs").exists());

    cleanup(&dir);
}

// ── puerto new ────────────────────────────────────────────────────────────

#[test]
fn creates_project_structure() {
    let dir = temp_dir("cg_structure");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("test-app", &dir).unwrap();

    assert!(output.join("Cargo.toml").exists());
    assert!(output.join("business/src/lib.rs").exists());
    assert!(output.join("infrastructure/src/lib.rs").exists());
    assert!(output.join("presentation/src/main.rs").exists());

    cleanup(&dir);
}

#[test]
fn presentation_cargo_toml_has_project_name() {
    let dir = temp_dir("cg_cargo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("my-puerto-app", &dir).unwrap();

    let content = fs::read_to_string(output.join("presentation/Cargo.toml")).unwrap();
    assert!(content.contains("name = \"my-puerto-app\""));

    cleanup(&dir);
}

#[test]
fn main_rs_is_minimal_entry_point() {
    let dir = temp_dir("cg_main");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("cool-project", &dir).unwrap();

    let content = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();
    assert!(content.contains("generated::bootstrap::build_app"));
    assert!(content.contains("mod generated"));
    assert!(!content.contains("{{project-name}}"));

    cleanup(&dir);
}

#[test]
fn bootstrap_rs_wires_ddd_layers() {
    let dir = temp_dir("cg_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("api-test", &dir).unwrap();

    let content =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(content.contains("GetGreetingUseCaseImpl"));
    assert!(content.contains("InMemoryGreetingRepository"));
    assert!(content.contains("GreetingApi"));
    assert!(content.contains("OpenApiService"));
    assert!(content.contains("puerto_web::routes("));

    cleanup(&dir);
}

#[test]
fn ddd_layers_exist() {
    let dir = temp_dir("cg_ddd");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("ddd-app", &dir).unwrap();

    assert!(
        output
            .join("business/src/domain/greeting/model.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/domain/greeting/errors.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/domain/greeting/repository.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/domain/greeting/use_cases/get_greeting.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/application/greeting/get_greeting.rs")
            .exists()
    );
    assert!(
        output
            .join("infrastructure/src/greeting/repository.rs")
            .exists()
    );
    assert!(
        output
            .join("presentation/src/api/greeting/routes.rs")
            .exists()
    );
    assert!(output.join("presentation/src/api/greeting/dto.rs").exists());
    assert!(
        output
            .join("presentation/src/api/greeting/error_mapper.rs")
            .exists()
    );
    assert!(
        output
            .join("presentation/src/api/greeting/responses.rs")
            .exists()
    );
    assert!(output.join("puerto.toml").exists());
    assert!(output.join("presentation/src/generated.rs").exists());
    assert!(
        output
            .join("presentation/src/generated/bootstrap.rs")
            .exists()
    );
    assert!(output.join("presentation/src/api/tags.rs").exists());

    cleanup(&dir);
}

#[test]
fn puerto_toml_has_project_name_and_greeting_entity() {
    let dir = temp_dir("cg_puerto_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("my-app", &dir).unwrap();

    let content = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(content.contains("name = \"my-app\""));
    assert!(content.contains("name = \"Greeting\""));
    assert!(content.contains("get_greeting"));

    cleanup(&dir);
}

#[test]
#[ignore = "slow: compiles and tests a full generated project"]
fn generated_project_compiles_and_tests_pass() {
    let dir = temp_dir("cg_compile");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("compile-test", &dir).unwrap();
    super::compile_matrix::patch_runtime_crates(&output);

    let result = std::process::Command::new("cargo")
        .args(["test", "--workspace"])
        .current_dir(&output)
        .output()
        .expect("failed to run cargo test");

    if !result.status.success() {
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
        panic!("cargo test failed in generated project");
    }

    cleanup(&dir);
}

// ── puerto new --db ───────────────────────────────────────────────────────

pub(super) fn new_project_non_interactive(
    name: Option<String>,
    db: bool,
    destination: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let tmp = extract_template()?;
    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(tmp.path().to_string_lossy().into_owned()),
            ..Default::default()
        },
        name: name.clone(),
        destination: Some(destination.to_path_buf()),
        vcs: Some(Vcs::None),
        no_workspace: true,
        ..Default::default()
    };
    let output = generate(args)?;
    drop(tmp);
    if db {
        scaffold::apply_db_to_new_project(&output)?;
    }
    Ok(output)
}

fn generate_db_project(
    name: &str,
    destination: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = generate_project(name, destination)?;
    scaffold::apply_db_to_new_project(&output)?;
    Ok(output)
}

#[test]
fn db_project_has_env_example() {
    let dir = temp_dir("db_env");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let content = fs::read_to_string(output.join(".env.example")).unwrap();
    assert!(content.contains("DATABASE_URL"));
    assert!(content.contains("db_app"));
    cleanup(&dir);
}

#[test]
fn db_project_has_env_file_with_project_db_name() {
    let dir = temp_dir("db_env_file");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("my-api", &dir).unwrap();
    let content = fs::read_to_string(output.join(".env")).unwrap();
    assert!(content.contains("DATABASE_URL"));
    assert!(content.contains("my_api"));
    cleanup(&dir);
}

#[test]
fn db_project_has_docker_compose_with_project_name() {
    let dir = temp_dir("db_docker_compose");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("my-api", &dir).unwrap();
    let content = fs::read_to_string(output.join("docker-compose.yml")).unwrap();
    assert!(content.contains("postgres:16"));
    assert!(content.contains("my-api-postgres"));
    assert!(content.contains("my_api"));
    assert!(content.contains("5432"));
    cleanup(&dir);
}

#[test]
fn db_project_has_gitignore_with_env() {
    let dir = temp_dir("db_gitignore");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let gitignore_path = output.join(".gitignore");
    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(content.contains(".env"));
    }
    cleanup(&dir);
}

#[test]
fn db_project_makefile_has_docker_and_sqlx_targets() {
    let dir = temp_dir("db_makefile_targets");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let content = fs::read_to_string(output.join("Makefile")).unwrap();
    assert!(content.contains("docker-compose/up"));
    assert!(content.contains("docker-compose/down"));
    assert!(content.contains("sqlx/migrate"));
    assert!(content.contains("sqlx/prepare"));
    assert!(content.contains("sqlx/check"));
    assert!(content.contains("sqlx/online"));
    assert!(content.contains("sqlx/offline"));
    assert!(content.contains("reset-db"));
    assert!(content.contains("test/infrastructure"));
    cleanup(&dir);
}

/// `make <args>` in a generated project: (exit success, stdout + stderr).
fn make(output: &Path, args: &[&str]) -> (bool, String) {
    let result = std::process::Command::new("make")
        .args(args)
        .current_dir(output)
        .output()
        .expect("make not found");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    (result.status.success(), text)
}

#[test]
fn should_define_each_db_make_target_once_when_the_project_has_a_database() {
    // `test/infrastructure` had a recipe in the template and another in the db targets: every
    // `make` call of a --db project printed "warning: overriding recipe".
    let dir = temp_dir("db_makefile_once");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-make", &dir).unwrap();

    // Act
    let (ok, text) = make(&output, &["-n", "help"]);

    // Assert
    assert!(ok, "{text}");
    assert!(!text.contains("overriding recipe"), "{text}");
    cleanup(&dir);
}

#[test]
fn should_start_postgres_and_wait_for_it_when_running_a_db_project() {
    // `up -d` returns before Postgres accepts connections: the first `make sqlx/migrate` of a
    // fresh project failed with "connection reset by peer". And `make run` started an API whose
    // database was not up.
    let dir = temp_dir("db_makefile_wait");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-wait", &dir).unwrap();

    for target in ["sqlx/migrate", "run", "test/infrastructure"] {
        // Act
        let (ok, text) = make(&output, &["-n", target]);

        // Assert
        assert!(ok, "{text}");
        assert!(
            text.contains("docker compose up -d --wait"),
            "`make {target}` does not wait for the database:\n{text}"
        );
    }
    cleanup(&dir);
}

#[test]
fn should_print_colors_not_escape_codes_when_make_echoes() {
    // `echo "\033[…"` prints the escape literally where `sh` is bash (Arch, Fedora…).
    let dir = temp_dir("makefile_colors");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("colors", &dir).unwrap();

    // Act
    let (ok, text) = make(&output, &["help"]);

    // Assert
    assert!(ok, "{text}");
    assert!(!text.contains("\\033"), "{text}");
    cleanup(&dir);
}

#[test]
fn should_ask_for_web_install_when_the_frontend_dependencies_are_missing() {
    // Without `make web/install`, `make lint` / `make test` failed with bun's "exited with code
    // 127" — nothing said what to run.
    let dir = temp_dir("web_not_installed");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("web-missing", &dir).unwrap();

    for target in ["web/lint", "web/test", "web/dev"] {
        // Act
        let (ok, text) = make(&output, &[target]);

        // Assert
        assert!(!ok, "`make {target}` should fail without node_modules");
        assert!(
            text.contains("make web/install"),
            "`make {target}` does not say how to fix it:\n{text}"
        );
    }
    cleanup(&dir);
}

#[test]
fn db_project_has_cargo_config_with_sqlx_offline() {
    let dir = temp_dir("db_cargo_config");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let content = fs::read_to_string(output.join(".cargo/config.toml")).unwrap();
    assert!(content.contains("SQLX_OFFLINE"));
    cleanup(&dir);
}

#[test]
fn db_project_has_migrations_dir() {
    let dir = temp_dir("db_migrations");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    assert!(output.join("infrastructure/migrations").is_dir());
    cleanup(&dir);
}

#[test]
fn db_project_has_db_rs_with_the_project_migrator() {
    let dir = temp_dir("db_db_rs");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let content = fs::read_to_string(output.join("infrastructure/src/db.rs")).unwrap();
    assert!(content.contains("MIGRATOR"));
    assert!(content.contains("puerto_sqlx"));
    cleanup(&dir);
}

#[test]
fn db_project_infrastructure_cargo_has_sqlx() {
    let dir = temp_dir("db_infra_cargo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let content = fs::read_to_string(output.join("infrastructure/Cargo.toml")).unwrap();
    assert!(content.contains("sqlx"));
    assert!(content.contains("postgres"));
    cleanup(&dir);
}

#[test]
fn db_project_puerto_toml_has_project_db_true() {
    let dir = temp_dir("db_proj_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_db_project("db-app", &dir).unwrap();
    let config = puerto_toml::read(&output).unwrap();
    assert!(
        config.project.db,
        "project.db should be true for --db projects"
    );
    cleanup(&dir);
}

#[test]
fn no_db_project_puerto_toml_omits_project_db() {
    let dir = temp_dir("no_db_proj_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("plain-app", &dir).unwrap();
    let config = puerto_toml::read(&output).unwrap();
    assert!(
        !config.project.db,
        "project.db should be false for non-db projects"
    );
    let content = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(
        !content.contains("db = true"),
        "puerto.toml should not contain db = true for non-db projects"
    );
    cleanup(&dir);
}

// ── Phase 7.3: puerto new --no-demo ──────────────────────────────────────

#[test]
fn no_demo_has_no_greeting_files() {
    let dir = temp_dir("no_demo_files");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("no-demo-app".into()), false, &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();

    assert!(!output.join("business/src/domain/greeting").exists());
    assert!(!output.join("business/src/application/greeting").exists());
    assert!(!output.join("infrastructure/src/greeting").exists());
    assert!(!output.join("presentation/src/api/greeting").exists());
    assert!(!output.join("presentation/src/api/greeting.rs").exists());

    cleanup(&dir);
}

#[test]
fn no_demo_puerto_toml_has_no_entity_block() {
    let dir = temp_dir("no_demo_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("no-demo-app".into()), false, &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();

    let content = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(
        !content.contains("[[entity]]"),
        "puerto.toml should have no entity blocks"
    );
    assert!(
        !content.contains("Greeting"),
        "puerto.toml should not mention Greeting"
    );
    assert!(
        content.contains("[project]"),
        "puerto.toml should still have [project]"
    );

    cleanup(&dir);
}

#[test]
fn no_demo_lib_files_have_no_greeting_modules() {
    let dir = temp_dir("no_demo_lib");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("no-demo-app".into()), false, &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();

    let biz_lib = fs::read_to_string(output.join("business/src/lib.rs")).unwrap();
    assert!(
        !biz_lib.contains("greeting"),
        "business lib.rs should not reference greeting"
    );

    let infra_lib = fs::read_to_string(output.join("infrastructure/src/lib.rs")).unwrap();
    assert!(
        !infra_lib.contains("greeting"),
        "infra lib.rs should not reference greeting"
    );

    let api_rs = fs::read_to_string(output.join("presentation/src/api.rs")).unwrap();
    assert!(
        !api_rs.contains("greeting"),
        "api.rs should not reference greeting"
    );
    assert!(
        api_rs.contains("pub mod error"),
        "api.rs should retain error module"
    );

    cleanup(&dir);
}

#[test]
fn no_demo_bootstrap_has_no_greeting() {
    let dir = temp_dir("no_demo_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("no-demo-app".into()), false, &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();

    let content =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(
        !content.contains("Greeting"),
        "bootstrap.rs should have no Greeting reference"
    );
    assert!(
        !content.contains("greeting"),
        "bootstrap.rs should have no greeting reference"
    );

    cleanup(&dir);
}

#[test]
fn default_project_still_has_greeting_files() {
    let dir = temp_dir("with_demo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("demo-app".into()), false, &dir).unwrap();

    assert!(
        output
            .join("business/src/domain/greeting/model.rs")
            .exists()
    );
    assert!(
        output
            .join("presentation/src/api/greeting/routes.rs")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn no_demo_then_scaffold_generates_new_entity() {
    let dir = temp_dir("no_demo_then_scaffold");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("no-demo-app".into()), false, &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();
    scaffold::run_scaffold("Player", &output, &[], OverwritePolicy::Ask).unwrap();

    assert!(output.join("business/src/domain/player/model.rs").exists());
    assert!(
        output
            .join("presentation/src/api/player/routes.rs")
            .exists()
    );
    assert!(!output.join("business/src/domain/greeting").exists());

    cleanup(&dir);
}

// ── T-02: --no-demo keeps the tags module ─────────────────────────────────

#[test]
fn no_demo_keeps_tags_module_declared() {
    let dir = temp_dir("no_demo_tags");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("no-demo-tags", &dir).unwrap();

    scaffold::apply_no_demo(&output).unwrap();

    let api_rs = fs::read_to_string(output.join("presentation/src/api.rs")).unwrap();
    assert!(
        api_rs.contains("pub mod tags;"),
        "generated routes.rs import crate::api::tags::ApiTags — the declaration must survive \
         --no-demo. Got:\n{api_rs}"
    );
    cleanup(&dir);
}

#[test]
fn api_rs_patch_restores_missing_tags_module() {
    let dir = temp_dir("api_rs_tags_restore");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("tags-restore", &dir).unwrap();

    // Simulate a hand-edited api.rs that lost the declaration.
    fs::write(output.join("presentation/src/api.rs"), "pub mod error;\n").unwrap();
    crate::patchers::api_rs::patch_api_rs(&output, "product").unwrap();

    let api_rs = fs::read_to_string(output.join("presentation/src/api.rs")).unwrap();
    assert!(api_rs.contains("pub mod tags;"));
    assert!(api_rs.contains("pub mod product;"));
    cleanup(&dir);
}

// ── Phase 9.0: the web frontend shell ────────────────────────────────────

#[test]
fn new_project_includes_the_web_frontend_by_default() {
    let dir = temp_dir("new_web_default");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = crate::commands::new::new_project(
        Some("web-app".into()),
        crate::commands::new::NewOptions {
            no_db: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();

    for file in [
        "web/package.json",
        "web/bunfig.toml",
        "web/.dependency-cruiser.cjs",
        "web/src/main.tsx",
        "web/src/application/README.md",
        "web/src/presentation/core/generated/dependencies.ts",
        "web/src/presentation/core/generated/routes.tsx",
        "web/AGENTS.md",
    ] {
        assert!(output.join(file).exists(), "missing {file}");
    }
    assert!(puerto_toml::read(&output).unwrap().project.web);
    let agents = fs::read_to_string(output.join("AGENTS.md")).unwrap();
    assert!(agents.contains("web/AGENTS.md"), "{agents}");
    cleanup(&dir);
}

#[test]
fn new_project_with_no_web_has_no_frontend_and_does_not_mention_one() {
    let dir = temp_dir("new_no_web");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = crate::commands::new::new_project(
        Some("api-only".into()),
        crate::commands::new::NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();

    assert!(!output.join("web").exists());
    assert!(!puerto_toml::read(&output).unwrap().project.web);
    let manifest = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(!manifest.contains("web ="), "{manifest}");
    let agents = fs::read_to_string(output.join("AGENTS.md")).unwrap();
    assert!(!agents.contains("web/"), "{agents}");
    // Only the frontend section goes: what follows it stays.
    assert!(agents.contains("## Conventions"), "{agents}");
    cleanup(&dir);
}

#[test]
fn web_index_html_is_titled_after_the_project() {
    let dir = temp_dir("web_title");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("shop-front", &dir).unwrap();

    let index = fs::read_to_string(output.join("web/index.html")).unwrap();
    assert!(index.contains("<title>shop-front</title>"), "{index}");
    cleanup(&dir);
}

#[test]
fn web_package_pins_exact_versions_and_runs_on_bun() {
    // A caret range lets a fresh install pick a release the templates were never tested against.
    let dir = temp_dir("web_pins");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("web-pins", &dir).unwrap();

    let package = fs::read_to_string(output.join("web/package.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&package).unwrap();
    assert!(json["packageManager"].as_str().unwrap().starts_with("bun@"));
    for section in ["dependencies", "devDependencies"] {
        for (name, version) in json[section].as_object().unwrap() {
            let version = version.as_str().unwrap();
            assert!(
                version.chars().next().unwrap().is_ascii_digit(),
                "{name} is not pinned: {version}"
            );
        }
    }
    cleanup(&dir);
}

#[test]
fn template_never_embeds_installed_web_dependencies() {
    // The template is embedded with include_dir!: a `bun install` run inside it would ship
    // node_modules inside the puerto binary.
    for path in ["web/node_modules", "web/dist"] {
        assert!(
            crate::TEMPLATE_DIR.get_dir(path).is_none(),
            "{path} is embedded in the template"
        );
    }
}

#[test]
fn makefile_runs_the_frontend_checks_when_web_exists() {
    let dir = temp_dir("web_makefile");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("web-make", &dir).unwrap();

    let makefile = fs::read_to_string(output.join("Makefile")).unwrap();
    for target in ["web/install:", "web/dev:", "web/test:", "web/lint:"] {
        assert!(makefile.contains(target), "Makefile misses {target}");
    }
    for recipe in ["$(MAKE) web/test", "$(MAKE) web/lint"] {
        assert!(makefile.contains(recipe), "Makefile misses {recipe}");
    }
    cleanup(&dir);
}
