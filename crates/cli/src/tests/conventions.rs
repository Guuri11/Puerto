use super::*;

// ── Phase 5: logger ───────────────────────────────────────────────────────

#[test]
fn new_project_bootstrap_injects_no_logger() {
    // Logging goes through `tracing` directly; the subscriber is set up once, by
    // `puerto_web::App::servir` (ADR-0017).
    let dir = temp_dir("logger_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("logger-test", &dir).unwrap();

    let content = wiring(&output);
    assert!(!content.contains("logger"), "{content}");
    let main = fs::read_to_string(output.join("presentacion/api/src/main.rs")).unwrap();
    assert!(main.contains("puerto_web::App::new(config)"), "{main}");

    cleanup(&dir);
}

#[test]
fn scaffold_use_case_impl_logs_with_tracing() {
    let dir = temp_dir("logger_scaffold_impl");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/crear_producto.rs"))
            .unwrap();
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
    scaffold::run_use_case("Producto", "borrar_producto", &dir).unwrap();

    let content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/borrar_producto.rs"))
            .unwrap();
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
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = wiring(&dir);
    assert!(
        content.contains(
            "crear_producto: Arc::new(CrearProductoImpl { repositorio: Arc::clone(&repositorio) }),"
        ),
        "{content}"
    );
    assert!(
        content.contains("let repositorio: Arc<dyn RepositorioProducto> = Arc::new(RepositorioProductoEnMemoria::new());"),
        "{content}"
    );
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

/// A generated project with the Saludo demo plus a scaffolded entity carrying a VO.
fn project_with_producto(name: &str, db: bool) -> (PathBuf, PathBuf) {
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
    scaffold::run_scaffold("Producto", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

#[test]
fn generated_project_logs_with_tracing_and_has_no_logger_port() {
    // document-api's convention: `tracing` is already a facade, so an injected `LoggerTrait` only
    // added a field to every use case and repository and a mock helper to every test module.
    let (dir, output) = project_with_producto("no_logger_port", true);
    scaffold::run_use_case("Producto", "archive_producto", &output).unwrap();

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
    assert!(
        !output
            .join("contextos/producto/dominio/src/logger.rs")
            .exists()
    );
    assert!(
        !output
            .join("contextos/producto/infraestructura/src/logger.rs")
            .exists()
    );
    let create =
        fs::read_to_string(output.join("contextos/producto/aplicacion/src/crear_producto.rs"))
            .unwrap();
    assert!(create.contains("tracing::info!"), "{create}");
    let application_toml =
        fs::read_to_string(output.join("contextos/producto/aplicacion/Cargo.toml")).unwrap();
    assert!(application_toml.contains("tracing"), "{application_toml}");

    cleanup(&dir);
}

#[test]
fn no_demo_project_has_no_logger_module() {
    let dir = temp_dir("no_demo_no_logger");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("no-demo-no-logger", &dir).unwrap();

    scaffold::apply_no_demo(&output).unwrap();

    let lib = fs::read_to_string(output.join("nucleo/src/lib.rs")).unwrap();
    assert!(!lib.contains("logger"), "{lib}");
    assert!(!wiring(&output).contains("logger"));
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
        crate::generators::presentation::build_error_mapper_ctx("Producto", "producto", &fields);

    let mapper = crate::generators::render::render("presentation/error_mapper.tera", &ctx).unwrap();

    assert!(mapper.contains("RespuestaError::from_codigo"), "{mapper}");
    assert!(!mapper.contains("\"producto_error\""), "{mapper}");
    assert!(mapper.contains("ErrorProducto::Validacion(_) => StatusCode::UNPROCESSABLE_ENTITY"));
    assert!(mapper.contains("ErrorProducto::StatusInvalido => StatusCode::UNPROCESSABLE_ENTITY"));
    // One test per arm, so a status can't drift unnoticed.
    for test in [
        "deberia_mapear_validacion_a_422_con_su_codigo",
        "deberia_mapear_no_encontrado_a_404_con_su_codigo",
        "deberia_mapear_duplicado_a_409_con_su_codigo",
        "deberia_mapear_repositorio_a_500_con_su_codigo",
        "deberia_mapear_status_invalido_a_422_con_su_codigo",
    ] {
        assert!(mapper.contains(test), "missing {test}:\n{mapper}");
    }
}

#[test]
fn responses_offer_422_and_no_longer_map_400() {
    let ctx = crate::generators::presentation::build_simple_ctx("Producto", "producto");

    let responses =
        crate::generators::render::render("presentation/responses_crud.tera", &ctx).unwrap();

    assert!(responses.contains("#[oai(status = 422)]"));
    assert!(responses.contains("StatusCode::UNPROCESSABLE_ENTITY => Self::NoProcesable(error)"));
    assert!(!responses.contains("BAD_REQUEST"), "{responses}");
}

// ── T-38: health probes + graceful shutdown ───────────────────────────────
// The probes, the server and the shutdown are `puerto-web`'s (ADR-0017) and tested there; the
// project hands its readiness check to `puerto_web::rutas` (tests/runtime_crates.rs).

#[test]
fn readiness_pings_the_database_in_a_db_project() {
    let (dir, output) = project_with_producto("health_db", true);

    let bootstrap = wiring(&output);
    // `ping` is `puerto-sqlx`'s, which the binary names.
    assert!(
        bootstrap.contains("puerto_sqlx::ping(&pool)"),
        "{bootstrap}"
    );
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
    for krate in [
        "nucleo",
        "contextos/saludo/dominio",
        "contextos/saludo/aplicacion",
        "contextos/saludo/infraestructura",
        "contextos/saludo/rest",
        "presentacion/cableado",
        "presentacion/api",
    ] {
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
                    line.to_lowercase().contains("sin `mod.rs`"),
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
        "puerto generar",
        "generado/bootstrap.rs",
        "Nivel 1",
        "make lint",
        "un commit",
        "contextos/AGENTS.md",
        "presentacion/AGENTS.md",
        // ADR-0017: an agent must know which code is the project's and which is the runtime's.
        "puerto-nucleo",
        "puerto-web",
        "puerto-sqlx",
    ] {
        assert!(agents.contains(must), "root AGENTS.md misses '{must}'");
    }
    cleanup(&dir);
}

#[test]
fn template_bootstrap_is_exactly_what_the_generator_writes() {
    // `puerto nuevo` copies the template's bootstrap.rs verbatim — it is not regenerated. When the
    // generator changed (the logger removal), the copy kept wiring a `TracingLogger` that no longer
    // existed, and every fresh project failed to compile.
    let dir = temp_dir("template_bootstrap_drift");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("template-bootstrap-drift", &dir).unwrap();
    let config = puerto_toml::read(&output).unwrap();

    let shipped =
        fs::read_to_string(output.join("presentacion/api/src/generado/bootstrap.rs")).unwrap();

    assert_eq!(
        shipped,
        crate::generators::bootstrap::generate_bootstrap_content(
            &config.entity,
            config.project.web
        )
        .unwrap(),
        "template/presentacion/api/src/generado/bootstrap.rs is stale — regenerate it"
    );
    // `presentacion/cableado` is copied verbatim too, and `puerto generar bootstrap` must not
    // change it.
    let cableado = |file: &str| fs::read_to_string(output.join("presentacion/cableado").join(file));
    let (manifest, lib) = (
        cableado("Cargo.toml").unwrap(),
        cableado("src/lib.rs").unwrap(),
    );
    crate::generators::bootstrap::regenerate_bootstrap(&output).unwrap();
    assert_eq!(
        manifest,
        cableado("Cargo.toml").unwrap(),
        "template/presentacion/cableado/Cargo.toml.liquid is stale — regenerate it"
    );
    assert_eq!(
        lib,
        cableado("src/lib.rs").unwrap(),
        "template/presentacion/cableado/src/lib.rs is stale — regenerate it"
    );
    cleanup(&dir);
}

// ── T-26: presentation holds ports, routes are tested ──────────────────────

fn scaffolded_producto(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::run_scaffold(
        "Producto",
        &output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Ask,
    )
    .unwrap();
    (dir, output)
}

#[test]
fn scaffold_routes_hold_use_case_ports_not_impls() {
    let (dir, output) = scaffolded_producto("t26_routes_ports");

    let routes = fs::read_to_string(output.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    for uc in [
        "CrearProducto",
        "ObtenerProducto",
        "ListarProductos",
        "ActualizarProducto",
        "BorrarProducto",
    ] {
        assert!(routes.contains(&format!("Arc<dyn {uc}>")), "{routes}");
    }
    assert!(!routes.contains("Impl"), "{routes}");
    assert!(!routes.contains("application::"), "{routes}");

    cleanup(&dir);
}

#[test]
fn scaffold_use_case_traits_ship_a_mock() {
    let (dir, output) = scaffolded_producto("t26_trait_mocks");

    for (module, uc) in [
        ("crear_producto", "CrearProducto"),
        ("obtener_producto", "ObtenerProducto"),
        ("listar_productos", "ListarProductos"),
        ("actualizar_producto", "ActualizarProducto"),
        ("borrar_producto", "BorrarProducto"),
    ] {
        let src = fs::read_to_string(
            output.join(format!("contextos/producto/aplicacion/src/{module}.rs")),
        )
        .unwrap();
        assert!(
            src.contains("#[cfg(any(test, feature = \"pruebas\"))]\npub mod dobles {"),
            "{src}"
        );
        assert!(src.contains(&format!("pub {uc} {{}}")), "{src}");
    }

    cleanup(&dir);
}

#[test]
fn object_mothers_are_available_to_other_crates_tests() {
    let (dir, output) = scaffolded_producto("t26_mothers_feature");

    let lib = fs::read_to_string(output.join("contextos/producto/dominio/src/lib.rs")).unwrap();
    assert!(
        lib.contains("#[cfg(any(test, feature = \"pruebas\"))]\npub mod pruebas {"),
        "{lib}"
    );
    let manifest =
        fs::read_to_string(output.join("contextos/producto/dominio/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("pruebas = [\"dep:mockall\"]"),
        "{manifest}"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_routes_test_every_status_path_with_mocked_use_cases() {
    let (dir, output) = scaffolded_producto("t26_routes_tests");

    let routes = fs::read_to_string(output.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(routes.contains("#[cfg(test)]\nmod tests {"), "{routes}");
    assert!(routes.contains("poem::test::TestClient") || routes.contains("test::TestClient"));
    for status in [
        "CREATED",
        "UNPROCESSABLE_ENTITY",
        "OK",
        "NOT_FOUND",
        "INTERNAL_SERVER_ERROR",
        "NO_CONTENT",
    ] {
        assert!(
            routes.contains(&format!("StatusCode::{status}")),
            "{status}: {routes}"
        );
    }
    assert!(routes.contains("MockCrearProducto"), "{routes}");
    assert!(routes.contains("MadreProducto::aleatorio()"), "{routes}");

    cleanup(&dir);
}

#[test]
fn template_greeting_api_holds_a_port_and_is_tested() {
    let dir = temp_dir("t26_saludo");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("t26-saludo", &dir).unwrap();

    let routes = fs::read_to_string(output.join("contextos/saludo/rest/src/rutas.rs")).unwrap();
    assert!(routes.contains("Arc<dyn ObtenerSaludo>"), "{routes}");
    assert!(!routes.contains("Impl"), "{routes}");
    assert!(routes.contains("MockObtenerSaludo"), "{routes}");
    let uc = fs::read_to_string(output.join("contextos/saludo/aplicacion/src/obtener_saludo.rs"))
        .unwrap();
    assert!(uc.contains("pub ObtenerSaludo {}"), "{uc}");

    cleanup(&dir);
}

#[test]
fn presentation_tests_get_business_test_utils_and_the_poem_test_client() {
    let dir = temp_dir("t26_presentation_deps");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("t26-presentation-deps", &dir).unwrap();

    let manifest = fs::read_to_string(output.join("presentacion/api/Cargo.toml")).unwrap();
    let dev = manifest
        .split("[dev-dependencies]")
        .nth(1)
        .expect("api has [dev-dependencies]");
    assert!(
        dev.contains(r#"aplicacion_saludo = { workspace = true, features = ["pruebas"] }"#),
        "{manifest}"
    );
    let rest = fs::read_to_string(output.join("contextos/saludo/rest/Cargo.toml")).unwrap();
    let rest_dev = rest.split("[dev-dependencies]").nth(1).unwrap();
    assert!(
        rest_dev.contains(r#"poem = { workspace = true, features = ["test"] }"#),
        "{rest}"
    );

    cleanup(&dir);
}

#[test]
fn generated_make_lint_lints_the_tests_too() {
    // 17.1: `clippy --workspace` alone skips `#[cfg(test)]` code — the compile matrix lints it
    // with `--all-targets`, the project's own `make lint` did not.
    let dir = temp_dir("make_lint_all_targets");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("make-lint-all-targets", &dir).unwrap();

    let makefile = fs::read_to_string(output.join("Makefile")).unwrap();
    let lint = makefile
        .lines()
        .skip_while(|l| !l.starts_with("lint:"))
        .find(|l| l.contains("clippy"))
        .unwrap();
    assert!(lint.contains("--all-targets"), "{lint}");
    cleanup(&dir);
}

#[test]
fn context_agent_rules_describe_what_a_use_case_file_holds() {
    // 17.2: it said "exactly Params + Trait", but every use case file ships its mock, and an
    // events project writes through a unit of work the tree did not show.
    let agents = include_str!("../../template/contextos/AGENTS.md");
    assert!(!agents.contains("exactly `Params struct` + `Trait`"));
    for must in [
        "dobles::Mock<Accion>",
        "unit_of_work.rs",
        "eventos.rs",
        "UnitOfWorkFactoryGrabadora",
    ] {
        assert!(agents.contains(must), "contextos/AGENTS.md misses '{must}'");
    }
}

#[test]
fn root_agent_rules_say_how_to_work_with_puerto() {
    // 17.3: the rules said what the project is, not how to work with the generator — an agent did
    // not know what `generate use-case` leaves to fill in, which files a generator rewrites, or how
    // an entity's fields change once its table exists.
    let agents = include_str!("../../template/AGENTS.md");
    for must in [
        "## Trabajar con Puerto",
        // what `generate use-case` leaves
        "todo!()",
        "#[ignore = \"escribe este test\"]",
        // generated vs yours
        "Se reescriben en cada ejecución",
        "Se escriben una vez",
        "presentacion/api/src/app.rs",
        // changing an entity's fields
        "### Cambiar los campos de una entidad",
        "puerto generar migracion",
        // the checklist, per project kind
        "puerto validar",
        "make docker-compose/up",
        "make sqlx/prepare",
        "make web/sync",
    ] {
        assert!(agents.contains(must), "root AGENTS.md misses '{must}'");
    }
}

#[test]
fn root_agent_rules_document_the_field_syntax() {
    // 17.3: the syntax lived only in `--help`, so an agent wrote plain Strings where a value
    // object or an enum was one argument away.
    let agents = include_str!("../../template/AGENTS.md");
    for form in [
        "titulo:String",
        "precio:i64!",
        "resumen:opt:String",
        "etiquetas:vec:String",
        "publicado_en:DateTime",
        "meta:map",
        "nombre:Nombre:String",
        "segundo_nombre:SegundoNombre:opt:String",
        "etiqueta:Etiqueta:vec:String",
        "estado:Estado:enum:Borrador/Publicado",
        "email:Email",
        "cliente_id:ref:Cliente",
        "revisor_id:ref:opt:Cliente",
    ] {
        assert!(
            agents.contains(form),
            "root AGENTS.md misses the form '{form}'"
        );
    }
}

#[test]
fn root_agent_rules_name_every_generate_subcommand() {
    // 17.4: `generate mail` was named only after the first mail, `generate web` and the layered
    // generators only in other files — a command an agent is not told about is code it writes by
    // hand. A new subcommand fails here until the root AGENTS.md names it.
    use clap::CommandFactory;
    let agents = include_str!("../../template/AGENTS.md");
    let cli = crate::Cli::command();
    let generate = cli.find_subcommand("generar").unwrap();
    for sub in generate.get_subcommands().map(|c| c.get_name()) {
        let named = format!("puerto generar {sub}");
        assert!(
            agents.contains(&named),
            "root AGENTS.md never names `{named}`"
        );
    }
    for top in ["puerto validar", "puerto listar"] {
        assert!(agents.contains(top), "root AGENTS.md never names `{top}`");
    }
}
