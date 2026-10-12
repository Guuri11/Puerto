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
    assert!(result.unwrap_err().to_string().contains("unknown IDE"));

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

/// Placeholder values for an entity `Gadget` with a use case `create_gadget`.
const GADGET: &[(&str, &str)] = &[
    ("entity", "gadget"),
    ("Entity", "Gadget"),
    ("action", "create_gadget"),
    ("create", "create_gadget"),
    ("Action", "Create"),
    ("description", "Create a Gadget"),
];

fn expand(snippets: &std::collections::HashMap<String, String>, prefix: &str) -> String {
    expand_snippet(&snippets[prefix], GADGET)
}

/// Writes the domain, application and presentation of `Gadget` from snippets alone, plus its
/// manifest entry and bootstrap. The repository is the caller's: in-memory or Postgres.
fn compose_gadget_from_snippets(output: &Path, db: bool) {
    use crate::patchers::lib_rs::insert_before_block_end;
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    let write = |rel: &str, content: String| {
        let path = output.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    };

    // business/src/lib.rs — the module tree, plus the value objects module.
    let lib_path = output.join("business/src/lib.rs");
    let lib = fs::read_to_string(&lib_path).unwrap();
    let domain_block = expand(&s, "lib-domain-entity").replace(
        "pub mod model;",
        "pub mod model;\n    pub mod value_objects;",
    );
    let lib = insert_before_block_end(&lib, "domain", &domain_block).unwrap();
    let lib = insert_before_block_end(&lib, "application", &expand(&s, "lib-application-entity"))
        .unwrap();
    fs::write(&lib_path, lib).unwrap();

    // Domain. The value objects need their `Invalid*` variants — the step a user does by hand.
    write(
        "business/src/domain/gadget/errors.rs",
        expand(&s, "domain-errors").replace(
            "    Unknown,\n",
            "    Unknown,\n    #[error(\"gadget.invalid_label\")]\n    InvalidLabel,\n    #[error(\"gadget.invalid_weight\")]\n    InvalidWeight,\n    #[error(\"gadget.invalid_status\")]\n    InvalidStatus,\n",
        ),
    );
    // The test skeleton, filled in the way its tab stops invite.
    let filled_test = expand(&s, "should-do-test")
        .replace(
            "// Act",
            "// Act\n    let result = Gadget::new(GadgetProps { name: \"Lamp\".into() });",
        )
        .replace("// Assert", "// Assert\n    assert!(result.is_ok());");
    let model_tests = expand(&s, "cfg-test").replace(
        "    use super::*;\n",
        &format!("    use super::*;\n{filled_test}"),
    );
    write(
        "business/src/domain/gadget/model.rs",
        format!("{}\n{model_tests}", expand(&s, "domain-model")),
    );
    write(
        "business/src/domain/gadget/value_objects.rs",
        format!(
            "use super::errors::GadgetError;\n\n{}\n{}\n{}",
            expand_snippet(&s["vo-string"], &[("Name", "Label"), ("Entity", "Gadget")]),
            expand_snippet(
                &s["vo-numeric"],
                &[("Name", "Weight"), ("Entity", "Gadget")]
            ),
            expand_snippet(
                &s["vo-enum"],
                &[
                    ("Name", "Status"),
                    ("Variant1", "Active"),
                    ("Variant2", "Archived"),
                    ("Entity", "Gadget")
                ],
            ),
        ),
    );
    write(
        "business/src/domain/gadget/repository.rs",
        expand(&s, "repository-trait"),
    );
    write(
        "business/src/domain/gadget/use_cases/create_gadget.rs",
        expand(&s, "domain-use-case"),
    );

    // Application: the use case, its happy-path test and the error-path test.
    let use_case = expand(&s, "app-use-case");
    let error_test = expand(&s, "app-use-case-error-test");
    let close = use_case.rfind("}\n").unwrap();
    write(
        "business/src/application/gadget/create_gadget.rs",
        format!("{}\n{error_test}}}\n", &use_case[..close]),
    );

    // Object Mother.
    write(
        "business/src/tests/mothers/gadget_mother.rs",
        expand(&s, "object-mother"),
    );
    crate::patchers::lib_rs::patch_mothers_lib(output, "gadget").unwrap();

    // Presentation.
    crate::patchers::api_rs::patch_api_rs(output, "gadget").unwrap();
    write(
        "presentation/src/api/gadget.rs",
        expand(&s, "lib-presentation-entity"),
    );
    write(
        "presentation/src/api/gadget/dto.rs",
        format!(
            "{}\n{}",
            expand(&s, "poem-dto"),
            expand(&s, "poem-request-dto")
        ),
    );
    write(
        "presentation/src/api/gadget/responses.rs",
        expand(&s, "poem-response-enum"),
    );
    let mapper_test = expand_snippet(
        &s["poem-error-mapper-test"],
        &[("Entity", "Gadget"), ("entity", "gadget")],
    );
    write(
        "presentation/src/api/gadget/error_mapper.rs",
        format!(
            "{}\n#[cfg(test)]\nmod tests {{\n    use super::*;\n\n{mapper_test}}}\n",
            expand(&s, "poem-error-mapper").replace(
                "            GadgetError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,\n",
                "            GadgetError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,\n            GadgetError::InvalidLabel | GadgetError::InvalidWeight | GadgetError::InvalidStatus => StatusCode::UNPROCESSABLE_ENTITY,\n",
            )
        ),
    );
    write(
        "presentation/src/api/gadget/routes.rs",
        expand(&s, "poem-api-struct"),
    );

    puerto_toml::add_entity(output, "Gadget", vec!["create_gadget".into()], db, vec![]).unwrap();
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
            ("entity", "gadget"),
            ("Entity", "Gadget"),
            ("Action", "Create"),
        ],
    );

    assert!(body.contains("pub mod mocks {"), "{body}");
    assert!(body.contains("pub CreateGadgetUseCase {}"), "{body}");
}

#[test]
fn snippet_expansion_resolves_tab_stops_like_an_ide() {
    assert_eq!(
        expand_snippet(
            "pub struct ${1:Entity}Api { $0 } \\$1",
            &[("Entity", "Gadget")]
        ),
        "pub struct GadgetApi {  } $1\n"
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
    scaffold::run_scaffold("Widget", &output, &[], OverwritePolicy::Ask).unwrap();

    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    compose_gadget_from_snippets(&output, false);
    fs::write(
        output.join("infrastructure/src/lib.rs"),
        format!(
            "{}{}",
            fs::read_to_string(output.join("infrastructure/src/lib.rs")).unwrap(),
            expand(&s, "lib-infra-entity")
        ),
    )
    .unwrap();
    fs::create_dir_all(output.join("infrastructure/src/gadget")).unwrap();
    fs::write(
        output.join("infrastructure/src/gadget/repository.rs"),
        expand(&s, "inmemory-repo"),
    )
    .unwrap();

    // The standalone endpoints replace the generated ones in Widget's routes.rs.
    let routes_path = output.join("presentation/src/api/widget/routes.rs");
    let routes = fs::read_to_string(&routes_path).unwrap();
    let widget = &[
        ("entity", "widget"),
        ("Entity", "Widget"),
        ("action", "create_widget"),
        ("Action", "Create"),
        ("description", "Create a Widget"),
    ];
    let replace_method = |src: &str, doc: &str, snippet: &str| {
        let start = src.find(doc).unwrap();
        let end = start + src[start..].find("\n    }\n").unwrap() + "\n    }\n".len();
        let indented: String = expand_snippet(snippet, widget)
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
    let routes = replace_method(&routes, "/// Get a Widget by ID", &s["poem-endpoint-get"]);
    let routes = replace_method(&routes, "/// Create a new Widget", &s["poem-endpoint"]);
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
    compose_gadget_from_snippets(&output, true);
    crate::patchers::lib_rs::patch_infra_lib(&output, "gadget", true, true).unwrap();
    fs::create_dir_all(output.join("infrastructure/src/gadget")).unwrap();
    fs::write(
        output.join("infrastructure/src/gadget/entity.rs"),
        expand(&s, "persistence-entity"),
    )
    .unwrap();
    fs::write(
        output.join("infrastructure/src/gadget/repository.rs"),
        format!(
            "{}\n{}",
            expand(&s, "persistence-repo"),
            expand(&s, "sqlx-repo-test-module")
        ),
    )
    .unwrap();
    let table = &[("entities", "gadgets"), ("column", "name")];
    fs::write(
        output.join("infrastructure/migrations/20260101000000_create_gadgets.sql"),
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
        &["migrate", "run", "--source", "infrastructure/migrations"][..],
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
    assert!(!s["app-use-case-uow"].contains("repository.save"));
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
    // `events_db`'s Product, its custom use case `publish_product`, the event `product_published`
    // and the consumer `notify_new_product` — what the user then writes comes from snippets.
    let scenario = Scenario {
        name: "snippets_events",
        no_web: true,
        ..EVENTS_DB
    };
    assert_edited_postgres_scenario_is_clean(&scenario, "puerto_snippets_events", |output| {
        compose_publish_from_snippets(output);
    });
}

/// Writes what `generate use-case publish_product` leaves to the user — the use case, its
/// endpoint, response and route test — and a consumer arm, from snippets alone.
fn compose_publish_from_snippets(output: &Path) {
    let s = snippet_bodies(crate::snippets::SNIPPETS_JSON);
    let publish = &[
        ("entity", "product"),
        ("Entity", "Product"),
        ("action", "publish_product"),
        ("Action", "Publish"),
        ("Published", "Published"),
        ("event", "product_published"),
        ("entities", "products"),
        ("verb", "publish"),
        ("done", "published"),
    ];

    // The use case acts on one product: its params are the id (the step a user does by hand).
    replace_in(
        output,
        "business/src/domain/product/use_cases/publish_product.rs",
        "pub name: String,",
        "pub id: ProductId,",
    );
    replace_in(
        output,
        "business/src/domain/product/use_cases/publish_product.rs",
        "model::Product}",
        "model::{Product, ProductId}}",
    );
    fs::write(
        output.join("business/src/application/product/publish_product.rs"),
        expand_snippet(&s["app-use-case-uow"], publish),
    )
    .unwrap();

    // `uow-write` in place of the generated write block of update_product.
    let update = "business/src/application/product/update_product.rs";
    let src = fs::read_to_string(output.join(update)).unwrap();
    let start = src.find("        let mut uow = ").unwrap();
    let end_marker = "uow.commit().await.inspect_err(failed)?;\n";
    let end = start + src[start..].find(end_marker).unwrap() + end_marker.len();
    let write = expand_snippet(
        &s["uow-write"],
        &[("action", "update_product"), ("Entity", "Product")],
    );
    fs::write(
        output.join(update),
        format!("{}{}{}", &src[..start], indent(&write, 2), &src[end..]),
    )
    .unwrap();

    // The endpoint, its response and its route test.
    let responses = "presentation/src/api/product/responses.rs";
    let response = expand_snippet(
        &s["poem-response-enum"],
        &[
            ("entity", "product"),
            ("Entity", "Product"),
            ("Action", "Publish"),
            ("201", "200"),
            ("Created", "Ok"),
        ],
    );
    let enum_only = &response[response.find("#[derive(ApiResponse)]").unwrap()..];
    let src = fs::read_to_string(output.join(responses)).unwrap();
    fs::write(output.join(responses), format!("{src}\n{enum_only}")).unwrap();

    let routes = "presentation/src/api/product/routes.rs";
    replace_in(
        output,
        routes,
        "use crate::api::tags::ApiTags;\n",
        "use crate::api::tags::ApiTags;\nuse crate::api::product::responses::PublishProductResponse;\nuse business::domain::product::use_cases::publish_product::PublishProductParams;\n",
    );
    replace_in(
        output,
        routes,
        "impl ProductApi {\n",
        &format!(
            "impl ProductApi {{\n{}\n",
            indent(&expand_snippet(&s["poem-endpoint-action"], publish), 1)
        ),
    );
    let src = fs::read_to_string(output.join(routes)).unwrap();
    let close = src.trim_end().rfind('}').unwrap();
    let test = expand_snippet(
        &s["poem-route-test"],
        &[
            ("200", "200"),
            ("entity", "product"),
            ("done", "published"),
            ("action", "publish_product"),
            ("Entity", "Product"),
            ("entities", "products"),
            ("verb", "publish"),
            ("OK", "OK"),
        ],
    );
    fs::write(
        output.join(routes),
        format!("{}\n{}}}\n", &src[..close], indent(&test, 1)),
    )
    .unwrap();

    // The consumer's `product_published` arm, rewritten from `consumer-arm`.
    let consumer = "worker/src/consumers/notify_new_product.rs";
    let src = fs::read_to_string(output.join(consumer)).unwrap();
    let start = src.find("            PRODUCT_PUBLISHED => ").unwrap();
    let end_marker = "REASON_PAYLOAD_INVALID, err.detail),\n            },\n";
    let end = start + src[start..].find(end_marker).unwrap() + end_marker.len();
    let arm = expand_snippet(
        &s["consumer-arm"],
        &[
            ("ENTITY", "PRODUCT"),
            ("VERB", "PUBLISHED"),
            ("Entity", "Product"),
            ("entity", "product"),
            ("consumer", "notify_new_product"),
            ("event", "product_published"),
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

    for file in [
        ".zed/snippets/typescript.json",
        ".zed/snippets/tsx.json",
        ".vscode/puerto.ts.code-snippets",
    ] {
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
        "web-repo-method",
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
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();
    let ts = snippet_bodies(crate::snippets::WEB_TS_SNIPPETS_JSON);
    let tsx = snippet_bodies(crate::snippets::WEB_TSX_SNIPPETS_JSON);
    let product = &[
        ("Entity", "Product"),
        ("entity", "product"),
        ("entity-name", "product"),
        ("Summary", "Summary"),
        ("summary", "summary"),
        ("entity label", "product"),
        ("method", "refresh"),
        ("entities", "products"),
        ("/entities/:id/summary", "/products/:id/summary"),
    ];
    let web = "web/src";

    // A repository method: on the port, the fake and the HTTP adapter.
    replace_in(
        &output,
        &format!("{web}/domain/product/repository.ts"),
        "  get(id: string): Promise<Product>;\n",
        "  get(id: string): Promise<Product>;\n  refresh(id: string): Promise<Product>;\n",
    );
    replace_in(
        &output,
        &format!("{web}/test-support/product.ts"),
        "    get: async (id) => find(id),\n",
        "    get: async (id) => find(id),\n    refresh: async (id) => find(id),\n",
    );
    replace_in(
        &output,
        &format!("{web}/infrastructure/product/product-repository-http.ts"),
        "ProductRepository {\n  return {\n",
        &format!(
            "ProductRepository {{\n  return {{\n{}",
            expand_snippet(&ts["web-repo-method"], product)
        ),
    );

    // A screen, its test, and its route.
    let containers = format!("{web}/presentation/features/product/containers");
    fs::write(
        output.join(format!("{containers}/product-summary-container.tsx")),
        expand_snippet(&tsx["web-container"], product),
    )
    .unwrap();
    fs::write(
        output.join(format!("{containers}/product-summary-container.test.tsx")),
        expand_snippet(&tsx["web-container-test"], product),
    )
    .unwrap();
    let route = expand_snippet(&tsx["web-route"], product);
    let routes = format!("{web}/presentation/core/routes.tsx");
    replace_in(
        &output,
        &routes,
        "export const customRoutes: ReactElement[] = [];",
        &format!("export const customRoutes: ReactElement[] = [\n{route}];"),
    );
    replace_in(
        &output,
        &routes,
        "import type { ReactElement } from \"react\";\n",
        "import type { ReactElement } from \"react\";\nimport { Route } from \"wouter\";\nimport { ProductSummaryContainer } from \"../features/product/containers/product-summary-container\";\n",
    );

    // The route is reached through the generated table: `{customRoutes}` inside its `<Switch>`.
    fs::write(
        output.join(format!("{web}/presentation/core/routes.test.tsx")),
        r#"import { expect, test } from "bun:test";
import { screen } from "@testing-library/react";
import { fakeProductRepository, ProductMother } from "../../test-support/product";
import { renderScreen } from "../../test-support/render";
import { AppRoutes } from "./generated/routes";

test("should reach a custom route through the generated route table", async () => {
  // Arrange
  const seeded = ProductMother.random();
  const productRepository = fakeProductRepository([seeded]);

  // Act
  renderScreen(<AppRoutes />, { productRepository }, `/products/${seeded.id}/summary`);

  // Assert
  expect(await screen.findByText(seeded.id)).toBeTruthy();
});
"#,
    )
    .unwrap();

    // The form's `name` input, rewritten from `web-form-field`.
    let form = format!("{web}/presentation/features/product/views/product-form-view.tsx");
    let src = fs::read_to_string(output.join(&form)).unwrap();
    let start = src.find("      <label className=\"fieldset\">\n        <span className=\"fieldset-legend\">Name</span>").unwrap();
    let end = start + src[start..].find("</label>\n").unwrap() + "</label>\n".len();
    let field = expand_snippet(
        &tsx["web-form-field"],
        &[("Label", "Name"), ("text", "text"), ("field", "name")],
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
