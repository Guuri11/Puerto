use super::*;

// ── Phase 6: IDE snippets ─────────────────────────────────────────────────

#[test]
fn new_project_creates_zed_snippet_file() {
    let dir = temp_dir("snippets_zed_new");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("snip-test".into()), false, &dir).unwrap();
    snippets::apply(&output, None).unwrap();

    assert!(output.join(".zed/snippets/rust.json").exists());

    cleanup(&dir);
}

#[test]
fn new_project_creates_vscode_snippet_file() {
    let dir = temp_dir("snippets_vscode_new");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = new_project_non_interactive(Some("snip-test".into()), false, &dir).unwrap();
    snippets::apply(&output, None).unwrap();

    assert!(output.join(".vscode/puerto.code-snippets").exists());

    cleanup(&dir);
}

#[test]
fn snippet_files_contain_valid_json() {
    let dir = temp_dir("snippets_valid_json");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    snippets::apply(&dir, None).unwrap();

    let zed = fs::read_to_string(dir.join(".zed/snippets/rust.json")).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&zed).is_ok(),
        "zed snippet JSON invalid"
    );

    let vscode = fs::read_to_string(dir.join(".vscode/puerto.code-snippets")).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&vscode).is_ok(),
        "vscode snippet JSON invalid"
    );

    let zed_sql = fs::read_to_string(dir.join(".zed/snippets/sql.json")).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&zed_sql).is_ok(),
        "zed sql snippet JSON invalid"
    );

    let vscode_sql = fs::read_to_string(dir.join(".vscode/puerto.sql.code-snippets")).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&vscode_sql).is_ok(),
        "vscode sql snippet JSON invalid"
    );

    cleanup(&dir);
}

#[test]
fn generate_snippets_ide_zed_creates_only_zed_file() {
    let dir = temp_dir("snippets_ide_zed");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    snippets::run(&dir, Some("zed")).unwrap();

    assert!(dir.join(".zed/snippets/rust.json").exists());
    assert!(!dir.join(".vscode/puerto.code-snippets").exists());

    cleanup(&dir);
}

#[test]
fn generate_snippets_ide_vscode_creates_only_vscode_file() {
    let dir = temp_dir("snippets_ide_vscode");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    snippets::run(&dir, Some("vscode")).unwrap();

    assert!(!dir.join(".zed/snippets/rust.json").exists());
    assert!(dir.join(".vscode/puerto.code-snippets").exists());

    cleanup(&dir);
}

#[test]
fn generate_snippets_is_idempotent() {
    let dir = temp_dir("snippets_idempotent");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    snippets::run(&dir, None).unwrap();
    snippets::run(&dir, None).unwrap();

    assert!(dir.join(".zed/snippets/rust.json").exists());
    assert!(dir.join(".vscode/puerto.code-snippets").exists());

    cleanup(&dir);
}

#[test]
fn generate_snippets_unknown_ide_returns_error() {
    let dir = temp_dir("snippets_unknown_ide");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let result = snippets::run(&dir, Some("neovim"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("IDE desconocido"));

    cleanup(&dir);
}

#[test]
fn snippets_json_contains_vo_prefixes() {
    let json: serde_json::Value =
        serde_json::from_str(snippets::SNIPPETS_JSON).expect("SNIPPETS_JSON must be valid JSON");
    let prefixes: Vec<&str> = json
        .as_object()
        .unwrap()
        .values()
        .filter_map(|v| v.get("prefix")?.as_str())
        .collect();

    for expected in &[
        "vo-string",
        "vo-numeric",
        "vo-enum",
        "vo-option-construct",
        "vo-vec-construct",
    ] {
        assert!(
            prefixes.contains(expected),
            "SNIPPETS_JSON missing prefix '{expected}'"
        );
    }
}

// ── Snippets compose into code that compiles ──────────────────────────────
//
// The IDE snippets are hand-written templates of what the generator writes. Nothing tied them to
// it, so they drifted: `NaiveDateTime` where projects use `DateTime<Utc>`, DTOs without camelCase,
// repositories that swallowed errors, a Mother with another API, an unused `Arc` import. These
// tests expand the snippets the way an IDE does and build a whole entity from them alone.

/// The snippets of `json`, by prefix: `prefix → body` (lines joined).
fn snippet_bodies(json: &str) -> std::collections::HashMap<String, String> {
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(json).unwrap();
    parsed
        .values()
        .map(|v| {
            let prefix = v["prefix"].as_str().unwrap().to_string();
            let body = v["body"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l.as_str().unwrap().replace('\t', "    "))
                .collect::<Vec<_>>()
                .join("\n");
            (prefix, body)
        })
        .collect()
}

/// Expands a snippet body like an IDE accepting every tab stop: `${n:default}` becomes
/// `values[default]` (or the default itself), `$0` disappears, `\$` is a literal `$`.
fn expand_snippet(body: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some(i) = rest.find('$') {
        if i > 0 && rest.as_bytes()[i - 1] == b'\\' {
            out.push_str(&rest[..i - 1]);
            out.push('$');
            rest = &rest[i + 1..];
            continue;
        }
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        if let Some(stripped) = after.strip_prefix('0') {
            rest = stripped;
        } else if after.starts_with('{') {
            let close = after.find('}').unwrap();
            let default = after[1..close]
                .split_once(':')
                .map(|(_, d)| d)
                .unwrap_or("");
            let value = values
                .iter()
                .find(|(k, _)| *k == default)
                .map(|(_, v)| *v)
                .unwrap_or(default);
            out.push_str(value);
            rest = &after[close + 1..];
        } else {
            out.push('$');
            rest = after;
        }
    }
    out.push_str(rest);
    out + "\n"
}

/// Placeholder values for an entity `Aparato` with a use case `create_aparato`.
const GADGET: &[(&str, &str)] = &[
    ("entidad", "aparato"),
    ("Entidad", "Aparato"),
    ("accion", "crear_aparato"),
    ("create", "crear_aparato"),
    ("Accion", "Crear"),
    ("descripcion", "Create a Aparato"),
];

fn expand(snippets: &std::collections::HashMap<String, String>, prefix: &str) -> String {
    expand_snippet(&snippets[prefix], GADGET)
}

/// Writes the domain, application and REST crates of `Aparato` from snippets alone, plus its
/// manifest entry. Their `Cargo.toml` and `lib.rs` are Puerto's (`crates::finish`): a snippet is
/// code, not a manifest. The repository is the caller's: in-memory or Postgres, then
/// [`finish_aparato`].
fn compose_aparato_from_snippets(output: &Path, db: bool) {
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    let write = |rel: &str, content: String| {
        let path = output.join("contextos/aparato").join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    };

    // Domain. The value objects need their `Invalid*` variants — the step a user does by hand.
    write(
        "dominio/src/errores.rs",
        expand(&s, "domain-errors").replace(
            "    Desconocido,\n",
            "    Desconocido,\n    #[error(\"aparato.label_invalido\")]\n    LabelInvalido,\n    #[error(\"aparato.weight_invalido\")]\n    WeightInvalido,\n    #[error(\"aparato.status_invalido\")]\n    StatusInvalido,\n",
        ),
    );
    // The test skeleton, filled in the way its tab stops invite.
    let filled_test = expand(&s, "should-do-test")
        .replace(
            "// Actuar",
            "// Actuar\n    let resultado = Aparato::new(PropsAparato { nombre: \"Lamp\".into() });",
        )
        .replace("// Comprobar", "// Comprobar\n    assert!(resultado.is_ok());");
    let model_tests = expand(&s, "cfg-test").replace(
        "    use super::*;\n",
        &format!("    use super::*;\n{filled_test}"),
    );
    write(
        "dominio/src/modelo.rs",
        format!("{}\n{model_tests}", expand(&s, "domain-model")),
    );
    write(
        "dominio/src/valores.rs",
        format!(
            "use crate::errores::ErrorAparato;\n\n{}\n{}\n{}",
            expand_snippet(
                &s["vo-string"],
                &[("Nombre", "Label"), ("Entidad", "Aparato")]
            ),
            expand_snippet(
                &s["vo-numeric"],
                &[("Nombre", "Weight"), ("Entidad", "Aparato")]
            ),
            expand_snippet(
                &s["vo-enum"],
                &[
                    ("Nombre", "Status"),
                    ("Variante1", "Active"),
                    ("Variante2", "Archived"),
                    ("Entidad", "Aparato")
                ],
            ),
        ),
    );
    write("dominio/src/repositorio.rs", expand(&s, "repository-trait"));
    write("dominio/src/pruebas/madres.rs", expand(&s, "object-mother"));

    // Application: the contract, the use case, its happy-path test and the error-path test — one
    // file, as Puerto writes it.
    let use_case = expand(&s, "app-use-case");
    let error_test = expand(&s, "app-use-case-error-test");
    let close = use_case.rfind("}\n").unwrap();
    write(
        "aplicacion/src/crear_aparato.rs",
        crate::generators::merge::merge_use_case(
            &expand(&s, "domain-use-case"),
            &format!("{}\n{error_test}}}\n", &use_case[..close]),
        ),
    );

    // REST.
    write(
        "rest/src/dto.rs",
        format!(
            "{}\n{}",
            expand(&s, "poem-dto"),
            expand(&s, "poem-request-dto")
        ),
    );
    write("rest/src/respuestas.rs", expand(&s, "poem-response-enum"));
    let mapper_test = expand_snippet(
        &s["poem-error-mapper-test"],
        &[("Entidad", "Aparato"), ("entidad", "aparato")],
    );
    write(
        "rest/src/errores.rs",
        format!(
            "{}\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{mapper_test}}}\n",
            expand(&s, "poem-error-mapper").replace(
                "        ErrorAparato::Desconocido => StatusCode::INTERNAL_SERVER_ERROR,\n",
                "        ErrorAparato::Desconocido => StatusCode::INTERNAL_SERVER_ERROR,\n        ErrorAparato::LabelInvalido | ErrorAparato::WeightInvalido | ErrorAparato::StatusInvalido => StatusCode::UNPROCESSABLE_ENTITY,\n",
            )
        ),
    );
    write("rest/src/rutas.rs", expand(&s, "poem-api-struct"));

    puerto_toml::add_entity(output, "Aparato", vec!["crear_aparato".into()], db, vec![]).unwrap();
}

/// Aparato's crates — its infrastructure written by the caller — declared, then the wiring.
fn finish_aparato(output: &Path, db: bool) {
    use crate::generators::layout::Layer;
    let deps = crate::generators::crates::ContextDeps {
        db,
        ..Default::default()
    };
    for layer in Layer::ALL {
        crate::generators::crates::finish(output, "aparato", layer, &deps).unwrap();
    }
    crate::generators::bootstrap::regenerate_bootstrap(output).unwrap();
}

#[test]
fn use_case_snippet_ships_the_mock_every_use_case_trait_has() {
    // ADR-0012: every use case trait ships `mocks::Mock{Uc}UseCase`. A web project's bootstrap.rs
    // builds the spec from those mocks, so a use case written from the snippet without one broke
    // the presentation crate's tests (E0432).
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);

    let body = expand_snippet(
        &s["domain-use-case"],
        &[
            ("entidad", "aparato"),
            ("Entidad", "Aparato"),
            ("Accion", "Crear"),
        ],
    );

    assert!(body.contains("pub mod dobles {"), "{body}");
    assert!(body.contains("pub CrearAparato {}"), "{body}");
}

#[test]
fn snippet_expansion_resolves_tab_stops_like_an_ide() {
    assert_eq!(
        expand_snippet(
            "pub struct ${1:Entidad}Api { $0 } \\$1",
            &[("Entidad", "Aparato")]
        ),
        "pub struct AparatoApi {  } $1\n"
    );
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn snippets_compose_an_entity_that_compiles_lints_and_passes_its_tests() {
    let dir = scenario_workspace("snippets_inmemory");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("compile-snippets", &dir).unwrap();
    scaffold::apply_no_demo(&output).unwrap();
    // A scaffolded entity to drop the standalone endpoint snippets into.
    scaffold::run_scaffold("Artilugio", &output, &[], OverwritePolicy::Ask).unwrap();

    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    compose_aparato_from_snippets(&output, false);
    let infra = output.join("contextos/aparato/infraestructura/src");
    fs::create_dir_all(&infra).unwrap();
    fs::write(infra.join("repositorio.rs"), expand(&s, "inmemory-repo")).unwrap();
    finish_aparato(&output, false);

    // The standalone endpoints replace the generated ones in Artilugio's rutas.rs.
    let routes_path = output.join("contextos/artilugio/rest/src/rutas.rs");
    let routes = fs::read_to_string(&routes_path).unwrap();
    let artilugio = &[
        ("entidad", "artilugio"),
        ("Entidad", "Artilugio"),
        ("accion", "crear_artilugio"),
        ("Accion", "Crear"),
        ("descripcion", "Create a Artilugio"),
    ];
    let replace_method = |src: &str, doc: &str, snippet: &str| {
        let start = src.find(doc).unwrap();
        let end = start + src[start..].find("\n    }\n").unwrap() + "\n    }\n".len();
        let indented: String = expand_snippet(snippet, artilugio)
            .lines()
            .map(|l| {
                if l.is_empty() {
                    "\n".to_string()
                } else {
                    format!("    {l}\n")
                }
            })
            .collect();
        format!("{}{}{}", &src[..start], indented.trim_start(), &src[end..])
    };
    let routes = replace_method(
        &routes,
        "/// Obtiene: Artilugio por id",
        &s["poem-endpoint-get"],
    );
    let routes = replace_method(&routes, "/// Crea: Artilugio", &s["poem-endpoint"]);
    fs::write(&routes_path, routes).unwrap();

    // Snippets are expanded by an editor, which formats on save (or `make format/fix`).
    crate::generators::format::format_project(&output).unwrap();
    assert_generated_project_is_clean(&output, &dir, "snippets_inmemory", &[]);
    cleanup(&dir);
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_snippets_compose_a_repository_that_passes_its_integration_tests() {
    let Some(server) = test_database_server() else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let dir = scenario_workspace("snippets_postgres");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("compile-snippets-pg", &dir).unwrap();
    crate::generators::project::apply_db_to_new_project(&output).unwrap();

    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    let sql = snippet_bodies(crate::snippets::SQL_SNIPPETS_JSON);
    compose_aparato_from_snippets(&output, true);
    let infra = output.join("contextos/aparato/infraestructura/src");
    fs::create_dir_all(&infra).unwrap();
    fs::write(infra.join("fila.rs"), expand(&s, "persistence-entity")).unwrap();
    fs::write(
        infra.join("repositorio.rs"),
        format!(
            "{}\n{}",
            expand(&s, "persistence-repo"),
            expand(&s, "sqlx-repo-test-module")
        ),
    )
    .unwrap();
    finish_aparato(&output, true);
    let table = &[("entidades", "aparatos"), ("columna", "nombre")];
    fs::write(
        output.join("migraciones/20260101000000_create_aparatos.sql"),
        format!(
            "{}\n{}",
            expand_snippet(&sql["migration-create-table"], table),
            expand_snippet(&sql["migration-unique-index"], table)
        ),
    )
    .unwrap();

    let database_url = format!("{server}/puerto_snippets");
    for args in [
        &["database", "drop", "-y"][..],
        &["database", "create"][..],
        &["migrate", "run", "--source", "migraciones"][..],
    ] {
        let status = std::process::Command::new("sqlx")
            .args(args)
            .env("DATABASE_URL", &database_url)
            .current_dir(&output)
            .status()
            .expect("sqlx CLI not found — cargo install sqlx-cli");
        assert!(status.success(), "`sqlx {}` failed", args.join(" "));
    }

    crate::generators::format::format_project(&output).unwrap();
    assert_generated_project_is_clean(
        &output,
        &dir,
        "snippets_postgres",
        &[("DATABASE_URL", &database_url), ("SQLX_OFFLINE", "false")],
    );
    cleanup(&dir);
}

// ── Phase 15.1: snippets for what the generators now write ────────────────

#[test]
fn snippets_cover_events_projects_and_custom_endpoints() {
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);

    for prefix in [
        "uow-write",
        "app-use-case-uow",
        "consumer-arm",
        "poem-endpoint-action",
        "poem-route-test",
    ] {
        assert!(s.contains_key(prefix), "missing snippet '{prefix}'");
    }
    // The use case over the unit of work never calls the repository's `save`: in an events
    // project the port has none.
    assert!(!s["app-use-case-uow"].contains("repository.guardar"));
    assert!(s["app-use-case-uow"].contains("uow.commit()"));
}

/// Replaces the first occurrence of `from` in the file at `rel`, which must contain it.
fn replace_in(output: &Path, rel: &str, from: &str, to: &str) {
    let path = output.join(rel);
    let src = fs::read_to_string(&path).unwrap();
    assert!(src.contains(from), "{rel} has no {from:?}");
    fs::write(&path, src.replacen(from, to, 1)).unwrap();
}

/// Indents every non-empty line of `text` by `levels` × 4 spaces.
fn indent(text: &str, levels: usize) -> String {
    let pad = "    ".repeat(levels);
    text.lines()
        .map(|l| {
            if l.is_empty() {
                "\n".to_string()
            } else {
                format!("{pad}{l}\n")
            }
        })
        .collect()
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_events_snippets_compose_a_custom_action_that_compiles_lints_and_passes_its_tests() {
    use super::compile_matrix::{EVENTS_DB, Scenario, assert_edited_postgres_scenario_is_clean};
    // `events_db`'s Producto, its custom use case `publish_producto`, the event `producto_published`
    // and the consumer `notify_new_producto` — what the user then writes comes from snippets.
    let scenario = Scenario {
        name: "snippets_events",
        no_web: true,
        ..EVENTS_DB
    };
    assert_edited_postgres_scenario_is_clean(&scenario, "puerto_snippets_events", |output| {
        compose_publish_from_snippets(output);
    });
}

/// Writes what `generate use-case publish_producto` leaves to the user — the use case, its
/// endpoint, response and route test — and a consumer arm, from snippets alone.
fn compose_publish_from_snippets(output: &Path) {
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    let publish = &[
        ("entidad", "producto"),
        ("Entidad", "Producto"),
        ("accion", "publish_producto"),
        ("Accion", "Publish"),
        ("Publicado", "Published"),
        ("evento", "producto_published"),
        ("entidades", "productos"),
        ("verbo", "publish"),
        ("se_publica", "published"),
    ];

    // The use case acts on one producto: its params are the id (the step a user does by hand). Its
    // file is the contract, then the implementation (ADR-0023 § 4).
    let contract = expand_snippet(&s["domain-use-case"], publish)
        .replace("pub nombre: String,", "pub id: IdProducto,")
        .replace("modelo::Producto}", "modelo::{Producto, IdProducto}}");
    fs::write(
        output.join("contextos/producto/aplicacion/src/publish_producto.rs"),
        crate::generators::merge::merge_use_case(
            &contract,
            &expand_snippet(&s["app-use-case-uow"], publish),
        ),
    )
    .unwrap();

    // `uow-write` in place of the generated write block of update_producto.
    let update = "contextos/producto/aplicacion/src/actualizar_producto.rs";
    let src = fs::read_to_string(output.join(update)).unwrap();
    let start = src.find("        let mut uow = ").unwrap();
    let end_marker = "uow.commit().await.inspect_err(fallido)?;\n";
    let end = start + src[start..].find(end_marker).unwrap() + end_marker.len();
    let write = expand_snippet(
        &s["uow-write"],
        &[("accion", "actualizar_producto"), ("Entidad", "Producto")],
    );
    fs::write(
        output.join(update),
        format!("{}{}{}", &src[..start], indent(&write, 2), &src[end..]),
    )
    .unwrap();

    // The endpoint, its response and its route test.
    let responses = "contextos/producto/rest/src/respuestas.rs";
    let response = expand_snippet(
        &s["poem-response-enum"],
        &[
            ("entidad", "producto"),
            ("Entidad", "Producto"),
            ("Accion", "Publish"),
            ("201", "200"),
            ("Creado", "Ok"),
        ],
    );
    let enum_only = &response[response.find("#[derive(ApiResponse)]").unwrap()..];
    let src = fs::read_to_string(output.join(responses)).unwrap();
    fs::write(output.join(responses), format!("{src}\n{enum_only}")).unwrap();

    let routes = "contextos/producto/rest/src/rutas.rs";
    replace_in(
        output,
        routes,
        "use crate::errores::a_respuesta_error;\n",
        "use crate::errores::a_respuesta_error;\nuse crate::respuestas::RespuestaPublishProducto;\nuse aplicacion_producto::publish_producto::ParamsPublishProducto;\n",
    );
    replace_in(
        output,
        routes,
        "impl ApiProducto {\n",
        &format!(
            "impl ApiProducto {{\n{}\n",
            indent(&expand_snippet(&s["poem-endpoint-action"], publish), 1)
        ),
    );
    let src = fs::read_to_string(output.join(routes)).unwrap();
    let close = src.trim_end().rfind('}').unwrap();
    let test = expand_snippet(
        &s["poem-route-test"],
        &[
            ("200", "200"),
            ("entidad", "producto"),
            ("se_publica", "published"),
            ("accion", "publish_producto"),
            ("Entidad", "Producto"),
            ("entidades", "productos"),
            ("verbo", "publish"),
            ("OK", "OK"),
        ],
    );
    fs::write(
        output.join(routes),
        format!("{}\n{}}}\n", &src[..close], indent(&test, 1)),
    )
    .unwrap();

    // The consumer's `producto_published` arm, rewritten from `consumer-arm`.
    let consumer = "presentacion/worker/src/consumidores/notify_new_producto.rs";
    let src = fs::read_to_string(output.join(consumer)).unwrap();
    let start = src.find("            PRODUCTO_PUBLISHED => ").unwrap();
    let end_marker = "MOTIVO_PAYLOAD_INVALIDO, err.detalle),\n            },\n";
    let end = start + src[start..].find(end_marker).unwrap() + end_marker.len();
    let arm = expand_snippet(
        &s["consumer-arm"],
        &[
            ("ENTIDAD", "PRODUCTO"),
            ("VERBO", "PUBLISHED"),
            ("Entidad", "Producto"),
            ("entidad", "producto"),
            ("consumidor", "notify_new_producto"),
            ("evento", "producto_published"),
        ],
    );
    fs::write(
        output.join(consumer),
        format!("{}{}{}", &src[..start], indent(&arm, 3), &src[end..]),
    )
    .unwrap();

    // Snippets are expanded by an editor, which formats on save (or `make format/fix`).
    crate::generators::format::format_project(output).unwrap();
}

#[test]
fn web_projects_get_the_frontend_snippets() {
    let dir = temp_dir("snippets_web_files");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let web = new_project_non_interactive(Some("snip-web".into()), false, &dir).unwrap();
    snippets::apply(&web, None).unwrap();

    for file in [".zed/snippets/tsx.json", ".vscode/puerto.ts.code-snippets"] {
        let json = fs::read_to_string(web.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(
            serde_json::from_str::<serde_json::Value>(&json).is_ok(),
            "{file} is not valid JSON"
        );
    }
    let ts =
        snippet_bodies(&fs::read_to_string(web.join(".vscode/puerto.ts.code-snippets")).unwrap());
    for prefix in [
        "web-container",
        "web-container-test",
        "web-form-field",
        "web-route",
    ] {
        assert!(ts.contains_key(prefix), "missing snippet '{prefix}'");
    }

    cleanup(&dir);
}

#[test]
fn projects_without_web_get_no_frontend_snippets() {
    let dir = temp_dir("snippets_no_web_files");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("snip-no-web".into()), false, &dir).unwrap();
    scaffold::apply_no_web(&output).unwrap();
    snippets::apply(&output, None).unwrap();

    assert!(!output.join(".zed/snippets/tsx.json").exists());
    assert!(!output.join(".vscode/puerto.ts.code-snippets").exists());

    cleanup(&dir);
}

#[test]
#[ignore = "slow: compiles a generated project, then installs, lints and tests its frontend"]
fn web_snippets_compose_a_screen_that_passes_the_frontend_checks() {
    let dir = scenario_workspace("snippets_web");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("compile-snippets-web", &dir).unwrap();
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    let tsx = snippet_bodies(crate::snippets::WEB_TSX_SNIPPETS_JSON);
    let producto = &[
        ("Entidad", "Producto"),
        ("entidad", "producto"),
        ("nombre-entidad", "producto"),
        ("Resumen", "Resumen"),
        ("resumen", "resumen"),
        ("qué", "el resumen"),
        ("entidades", "productos"),
        ("/entidades/:id/resumen", "/productos/:id/resumen"),
    ];
    let web = "web/src";

    // A screen, its test, and its route.
    let containers = format!("{web}/contextos/producto/contenedores");
    fs::write(
        output.join(format!("{containers}/resumen-producto.tsx")),
        expand_snippet(&tsx["web-container"], producto),
    )
    .unwrap();
    fs::write(
        output.join(format!("{containers}/resumen-producto.test.tsx")),
        expand_snippet(&tsx["web-container-test"], producto),
    )
    .unwrap();
    let route = expand_snippet(&tsx["web-route"], producto);
    let routes = format!("{web}/rutas.tsx");
    replace_in(
        &output,
        &routes,
        "export const rutasPropias: ReactElement[] = [];",
        &format!("export const rutasPropias: ReactElement[] = [\n{route}];"),
    );
    replace_in(
        &output,
        &routes,
        "import type { ReactElement } from \"react\";\n",
        "import type { ReactElement } from \"react\";\nimport { Route } from \"wouter\";\nimport { ContenedorResumenProducto } from \"./contextos/producto/contenedores/resumen-producto\";\n",
    );

    // The route is reached through the generated table: `{customRoutes}` inside its `<Switch>`.
    fs::write(
        output.join(format!("{web}/rutas.test.tsx")),
        r#"import { expect, test } from "bun:test";
import { screen } from "@testing-library/react";
import { RutasApp } from "./generado/rutas";
import { apiFalsa } from "./pruebas/api-falsa";
import { coleccionProducto, MadreProducto } from "./pruebas/producto";
import { renderizarPantalla } from "./pruebas/renderizar";

test("debería llegar a una ruta propia por la tabla de rutas generada", async () => {
  // Preparar
  const sembrado = MadreProducto.aleatorio();
  const api = apiFalsa([coleccionProducto([sembrado])]);

  // Actuar
  renderizarPantalla(<RutasApp />, { api }, `/productos/${sembrado.id}/resumen`);

  // Comprobar
  expect(await screen.findByText(sembrado.id)).toBeTruthy();
});
"#,
    )
    .unwrap();

    // The form's `nombre` input, rewritten from `web-form-field`.
    let form = format!("{web}/contextos/producto/vistas/formulario-producto.tsx");
    let src = fs::read_to_string(output.join(&form)).unwrap();
    let start = src.find("      <label className=\"fieldset\">\n        <span className=\"fieldset-legend\">Nombre</span>").unwrap();
    let field_error = "<ErrorCampo problema={problema} campo=\"nombre\" />\n";
    let end = start + src[start..].find(field_error).unwrap() + field_error.len();
    let field = expand_snippet(
        &tsx["web-form-field"],
        &[
            ("Etiqueta", "Nombre"),
            ("text", "text"),
            ("campo", "nombre"),
        ],
    );
    fs::write(
        output.join(&form),
        format!("{}{field}{}", &src[..start], &src[end..]),
    )
    .unwrap();

    // An editor formats on save; Biome is the project's formatter.
    for args in [&["install"][..], &["run", "format"][..]] {
        let status = std::process::Command::new("bun")
            .args(args)
            .current_dir(output.join("web"))
            .status()
            .expect("bun not found — the web scenarios need it on PATH (https://bun.sh)");
        assert!(status.success(), "`bun {}` failed", args.join(" "));
    }
    crate::generators::format::format_project(&output).unwrap();
    assert_generated_project_is_clean(&output, &dir, "snippets_web", &[]);
    cleanup(&dir);
}
