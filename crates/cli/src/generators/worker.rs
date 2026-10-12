//! The worker of an `events` project (ADR-0018 § 5): the `worker/` crate `puerto new --events db`
//! writes, `worker/src/generated/registry.rs` (always regenerated from puerto.toml) and
//! `puerto generate consumer`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::generators::bootstrap::uses_unit_of_work;
use crate::generators::naming::{
    crud_events, event_variant, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::render;
use crate::puerto_toml::{Consumer, PuertoConfig};

/// The transport whose worker this version generates; Kafka's arrives with `puerto-kafka`.
pub const WORKER_TRANSPORT: &str = "db";

/// Names the worker's generated code already uses: a consumer named after one would not compile.
const RESERVED_CONSUMER_TYPES: &[&str] = &[
    "Arc",
    "EventEnvelope",
    "EventHandler",
    "HandlerOutcome",
    "Loops",
    "PgDeadLetters",
    "PgDeliveries",
    "PgOutbox",
    "PgSubscription",
    "WorkerConfig",
];

/// An event type some entity declares, and where its constant lives.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeclaredEvent {
    pub event_type: String,
    /// `PRODUCT_CREATED`, exported by the entity's `events.rs`.
    pub constant: String,
    /// `product_created`: a function name in the consumer's tests.
    pub const_lower: String,
    /// The `{Entity}Event` variant: `Created`.
    pub variant: String,
    pub entity_pascal: String,
    pub entity_snake: String,
}

/// Every event type the project's entities record: the CRUD ones of each entity that writes
/// through a unit of work, then its custom ones.
pub fn declared_events(config: &PuertoConfig) -> Vec<DeclaredEvent> {
    config
        .entity
        .iter()
        .filter(|e| uses_unit_of_work(e, config.project.has_events()))
        .flat_map(|entity| {
            let snake = pascal_to_snake(&entity.name);
            crud_events(&snake)
                .into_iter()
                .chain(
                    entity
                        .events
                        .iter()
                        .map(|event| (event_variant(&snake, event), event.clone())),
                )
                .map(|(variant, event_type)| DeclaredEvent {
                    constant: event_type.to_uppercase(),
                    const_lower: event_type.clone(),
                    event_type,
                    variant,
                    entity_pascal: entity.name.clone(),
                    entity_snake: snake.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether the project at `config` has a `worker/` (an `events` project on the `db` transport).
pub fn has_worker(config: &PuertoConfig) -> bool {
    config.project.events.as_deref() == Some(WORKER_TRANSPORT)
}

/// The declared events `consumer` subscribes to, in its order. Errors on one nobody declares.
fn subscribed(
    consumer: &Consumer,
    declared: &[DeclaredEvent],
) -> Result<Vec<DeclaredEvent>, String> {
    consumer
        .events
        .iter()
        .map(|event| {
            declared
                .iter()
                .find(|d| d.event_type == *event)
                .cloned()
                .ok_or_else(|| undeclared_event(event, declared))
        })
        .collect()
}

fn undeclared_event(event: &str, declared: &[DeclaredEvent]) -> String {
    let known: Vec<&str> = declared.iter().map(|d| d.event_type.as_str()).collect();
    if known.is_empty() {
        format!("event '{event}' is not declared: no CRUD entity records events yet")
    } else {
        format!(
            "event '{event}' is not declared by any entity. Declared: {}",
            known.join(", ")
        )
    }
}

#[derive(Serialize)]
struct RegistryConsumer {
    name: String,
    pascal: String,
    /// The constants it is routed: `PRODUCT_CREATED`.
    events: Vec<String>,
}

/// `worker/src/generated/registry.rs`: the relay and every `[[consumer]]`, on the `db` transport.
pub fn generate_registry(config: &PuertoConfig) -> Result<String, Box<dyn std::error::Error>> {
    let declared = declared_events(config);
    // entity snake → the constants its consumers name, in declaration order.
    let mut constants: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut consumers = vec![];
    for consumer in &config.consumer {
        let events = subscribed(consumer, &declared)
            .map_err(|e| format!("consumer '{}': {e}", consumer.name))?;
        for event in &events {
            let names = constants.entry(event.entity_snake.clone()).or_default();
            if !names.contains(&event.constant) {
                names.push(event.constant.clone());
            }
        }
        consumers.push(RegistryConsumer {
            name: consumer.name.clone(),
            pascal: to_pascal_case(&consumer.name),
            events: events.into_iter().map(|e| e.constant).collect(),
        });
    }

    let mut imports: Vec<String> = constants
        .into_iter()
        .map(|(snake, mut names)| {
            names.sort();
            format!("business::domain::{snake}::events::{}", braced(&names))
        })
        .collect();
    let adapters: Vec<String> = if consumers.is_empty() {
        vec!["PgDeliveries".into(), "PgOutbox".into()]
    } else {
        [
            "PgDeadLetters",
            "PgDeliveries",
            "PgOutbox",
            "PgSubscription",
        ]
        .map(String::from)
        .to_vec()
    };
    imports.push(format!("infrastructure::db::events::{}", braced(&adapters)));
    imports.push("puerto_events::{Loops, WorkerConfig}".into());

    let mut modules: Vec<(&str, &str)> = consumers
        .iter()
        .map(|c| (c.name.as_str(), c.pascal.as_str()))
        .collect();
    modules.sort_unstable();

    let mut ctx = tera::Context::new();
    ctx.insert("imports", &imports);
    ctx.insert("consumers", &consumers);
    ctx.insert("modules", &modules);
    Ok(render("worker/registry.tera", &ctx)?)
}

/// `A` or `{A, B}`, as rustfmt writes a `use`.
fn braced(names: &[String]) -> String {
    match names {
        [single] => single.clone(),
        _ => format!("{{{}}}", names.join(", ")),
    }
}

/// `worker/src/consumers/<name>.rs`: an `EventHandler` that matches each subscribed event,
/// decodes its snapshot (dead-lettering what does not decode), logs and acks — with its tests.
pub fn generate_consumer(
    name: &str,
    events: &[DeclaredEvent],
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(first) = events.first() else {
        return Err(format!("consumer '{name}' subscribes to no event").into());
    };
    let mut by_entity: BTreeMap<&str, (Vec<String>, &str)> = BTreeMap::new();
    for event in events {
        let (names, pascal) = by_entity
            .entry(&event.entity_snake)
            .or_insert((vec![], &event.entity_pascal));
        names.push(event.constant.clone());
        if !names.contains(&format!("{pascal}Snapshot")) {
            names.push(format!("{pascal}Snapshot"));
        }
    }
    let imports: Vec<String> = by_entity
        .into_iter()
        .map(|(snake, (mut names, _))| {
            names.sort();
            format!("business::domain::{snake}::events::{}", braced(&names))
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("name", name);
    ctx.insert("pascal", &to_pascal_case(name));
    ctx.insert("events", events);
    ctx.insert("first", first);
    ctx.insert("imports", &imports);
    Ok(render("worker/consumer.tera", &ctx)?)
}

/// Rewrites `registry.rs` of a project that has a worker — after `generate bootstrap`, a scaffold
/// and `generate consumer`, like `bootstrap.rs`.
pub fn regenerate_registry(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join("worker/src/generated");
    if !has_worker(config) || !dir.is_dir() {
        return Ok(());
    }
    write_file(&dir.join("registry.rs"), &generate_registry(config)?)?;
    Ok(())
}

/// `consumers.rs` with `pub mod {name};` among its module declarations, sorted as rustfmt keeps
/// them; every other line stays where it was, ahead of them.
fn with_module(src: &str, name: &str) -> String {
    let declaration = format!("pub mod {name};");
    let (mut modules, other): (Vec<&str>, Vec<&str>) = src
        .lines()
        .partition(|line| line.starts_with("pub mod ") && line.ends_with(';'));
    if !modules.contains(&declaration.as_str()) {
        modules.push(&declaration);
    }
    modules.sort_unstable();
    other
        .into_iter()
        .chain(modules)
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The Makefile targets a project with a worker gets.
fn makefile_targets(db_name: &str) -> String {
    format!(
        r#"
.PHONY: worker events/dead-letters

worker: ## Run the worker: relays the outbox and runs the consumers (health on :8081)
	@echo "${{GREEN}}Starting worker...${{NC}}"
	$(CARGO) run -p worker

events/dead-letters: docker-compose/up ## List the 20 most recent dead letters
	$(DOCKER_COMPOSE) exec -T postgres psql -U devuser -d {db_name} -c \
		"SELECT recorded_at, consumer, event_type, attempts, reason, detail FROM dead_letters ORDER BY recorded_at DESC LIMIT 20"
"#
    )
}

/// The `worker/` crate of a fresh `events` project on the `db` transport: the member, its runtime
/// dependency, the files, `make worker` / `make events/dead-letters` and `WORKER_PORT`.
pub fn apply_worker_to_new_project(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = base.join("Cargo.toml");
    let src = fs::read_to_string(&manifest)?;
    if !src.contains("\"worker\"") {
        let members = "members = [\"business\", \"infrastructure\", \"presentation\"]";
        if !src.contains(members) {
            return Err(format!("{}: unexpected workspace members", manifest.display()).into());
        }
        fs::write(
            &manifest,
            src.replace(
                members,
                "members = [\"business\", \"infrastructure\", \"presentation\", \"worker\"]",
            ),
        )?;
    }
    crate::generators::project::add_workspace_dependency(base, "puerto-events")?;

    let empty = tera::Context::new();
    for (path, template) in [
        ("worker/Cargo.toml", "worker/cargo_toml.tera"),
        ("worker/AGENTS.md", "worker/agents.tera"),
        ("worker/src/main.rs", "worker/main.tera"),
        ("worker/src/generated.rs", "worker/generated.tera"),
        ("worker/src/consumers.rs", "worker/consumers.tera"),
    ] {
        write_file(&base.join(path), &render(template, &empty)?)?;
    }
    let config = crate::puerto_toml::read(base)?;
    write_file(
        &base.join("worker/src/generated/registry.rs"),
        &generate_registry(&config)?,
    )?;

    let makefile = base.join("Makefile");
    if makefile.exists() {
        let mut src = fs::read_to_string(&makefile)?;
        if !src.contains("events/dead-letters") {
            let db_name = config.project.name.replace('-', "_");
            src.push_str(&makefile_targets(&db_name));
            fs::write(&makefile, src)?;
        }
    }

    for file in [".env", ".env.example"] {
        let path = base.join(file);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if !content.contains("WORKER_PORT") {
            content.push_str("\n# Worker\nWORKER_PORT=8081\n");
            fs::write(&path, content)?;
        }
    }
    Ok(())
}

/// `puerto generate consumer <name> --on <event_type>[,…]`: declares the consumer, writes its
/// handler, declares its module and regenerates the registry. Refuses an existing name, an event
/// no entity declares, and a file already on disk (it is never overwritten).
pub fn run_generate_consumer(
    name: &str,
    events: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    if !has_worker(&config) {
        return Err(
            "this project has no worker — consumers need a project created with \
             `puerto new --db --events db`"
                .into(),
        );
    }
    crate::validation::validate_consumer_name(name)?;
    let pascal = to_pascal_case(name);
    let taken = RESERVED_CONSUMER_TYPES.contains(&pascal.as_str())
        || config.entity.iter().any(|e| {
            [
                format!("{}Event", e.name),
                format!("{}Snapshot", e.name),
                format!("{}Mother", e.name),
            ]
            .contains(&pascal)
        });
    if taken {
        return Err(format!(
            "consumer name '{name}' would be the type `{pascal}`, which the worker already uses"
        )
        .into());
    }
    if config.consumer.iter().any(|c| c.name == name) {
        return Err(format!("consumer '{name}' is already declared").into());
    }
    if events.is_empty() {
        return Err("a consumer needs at least one event: --on <event_type>".into());
    }
    let mut unique: Vec<String> = vec![];
    for event in events {
        if !unique.contains(event) {
            unique.push(event.clone());
        }
    }
    let consumer = Consumer {
        name: name.to_string(),
        events: unique,
    };
    let subscribed = subscribed(&consumer, &declared_events(&config))?;

    let path = base.join(format!("worker/src/consumers/{name}.rs"));
    if path.exists() {
        return Err(format!(
            "{} already exists — it is never overwritten. Remove it or pick another name",
            path.display()
        )
        .into());
    }
    let content = generate_consumer(name, &subscribed)?;

    crate::puerto_toml::add_consumer(base, consumer)?;
    write_file(&path, &content)?;
    let modules = base.join("worker/src/consumers.rs");
    fs::write(&modules, with_module(&fs::read_to_string(&modules)?, name))?;
    regenerate_registry(base, &crate::puerto_toml::read(base)?)?;

    println!(
        "✓ Consumer '{name}' added, on: {}",
        subscribed
            .iter()
            .map(|e| e.event_type.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  worker/src/consumers/{name}.rs — yours: make it idempotent, it can see an event twice."
    );
    println!("  worker/src/generated/registry.rs regenerated. Run it with `make worker`.");
    Ok(())
}
