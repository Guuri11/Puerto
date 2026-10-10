use super::*;

// ── T-07: compile matrix ──────────────────────────────────────────────────
//
// The structural tests above assert on substrings of the generated *text*. Nothing there proves
// the text is valid Rust — which is how `Option<VO>`/`Vec<VO>` (T-01), `--no-demo` (T-02) and
// use-case-less entities (T-03) shipped broken. These scenarios generate a real project, scaffold
// into it, and run `cargo test --workspace` inside it: the only assertion that cannot be fooled.
//
// They are `#[ignore]`d (slow) and run via `make test/full`.

/// One end-to-end generation scenario.
struct Scenario<'a> {
    /// Temp-dir suffix; must be unique per scenario.
    name: &'a str,
    /// Strip the Greeting demo before scaffolding (`puerto new --no-demo`).
    no_demo: bool,
    /// Shared VOs declared in puerto.toml before any scaffold (`(name, inner_type)`).
    shared_vos: &'a [(&'a str, &'a str)],
    /// Entities to scaffold: `(PascalName, CLI field args)`.
    entities: &'a [(&'a str, &'a [&'a str])],
    /// Custom use cases added after scaffolding: `(PascalEntity, snake_action)`.
    use_cases: &'a [(&'a str, &'a str)],
}

/// Generates the project, applies the scenario, and asserts the result compiles, **passes its own
/// `make lint`** and **its tests pass**. `cargo check` is not enough: T-01 only broke test targets,
/// and every generated project ships a `make lint` with `-D warnings` that a fresh `puerto new`
/// used to fail.
fn assert_scenario_compiles(scenario: &Scenario) {
    let dir = scenario_workspace(scenario.name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project(&format!("compile-{}", scenario.name), &dir).unwrap();
    apply_scenario(scenario, &output);
    assert_generated_project_is_clean(&output, &dir, scenario.name, &[]);
    cleanup(&dir);
}

/// Applies the scenario's shared VOs, entities and custom use cases to a generated project.
fn apply_scenario(scenario: &Scenario, output: &Path) {
    if scenario.no_demo {
        scaffold::apply_no_demo(output).unwrap();
    }

    for (vo_name, inner) in scenario.shared_vos {
        puerto_toml::add_value_object(output, vo_name, inner).unwrap();
    }

    let shared_vos = puerto_toml::read(output).unwrap().value_object;

    for (entity, field_args) in scenario.entities {
        let parsed: Vec<puerto_toml::Field> = field_args
            .iter()
            .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
            .collect();
        let fields = puerto_toml::apply_shared_vo_inference(parsed, &shared_vos);
        crate::generators::types::validate_fields(&fields).unwrap();
        scaffold::run_scaffold(entity, output, &fields, OverwritePolicy::Ask).unwrap();
    }

    for (entity, action) in scenario.use_cases {
        scaffold::run_use_case(entity, action, output).unwrap();
    }
}

/// Runs `cargo clippy --all-targets -D warnings` and `cargo test` inside a generated project.
///
/// A target dir **per scenario**: every generated project names its crates
/// `business`/`infrastructure`/`presentation`, so a shared target dir has them overwrite each
/// other's artifacts and fail for reasons unrelated to the code under test.
pub(super) fn assert_generated_project_is_clean(
    output: &Path,
    dir: &Path,
    name: &str,
    envs: &[(&str, &str)],
) {
    for args in [
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ][..],
        &["test", "--workspace"][..],
    ] {
        let result = std::process::Command::new("cargo")
            .args(args)
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .envs(envs.iter().copied())
            .current_dir(output)
            .output()
            .expect("failed to run cargo");

        if !result.status.success() {
            eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
            eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
            panic!(
                "scenario '{name}': generated project failed `cargo {}`",
                args.join(" ")
            );
        }
    }
}

/// Workspace for one scenario: the generated project plus its cargo target dir.
///
/// Lives under Puerto's own `target/`, not `/tmp`: a full build tree per scenario is gigabytes,
/// and on a tmpfs `/tmp` the matrix hits "Disk quota exceeded" instead of reporting on the code.
/// `cleanup()` on success keeps the total bounded by how many scenarios run concurrently; a
/// failing scenario keeps its tree for inspection.
pub(super) fn scenario_workspace(scenario: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/compile-matrix")
        .join(scenario);
    let _ = fs::create_dir_all(&root);
    root
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_no_demo_then_scaffold() {
    assert_scenario_compiles(&Scenario {
        name: "no_demo",
        no_demo: true,
        shared_vos: &[],
        use_cases: &[],
        entities: &[("Persona", &["name:String"])],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_all_primitive_field_types() {
    assert_scenario_compiles(&Scenario {
        name: "primitives",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[(
            "Product",
            &[
                "title:String",
                "price:i64",
                "in_stock:bool",
                "rate:f64",
                "released_at:DateTime",
                "meta:map",
                // A `Uuid` field (the usual foreign key) was missing from this matrix, which is how
                // the generators shipped dropping it from every params struct and DTO.
                "category_id:Uuid",
            ],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_uuid_value_object() {
    assert_scenario_compiles(&Scenario {
        name: "uuid_vo",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[("Booking", &["reference:String", "tenant_id:TenantId:Uuid"])],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_option_and_vec_primitives() {
    assert_scenario_compiles(&Scenario {
        name: "opt_vec_prim",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[(
            "Article",
            &[
                "title:String",
                "summary:opt:String",
                "views:opt:i64",
                "tags:vec:String",
                "scores:vec:i64",
            ],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_plain_and_unique_value_objects() {
    assert_scenario_compiles(&Scenario {
        name: "plain_vo",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[(
            "Item",
            &["name:Name:String", "sku:Sku:String!", "weight:Weight:i64"],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_option_and_vec_value_objects() {
    assert_scenario_compiles(&Scenario {
        name: "opt_vec_vo",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[(
            "Persona",
            &[
                "name:String",
                "middle_name:MiddleName:opt:String",
                "tags:Tag:vec:String",
            ],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_enum_value_object() {
    assert_scenario_compiles(&Scenario {
        name: "enum_vo",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[(
            "Order",
            &[
                "reference:String",
                "status:Status:enum:Pending/Confirmed/Cancelled",
            ],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_shared_value_objects() {
    assert_scenario_compiles(&Scenario {
        name: "shared_vo",
        no_demo: false,
        shared_vos: &[("Email", "String"), ("Money", "i64")],
        use_cases: &[],
        entities: &[("Customer", &["name:String", "email:Email", "credit:Money"])],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_multiple_entities() {
    assert_scenario_compiles(&Scenario {
        name: "multi_entity",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[
            ("Product", &["title:String", "price:i64"]),
            ("Category", &["name:Name:String"]),
        ],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_every_value_object_kind_at_once() {
    assert_scenario_compiles(&Scenario {
        name: "vo_kitchen_sink",
        no_demo: false,
        shared_vos: &[("Email", "String")],
        use_cases: &[],
        entities: &[(
            "Account",
            &[
                "title:String",
                "price:i64",
                "name:Name:String",
                "sku:Sku:String!",
                "note:opt:String",
                "middle_name:MiddleName:opt:String",
                "tags:Tag:vec:String",
                "status:Status:enum:Active/Archived",
                "email:Email",
            ],
        )],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_domain_first_layered_flow_and_fields() {
    // T-06: `generate domain -- fields` persists the fields, so the three later layered commands
    // read them back from puerto.toml. Before, every one of them was field-blind and this flow
    // could only ever produce the default `name: String`.
    let dir = scenario_workspace("layered_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("compile-layered-fields", &dir).unwrap();

    let fields: Vec<puerto_toml::Field> = [
        "amount:i64",
        "reference:Reference:String",
        "note:opt:String",
    ]
    .iter()
    .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
    .collect();

    scaffold::run_generate_domain("Invoice", &output, &fields).unwrap();
    scaffold::run_generate_application("Invoice", &output).unwrap();
    scaffold::run_generate_repository("Invoice", &output).unwrap();
    scaffold::run_generate_presentation("Invoice", &output).unwrap();

    assert_generated_project_is_clean(&output, &dir, "layered_fields", &[]);

    cleanup(&dir);
}

// ── T-13: review fixes — matrix gaps ──────────────────────────────────────

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_enum_vo_as_only_string_field() {
    // The create test asserted `.value()` on the first String-typed field — an enum VO has
    // `as_str()`, so an entity whose only String was an enum never compiled its tests.
    assert_scenario_compiles(&Scenario {
        name: "enum_only",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[],
        entities: &[("Ticket", &["state:State:enum:Open/Closed"])],
    });
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_custom_use_case_after_scaffold() {
    // `generate use-case` on a scaffolded entity wired the use case into `ProductApi` in
    // bootstrap.rs while routes.rs had no such field (E0560).
    assert_scenario_compiles(&Scenario {
        name: "custom_use_case",
        no_demo: false,
        shared_vos: &[],
        use_cases: &[("Product", "archive_product")],
        entities: &[("Product", &["title:String", "price:i64"])],
    });
}

/// `PUERTO_TEST_DATABASE_URL` — a Postgres server URL **without** a database name, e.g.
/// `postgres://postgres:pg@localhost:55432`. `make test/db` starts one in Docker and sets it.
pub(super) fn test_database_server() -> Option<String> {
    std::env::var("PUERTO_TEST_DATABASE_URL")
        .ok()
        .map(|u| u.trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty())
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_project_with_every_type_compiles_lints_and_passes_integration_tests() {
    // Nothing in the matrix generated a `--db` project, which is how `HashMap` columns, the Pg
    // repository test imports, timestamp round-trips and `unique` seeds all shipped broken.
    let Some(server) = test_database_server() else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let dir = scenario_workspace("postgres_every_type");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("compile-postgres", &dir).unwrap();
    crate::generators::project::apply_db_to_new_project(&output).unwrap();
    apply_scenario(
        &Scenario {
            name: "postgres_every_type",
            no_demo: false,
            shared_vos: &[("Email", "String")],
            use_cases: &[("Product", "archive_product")],
            entities: &[
                (
                    "Product",
                    &[
                        "title:String",
                        "sku:String!",
                        "price:i64!",
                        "rating:f64",
                        "active:bool",
                        "ref_id:Uuid",
                        "published_at:DateTime",
                        "summary:opt:String",
                        "views:opt:i64",
                        "expires_at:opt:DateTime",
                        "tags:vec:String",
                        "scores:vec:i64",
                        "meta:map",
                    ],
                ),
                (
                    "Customer",
                    &[
                        "email:Email",
                        "name:Name:String",
                        "code:Code:String!",
                        "age:Age:i64",
                        "joined:Joined:DateTime",
                        "middle:MiddleName:opt:String",
                        "labels:Label:vec:String",
                        "status:Status:enum:Active/Inactive",
                    ],
                ),
                ("Ticket", &["state:State:enum:Open/Closed"]),
            ],
        },
        &output,
    );

    let database_url = format!("{server}/puerto_matrix");
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

    assert_generated_project_is_clean(
        &output,
        &dir,
        "postgres_every_type",
        &[("DATABASE_URL", &database_url), ("SQLX_OFFLINE", "false")],
    );

    cleanup(&dir);
}
