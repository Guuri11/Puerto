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
pub(super) struct Scenario<'a> {
    /// Temp-dir suffix and snapshot name; must be unique per scenario.
    pub(super) name: &'a str,
    /// A `--db` project (`puerto new --db`): Pg repositories, migrations.
    pub(super) db: bool,
    /// `puerto new --db --events <transport>`: units of work, `events.rs`, the outbox.
    pub(super) events: Option<&'a str>,
    /// `puerto new --auth <mode>`: the `User` entity, the provisioning authenticator, `GET /me`.
    pub(super) auth: Option<&'a str>,
    /// Strip the Greeting demo before scaffolding (`puerto new --no-demo`).
    pub(super) no_demo: bool,
    /// Strip the web frontend (`puerto new --no-web`).
    pub(super) no_web: bool,
    /// Shared VOs declared in puerto.toml before any scaffold (`(name, inner_type)`).
    pub(super) shared_vos: &'a [(&'a str, &'a str)],
    /// Entities to scaffold: `(PascalName, CLI field args)`.
    pub(super) entities: &'a [(&'a str, &'a [&'a str])],
    /// Auth projects: `(PascalName, --public use cases, --role use_case=role)` of a scaffold.
    pub(super) access: &'a [Access<'a>],
    /// Custom use cases added after scaffolding: `(PascalEntity, snake_action)`.
    pub(super) use_cases: &'a [(&'a str, &'a str)],
    /// Custom events declared after scaffolding (`puerto generate event`): `(PascalEntity, name)`.
    pub(super) entity_events: &'a [(&'a str, &'a str)],
    /// Worker consumers added last (`puerto generate consumer`): `(name, event types)`.
    pub(super) consumers: &'a [(&'a str, &'a [&'a str])],
    /// Mails added after the consumers (`puerto generate mail`): `(name, CLI field args, --on,
    /// --to)`.
    pub(super) mails: &'a [Mail<'a>],
}

pub(super) type Access<'a> = (&'a str, &'a [&'a str], &'a [&'a str]);

pub(super) type Mail<'a> = (&'a str, &'a [&'a str], &'a [&'a str], Option<&'a str>);

// The scenarios, shared with the snapshot tests (`generated_output.rs`): one proves the output
// compiles, the other that it did not change.

pub(super) const NO_DEMO: Scenario = Scenario {
    name: "no_demo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: true,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[("Persona", &["name:String"])],
};

pub(super) const NO_WEB: Scenario = Scenario {
    name: "no_web",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: true,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[],
};

pub(super) const PRIMITIVES: Scenario = Scenario {
    name: "primitives",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
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
};

pub(super) const UUID_VO: Scenario = Scenario {
    name: "uuid_vo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[("Booking", &["reference:String", "tenant_id:TenantId:Uuid"])],
};

pub(super) const OPT_VEC_PRIM: Scenario = Scenario {
    name: "opt_vec_prim",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
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
};

pub(super) const PLAIN_VO: Scenario = Scenario {
    name: "plain_vo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[(
        "Item",
        &["name:Name:String", "sku:Sku:String!", "weight:Weight:i64"],
    )],
};

pub(super) const OPT_VEC_VO: Scenario = Scenario {
    name: "opt_vec_vo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[(
        "Persona",
        &[
            "name:String",
            "middle_name:MiddleName:opt:String",
            "tags:Tag:vec:String",
        ],
    )],
};

pub(super) const ENUM_VO: Scenario = Scenario {
    name: "enum_vo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[(
        "Order",
        &[
            "reference:String",
            "status:Status:enum:Pending/Confirmed/Cancelled",
        ],
    )],
};

pub(super) const SHARED_VO: Scenario = Scenario {
    name: "shared_vo",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[("Email", "String"), ("Money", "i64")],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[("Customer", &["name:String", "email:Email", "credit:Money"])],
};

pub(super) const MULTI_ENTITY: Scenario = Scenario {
    name: "multi_entity",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[
        ("Product", &["title:String", "price:i64"]),
        ("Category", &["name:Name:String"]),
    ],
};

pub(super) const VO_KITCHEN_SINK: Scenario = Scenario {
    name: "vo_kitchen_sink",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[("Email", "String")],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
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
};

pub(super) const ENUM_ONLY: Scenario = Scenario {
    name: "enum_only",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[("Ticket", &["state:State:enum:Open/Closed"])],
};

pub(super) const CUSTOM_USE_CASE: Scenario = Scenario {
    name: "custom_use_case",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[("Product", "archive_product")],
    entity_events: &[],
    consumers: &[],
    entities: &[("Product", &["title:String", "price:i64"])],
};

/// Phase 9.2: an entity whose every field reaches the frontend — each primitive an input holds,
/// and a value object over one — with a two-word name (`order-item` files, `/order_items`).
pub(super) const WEB_CRUD: Scenario = Scenario {
    name: "web_crud",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[(
        "OrderItem",
        &[
            "title:String",
            "quantity:i64",
            "rate:f64",
            "in_stock:bool",
            "ref_id:Uuid",
            "sku:Sku:String",
        ],
    )],
};

/// Phase 9.3: every field type on the frontend — `Option` of each primitive, `Vec`, `HashMap`,
/// `DateTime`, an enum VO, `Option` / `Vec` VOs — on an entity named after a TypeScript keyword
/// (`Case` → `caseItem`).
pub(super) const WEB_EVERY_TYPE: Scenario = Scenario {
    name: "web_every_type",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[(
        "Case",
        &[
            "title:String",
            "summary:opt:String",
            "views:opt:i64",
            "ratio:opt:f64",
            "urgent:opt:bool",
            "due_at:opt:DateTime",
            "opened_at:DateTime",
            "tags:vec:String",
            "scores:vec:i64",
            "meta:map",
            "status:Status:enum:Open/Closed",
            "middle_name:MiddleName:opt:String",
            "labels:Label:vec:String",
        ],
    )],
};

pub(super) const POSTGRES_EVERY_TYPE: Scenario = Scenario {
    name: "postgres_every_type",
    db: true,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[("Email", "String")],
    mails: &[],
    use_cases: &[("Product", "archive_product")],
    entity_events: &[],
    consumers: &[],
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
};

pub(super) const EVENTS_DB: Scenario = Scenario {
    name: "events_db",
    db: true,
    events: Some("db"),
    auth: None,
    access: &[],
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    use_cases: &[("Product", "publish_product")],
    entity_events: &[("Product", "product_published")],
    consumers: &[(
        "notify_new_product",
        &["product_created", "product_published"],
    )],
    entities: &[(
        "Product",
        &[
            "name:Name:String",
            "sku:Sku:String!",
            "price:i64",
            "status:Status:enum:Draft/Published",
            "tags:vec:String",
            "released_at:opt:DateTime",
            "meta:map",
        ],
    )],
};

/// [`EVENTS_DB`] in an auth project (Phase 16.4): the unit-of-work writes take the caller too,
/// and the worker builds without one.
pub(super) const AUTH_OIDC_EVENTS: Scenario = Scenario {
    name: "auth_oidc_events",
    auth: Some("oidc"),
    access: &[("Product", &["list_products"], &["delete_product=admin"])],
    ..EVENTS_DB
};

/// `events_db` on the `kafka` transport (Phase 13.5): the worker's registry wires `puerto-kafka`,
/// the outbox migration has no `event_deliveries`, `docker-compose.yml` runs Redpanda.
pub(super) const EVENTS_KAFKA: Scenario = Scenario {
    name: "events_kafka",
    events: Some("kafka"),
    ..EVENTS_DB
};

/// `events_db` with mails the worker sends (Phase 14.4): one on two events of `Product`, its fields
/// taken from the snapshot (VO, enum, `Option<DateTime>`; `Vec` and map left out), and one with
/// fields named after `--`. Plus a mail nothing sends, which the registry leaves out.
pub(super) const EVENTS_MAIL: Scenario = Scenario {
    name: "events_mail",
    mails: &[
        (
            "product_live",
            &[],
            &["product_created", "product_published"],
            Some("sku"),
        ),
        (
            "price_alert",
            &["name:String", "price:i64"],
            &["product_published"],
            Some("name"),
        ),
        ("digest", &[], &[], None),
    ],
    ..EVENTS_DB
};

/// [`EVENTS_MAIL`] on the `kafka` transport: each mail is a consumer group of its own.
pub(super) const EVENTS_MAIL_KAFKA: Scenario = Scenario {
    name: "events_mail_kafka",
    events: Some("kafka"),
    ..EVENTS_MAIL
};

/// `puerto new --db --events db` and nothing else: the worker has to build before the first
/// scaffold (it named `infrastructure::db`, which only a scaffold declared). No `query!`, so no
/// database is needed.
pub(super) const EVENTS_FRESH: Scenario = Scenario {
    name: "events_fresh",
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[],
    ..EVENTS_DB
};

/// Two mails in a project without a database (Phase 14.3): every field type a mail can carry, the
/// `mail` blocks of both `lib.rs` opened by the first and extended by the second, `puerto-mail`
/// and its fakes, and each mail's render tests.
pub(super) const MAIL: Scenario = Scenario {
    name: "mail",
    db: false,
    events: None,
    auth: None,
    access: &[],
    no_demo: false,
    no_web: true,
    shared_vos: &[],
    mails: &[
        (
            "welcome",
            &["name:String", "plan:opt:String", "seats:i64"],
            &[],
            None,
        ),
        (
            "receipt",
            &[
                "order_id:Uuid",
                "paid_at:DateTime",
                "total:f64",
                "gift:bool",
                "note:opt:String",
                "points:opt:i64",
                "express:opt:bool",
                "discount:opt:f64",
                "shipped_at:opt:DateTime",
            ],
            &[],
            None,
        ),
    ],
    use_cases: &[],
    entity_events: &[],
    consumers: &[],
    entities: &[],
};

/// `puerto new --auth oidc` (Phase 16.4): the `User` entity in memory, the provisioning
/// authenticator and its tests, `GET /me` in the spec the frontend is typed from, Keycloak in a
/// `docker-compose.yml` of its own. The Greeting demo is `public`. Two scaffolds: protected routes
/// (`BearerAuth`, `actor`, 401/403/503) and the route tests that prove them.
pub(super) const AUTH_OIDC: Scenario = Scenario {
    name: "auth_oidc",
    db: false,
    events: None,
    auth: Some("oidc"),
    no_demo: false,
    no_web: false,
    shared_vos: &[],
    mails: &[],
    // Protected by default: its params carry the caller.
    use_cases: &[("Product", "publish_product")],
    entity_events: &[],
    consumers: &[],
    entities: &[
        (
            "Product",
            &[
                "name:Name:String",
                "sku:String!",
                "price:i64",
                "status:Status:enum:Draft/Live",
            ],
        ),
        ("Order", &["total:i64"]),
    ],
    // Product: public reads, two roles on one route, a protected route with no role. Order: every
    // route protected, no role.
    access: &[(
        "Product",
        &["list_products", "get_product"],
        &[
            "create_product=admin",
            "delete_product=admin",
            "delete_product=owner",
        ],
    )],
};

/// [`AUTH_OIDC`] with a database: `PgUserRepository` over runtime-checked queries (no query cache
/// needed) and the `users` migration; its `#[sqlx::test]`s run under `make test/db`.
pub(super) const AUTH_OIDC_DB: Scenario = Scenario {
    name: "auth_oidc_db",
    db: true,
    no_web: true,
    ..AUTH_OIDC
};

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
pub(super) fn apply_scenario(scenario: &Scenario, output: &Path) {
    if scenario.db {
        crate::generators::project::apply_db_to_new_project(output).unwrap();
    }
    if let Some(transport) = scenario.events {
        scaffold::apply_events_to_new_project(output, transport).unwrap();
    }
    if let Some(mode) = scenario.auth {
        crate::generators::auth::apply_auth_to_new_project(output, mode).unwrap();
    }
    if scenario.no_demo {
        scaffold::apply_no_demo(output).unwrap();
    }
    if scenario.no_web {
        scaffold::apply_no_web(output).unwrap();
    }
    // `puerto new`, and every later command that writes Rust, ends with `cargo fmt`: the next
    // step's patchers read formatted code, as they do for a user.
    let format = || crate::generators::format::format_project(output).unwrap();
    format();

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
        let access = scenario
            .access
            .iter()
            .find(|(name, _, _)| name == entity)
            .map(|(_, public, roles)| {
                let strings =
                    |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
                scaffold::ScaffoldAccess::from_args(&strings(public), &strings(roles)).unwrap()
            })
            .unwrap_or_default();
        scaffold::run_scaffold_with_access(entity, output, &fields, OverwritePolicy::Ask, &access)
            .unwrap();
        format();
    }

    for (entity, action) in scenario.use_cases {
        scaffold::run_use_case(entity, action, output).unwrap();
        format();
    }

    for (entity, event) in scenario.entity_events {
        scaffold::run_generate_event(entity, event, output).unwrap();
        format();
    }

    for (name, events) in scenario.consumers {
        let events: Vec<String> = events.iter().map(|e| e.to_string()).collect();
        scaffold::run_generate_consumer(name, &events, output).unwrap();
        format();
    }

    for (name, field_args, on, to) in scenario.mails {
        let field_args: Vec<String> = field_args.iter().map(|f| f.to_string()).collect();
        let on: Vec<String> = on.iter().map(|e| e.to_string()).collect();
        scaffold::run_generate_mail_on(name, &field_args, &on, *to, output).unwrap();
        format();
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
    patch_runtime_crates(output);
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
        // The project's own `make format`: generated code is formatted as it is written.
        &["fmt", "--all", "--", "--check"][..],
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

    if output.join("web").is_dir() {
        assert_spec_export_is_stable(output, dir, name, envs);
        assert_web_is_clean(output, name);
    }
}

/// Builds the project against the runtime crates in this checkout, not the ones on crates.io: the
/// templates and an unreleased runtime change are tested together (ADR-0017). Done by the harness
/// only — `puerto new` never writes it, so it is not in the snapshots either.
pub(super) fn patch_runtime_crates(output: &Path) {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let manifest = output.join("Cargo.toml");
    let mut src = fs::read_to_string(&manifest).unwrap();
    let mut patch = String::from("\n[patch.crates-io]\n");
    // `puerto-core` lives in `crates/core`, `puerto-web` in `crates/web`, … Only the crates the
    // project names: an unused patch is a cargo warning in every build.
    for name in crate::runtime::RUNTIME_CRATES {
        if !src.contains(&format!("{name} = ")) {
            continue;
        }
        let dir = name.trim_start_matches("puerto-");
        let path = crates.join(dir).canonicalize().unwrap();
        patch.push_str(&format!("{name} = {{ path = {path:?} }}\n"));
    }
    if !src.contains("[patch.crates-io]") {
        src.push_str(&patch);
        fs::write(&manifest, src).unwrap();
    }
}

/// The first `cargo test` of a web project wrote `docs/api/openapi.json` (it was missing). A
/// second run compares against it: the strict path users hit after every scaffold, and proof the
/// export is deterministic.
fn assert_spec_export_is_stable(output: &Path, dir: &Path, name: &str, envs: &[(&str, &str)]) {
    assert!(
        output.join("docs/api/openapi.json").exists(),
        "scenario '{name}': `cargo test` did not export docs/api/openapi.json"
    );
    let result = std::process::Command::new("cargo")
        .args(["test", "-p", "presentation", "openapi"])
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .envs(envs.iter().copied())
        .current_dir(output)
        .output()
        .expect("failed to run cargo");
    if !result.status.success() {
        eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
        eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
        panic!("scenario '{name}': the exported spec differs from the one just written");
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
    assert_scenario_compiles(&NO_DEMO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_all_primitive_field_types() {
    assert_scenario_compiles(&PRIMITIVES);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_uuid_value_object() {
    assert_scenario_compiles(&UUID_VO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_option_and_vec_primitives() {
    assert_scenario_compiles(&OPT_VEC_PRIM);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_plain_and_unique_value_objects() {
    assert_scenario_compiles(&PLAIN_VO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_option_and_vec_value_objects() {
    assert_scenario_compiles(&OPT_VEC_VO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_enum_value_object() {
    assert_scenario_compiles(&ENUM_VO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_shared_value_objects() {
    assert_scenario_compiles(&SHARED_VO);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_multiple_entities() {
    assert_scenario_compiles(&MULTI_ENTITY);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_every_value_object_kind_at_once() {
    assert_scenario_compiles(&VO_KITCHEN_SINK);
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

    // Formatted after each step, like the CLI. Until the last layer exists a declared module has
    // no file and `cargo fmt` fails — the CLI only warns, so the steps here ignore it too.
    let format = || {
        let _ = crate::generators::format::format_project(&output);
    };
    format();
    scaffold::run_generate_domain("Invoice", &output, &fields).unwrap();
    format();
    scaffold::run_generate_application("Invoice", &output).unwrap();
    format();
    scaffold::run_generate_repository("Invoice", &output).unwrap();
    format();
    scaffold::run_generate_presentation("Invoice", &output).unwrap();
    format();
    // The flow's last step in a web project: `core/generated/*` already routes Invoice.
    scaffold::run_generate_web("Invoice", &output, OverwritePolicy::Ask).unwrap();

    assert_generated_project_is_clean(&output, &dir, "layered_fields", &[]);

    cleanup(&dir);
}

// ── T-13: review fixes — matrix gaps ──────────────────────────────────────

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_enum_vo_as_only_string_field() {
    // The create test asserted `.value()` on the first String-typed field — an enum VO has
    // `as_str()`, so an entity whose only String was an enum never compiled its tests.
    assert_scenario_compiles(&ENUM_ONLY);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_mails_and_renders_their_templates() {
    assert_scenario_compiles(&MAIL);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_with_custom_use_case_after_scaffold() {
    // `generate use-case` on a scaffolded entity wired the use case into `ProductApi` in
    // bootstrap.rs while routes.rs had no such field (E0560).
    assert_scenario_compiles(&CUSTOM_USE_CASE);
}

// ── Phase 9: the web frontend ─────────────────────────────────────────────

/// Runs the frontend's own checks inside a generated project: install, wire types from the
/// exported spec, typecheck, Biome, dependency-cruiser, `bun test` and a production build. Needs
/// `bun` on `PATH`, and `docs/api/openapi.json` (`assert_generated_project_is_clean` exports it,
/// then calls this for every web scenario: a few seconds each, against minutes of `cargo`).
///
/// `bun run lint` runs **before** any formatting: the generated TypeScript must come out
/// Biome-clean. (`make web/sync` formats too — the net for names long enough to wrap a line.)
fn assert_web_is_clean(output: &Path, name: &str) {
    let web = output.join("web");
    for args in [
        &["install"][..],
        &["run", "openapi:types"][..],
        &["run", "typecheck"][..],
        &["run", "lint"][..],
        &["run", "lint:arch"][..],
        &["test"][..],
        &["run", "build"][..],
    ] {
        let result = std::process::Command::new("bun")
            .args(args)
            .current_dir(&web)
            .output()
            .expect("bun not found — the web scenarios need it on PATH (https://bun.sh)");

        if !result.status.success() {
            eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
            eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
            panic!(
                "scenario '{name}': generated frontend failed `bun {}`",
                args.join(" ")
            );
        }
    }
}

#[test]
#[ignore = "slow: compiles a generated project, then installs, lints and tests its frontend"]
fn web_shell_passes_its_own_checks() {
    let dir = scenario_workspace("web_shell");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("compile-web-shell", &dir).unwrap();
    // What `puerto new` does last.
    crate::generators::format::format_project(&output).unwrap();
    assert_generated_project_is_clean(&output, &dir, "web_shell", &[]);
    assert!(
        output
            .join("web/src/infrastructure/http/schema.d.ts")
            .exists()
    );

    cleanup(&dir);
}

#[test]
#[ignore = "slow: compiles a generated project, then installs, lints and tests its frontend"]
fn web_crud_slice_passes_its_own_checks() {
    assert_scenario_compiles(&WEB_CRUD);
}

#[test]
#[ignore = "slow: compiles a generated project, then installs, lints and tests its frontend"]
fn web_every_type_slice_passes_its_own_checks() {
    assert_scenario_compiles(&WEB_EVERY_TYPE);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_without_the_web_frontend() {
    assert_scenario_compiles(&NO_WEB);
}

#[test]
#[ignore = "slow: compiles a generated project, then installs, lints and tests its frontend"]
fn compiles_an_oidc_auth_project() {
    assert_scenario_compiles(&AUTH_OIDC);
}

#[test]
#[ignore = "slow: compiles and tests a generated project"]
fn compiles_a_fresh_events_project_before_any_scaffold() {
    assert_scenario_compiles(&EVENTS_FRESH);
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
    assert_postgres_scenario_is_clean(&POSTGRES_EVERY_TYPE, "puerto_matrix");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_oidc_auth_project_compiles_lints_and_passes_integration_tests() {
    assert_postgres_scenario_is_clean(&AUTH_OIDC_DB, "puerto_matrix_auth_oidc");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_oidc_auth_events_project_compiles_lints_and_passes_integration_tests() {
    assert_postgres_scenario_is_clean(&AUTH_OIDC_EVENTS, "puerto_matrix_auth_oidc_events");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_events_project_compiles_lints_and_passes_integration_tests() {
    // Phase 13.3: units of work, the outbox migration and `events.rs` — the Pg unit of work's
    // tests write an entity and its `product_created` row, and a taken `unique` writes neither.
    assert_postgres_scenario_is_clean(&EVENTS_DB, "puerto_matrix_events");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/kafka`"]
fn kafka_events_project_compiles_lints_and_passes_integration_tests() {
    // Phase 13.5: the worker against `puerto-kafka`, and the outbox migration without
    // `event_deliveries` under the Pg unit of work's tests. No broker is needed to build and
    // test it; `make test/kafka` provides the Postgres it compiles against.
    assert_postgres_scenario_is_clean(&EVENTS_KAFKA, "puerto_matrix_events_kafka");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_events_project_with_mails_compiles_lints_and_passes_its_tests() {
    // Phase 14.4: the mail consumers, their registry wiring and the `sent_mails` migration.
    assert_postgres_scenario_is_clean(&EVENTS_MAIL, "puerto_matrix_events_mail");
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/kafka`"]
fn kafka_events_project_with_mails_compiles_lints_and_passes_its_tests() {
    assert_postgres_scenario_is_clean(&EVENTS_MAIL_KAFKA, "puerto_matrix_events_mail_kafka");
}

/// A `--db` scenario against a real database: `query!` checks its SQL at compile time, so the
/// schema has to exist before the project builds.
fn assert_postgres_scenario_is_clean(scenario: &Scenario, database: &str) {
    assert_edited_postgres_scenario_is_clean(scenario, database, |_| {});
}

/// [`assert_postgres_scenario_is_clean`], with `edit` applied to the project before it builds —
/// what a user writes by hand after the generators.
pub(super) fn assert_edited_postgres_scenario_is_clean(
    scenario: &Scenario,
    database: &str,
    edit: impl FnOnce(&Path),
) {
    let Some(server) = test_database_server() else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let dir = scenario_workspace(scenario.name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project(
        &format!("compile-{}", scenario.name.replace('_', "-")),
        &dir,
    )
    .unwrap();
    apply_scenario(scenario, &output);
    edit(&output);

    let database_url = format!("{server}/{database}");
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
        scenario.name,
        &[("DATABASE_URL", &database_url), ("SQLX_OFFLINE", "false")],
    );
    assert_startup_reports_a_database_that_is_down(&output, &dir, &database_url);

    cleanup(&dir);
}

/// Phase 15.2: the server (and the worker) started against a database that is down end with a
/// message and a hint — exit code 1, no panic, no password.
fn assert_startup_reports_a_database_that_is_down(output: &Path, dir: &Path, database_url: &str) {
    let binaries: Vec<&str> = ["presentation", "worker"]
        .into_iter()
        .filter(|b| output.join(b).is_dir())
        .collect();
    for binary in &binaries {
        let built = std::process::Command::new("cargo")
            .args(["build", "-p", binary])
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .env("DATABASE_URL", database_url)
            .env("SQLX_OFFLINE", "false")
            .current_dir(output)
            .status()
            .expect("failed to run cargo");
        assert!(built.success(), "cargo build -p {binary}");

        // The executable is the `[[bin]]` name when the member declares one (presentation's is
        // the project's name), else the package's.
        let manifest = fs::read_to_string(output.join(binary).join("Cargo.toml")).unwrap();
        let executable = manifest
            .split_once("[[bin]]")
            .and_then(|(_, bin)| bin.lines().find_map(|l| l.strip_prefix("name = ")))
            .map_or(*binary, |name| name.trim_matches('"'));

        // Nothing listens on port 1.
        let run = std::process::Command::new(dir.join("target/debug").join(executable))
            .env("DATABASE_URL", "postgres://devuser:s3cret@127.0.0.1:1/shop")
            .env("PUERTO_BANNER", "false")
            .current_dir(output)
            .output()
            .expect("failed to start the binary");
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(1), "{binary}: {stderr}");
        assert!(stderr.contains("127.0.0.1:1/shop"), "{binary}: {stderr}");
        assert!(
            stderr.contains("is the database up? `make docker-compose/up`"),
            "{binary}: {stderr}"
        );
        assert!(!stderr.contains("panicked"), "{binary}: {stderr}");
        assert!(!stderr.contains("s3cret"), "{binary}: {stderr}");
    }
}
