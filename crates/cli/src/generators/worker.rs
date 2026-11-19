//! The worker of an `events` project (ADR-0018 § 5): the `presentacion/worker` crate
//! `puerto nuevo --eventos` writes, its `generado/registro.rs` (always regenerated from puerto.toml,
//! on the project's transport) and `puerto generar consumidor`.

use crate::generators::layout::{self, Layer};
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

/// The transport that needs a broker: its registry wires `puerto-kafka`, its outbox migration has
/// no `entregas_eventos` and its `docker-compose.yml` runs Redpanda.
pub const KAFKA_TRANSPORT: &str = "kafka";

/// Names the worker's generated code already uses: a consumer named after one would not compile.
const RESERVED_CONSUMER_TYPES: &[&str] = &[
    "Arc",
    "SobreEvento",
    "ManejadorEvento",
    "ResultadoManejo",
    "ConfigKafka",
    "PublicadorKafka",
    "SuscripcionKafka",
    "Bucles",
    "DescartadosPostgres",
    "EntregasPostgres",
    "OutboxPostgres",
    "SuscripcionPostgres",
    "ConfigWorker",
];

/// A use case a consumer holds (`generar consumidor --usa`, 27.14), and where `cableado` builds it.
#[derive(Debug, Clone, Serialize)]
pub struct UsedUseCase {
    pub uc: String,
    pub uc_pascal: String,
    pub entity_snake: String,
    /// `cableado::<entity>::construir` takes the pool: the entity is in Postgres.
    pub db: bool,
    /// Its params carry an `actor`: a protected use case of an auth project.
    pub actor: bool,
    /// A CRUD list: `todas_las_paginas` reads it whole.
    pub list: bool,
}

/// The use cases `names` name, each with the entity that declares it; an unknown one is an error.
fn used_use_cases(config: &PuertoConfig, names: &[String]) -> Result<Vec<UsedUseCase>, String> {
    let mut used = vec![];
    for name in names {
        let Some(entity) = config.entity.iter().find(|e| e.use_cases.contains(name)) else {
            return Err(format!(
                "--usa: ningún [[entidad]] de puerto.toml declara el caso de uso '{name}'"
            ));
        };
        let snake = pascal_to_snake(&entity.name);
        if used.iter().any(|u: &UsedUseCase| &u.uc == name) {
            continue;
        }
        used.push(UsedUseCase {
            uc: name.clone(),
            uc_pascal: to_pascal_case(name),
            db: entity.db,
            actor: config.project.has_auth() && !entity.internal.contains(name),
            list: *name == crate::generators::naming::crud_use_case("list", &snake),
            entity_snake: snake,
        });
    }
    Ok(used)
}

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

/// Whether the project at `config` has a `worker/`: every `events` project does.
pub fn has_worker(config: &PuertoConfig) -> bool {
    config.project.events.is_some()
}

fn on_kafka(config: &PuertoConfig) -> bool {
    config.project.events.as_deref() == Some(KAFKA_TRANSPORT)
}

/// The declared events `consumer` subscribes to, in its order. Errors on one nobody declares.
pub(crate) fn subscribed(
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
        format!(
            "ninguna entidad declara el evento '{event}': aún no hay entidades CRUD que registren \
             eventos"
        )
    } else {
        format!(
            "ninguna entidad declara el evento '{event}'. Los declarados: {}",
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
    /// The aggregates whose topics it reads (Kafka): `product`, sorted.
    aggregates: Vec<String>,
    /// The handler, as the registry builds it: `{Consumer}::construir(&pool)` for a `[[consumidor]]`, the
    /// `Send{Mail}Mail { … }` of a mail sent on events.
    handler: String,
}

/// `presentacion/worker/src/generado/registro.rs`: the relay and every `[[consumidor]]`, on the project's
/// transport.
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
        let mut aggregates: Vec<String> = events.iter().map(|e| e.entity_snake.clone()).collect();
        aggregates.sort();
        aggregates.dedup();
        let pascal = to_pascal_case(&consumer.name);
        consumers.push(RegistryConsumer {
            name: consumer.name.clone(),
            handler: format!("{pascal}::construir(&pool)"),
            pascal,
            events: events.into_iter().map(|e| e.constant).collect(),
            aggregates,
        });
    }
    // Each mail sent on events is one more consumer, `correo_<name>` (ADR-0020 § 3).
    let mut template_mailers: Vec<String> = vec![];
    for mail in config.mail.iter().filter(|m| !m.on.is_empty()) {
        let consumer = Consumer {
            name: format!("correo_{}", mail.name),
            events: mail.on.clone(),
        };
        let events =
            subscribed(&consumer, &declared).map_err(|e| format!("mail '{}': {e}", mail.name))?;
        for event in &events {
            let names = constants.entry(event.entity_snake.clone()).or_default();
            if !names.contains(&event.constant) {
                names.push(event.constant.clone());
            }
        }
        let mut aggregates: Vec<String> = events.iter().map(|e| e.entity_snake.clone()).collect();
        aggregates.sort();
        aggregates.dedup();
        let mail_pascal = to_pascal_case(&mail.name);
        template_mailers.push(format!(
            "infra_correo::{}::EnviadorCorreo{mail_pascal}Plantilla",
            mail.name
        ));
        let pascal = format!("EnviarCorreo{mail_pascal}");
        consumers.push(RegistryConsumer {
            handler: format!(
                "{pascal} {{ enviador: Arc::new(EnviadorCorreo{mail_pascal}Plantilla::new(enviador.clone())), \
                 correos_enviados: correos_enviados.clone() }}"
            ),
            name: consumer.name,
            pascal,
            events: events.into_iter().map(|e| e.constant).collect(),
            aggregates,
        });
    }
    let mails = !template_mailers.is_empty();

    let mut imports: Vec<String> = constants
        .into_iter()
        .map(|(snake, mut names)| {
            names.sort();
            format!("dominio_{snake}::eventos::{}", braced(&names))
        })
        .collect();
    let kafka = on_kafka(config);
    let adapters: &[&str] = match (kafka, consumers.is_empty()) {
        (false, true) => &["EntregasPostgres", "OutboxPostgres"],
        (false, false) => &[
            "DescartadosPostgres",
            "EntregasPostgres",
            "OutboxPostgres",
            "SuscripcionPostgres",
        ],
        (true, true) => &["OutboxPostgres"],
        (true, false) => &["DescartadosPostgres", "OutboxPostgres"],
    };
    let adapters: Vec<String> = adapters.iter().map(|a| a.to_string()).collect();
    imports.push(format!("puerto_sqlx::eventos::{}", braced(&adapters)));
    if mails {
        imports.push("puerto_sqlx::correo::CorreosEnviadosPostgres".into());
        imports.extend(template_mailers);
    }
    imports.push("puerto_eventos::{Bucles, ConfigWorker}".into());
    if kafka {
        let types = if consumers.is_empty() {
            "{ConfigKafka, PublicadorKafka}"
        } else {
            "{ConfigKafka, PublicadorKafka, SuscripcionKafka}"
        };
        imports.push(format!("puerto_kafka::{types}"));
    }
    if mails {
        imports.push("puerto_correo::{ConfigCorreo, EnviadorCorreoSmtp}".into());
    }

    let mut modules: Vec<(&str, &str)> = consumers
        .iter()
        .map(|c| (c.name.as_str(), c.pascal.as_str()))
        .collect();
    modules.sort_unstable();

    let mut ctx = tera::Context::new();
    ctx.insert("imports", &imports);
    ctx.insert("consumers", &consumers);
    ctx.insert("modules", &modules);
    ctx.insert("mails", &mails);
    if !kafka {
        return Ok(render("worker/registry.tera", &ctx)?);
    }
    // Every aggregate that records events gets its topic at startup, subscribed to or not: the
    // relay publishes them all.
    let mut aggregates: Vec<String> = declared.into_iter().map(|e| e.entity_snake).collect();
    aggregates.sort();
    aggregates.dedup();
    ctx.insert("aggregates", &aggregates);
    ctx.insert("project", &config.project.name);
    Ok(render("worker/registry_kafka.tera", &ctx)?)
}

/// `A` or `{A, B}`, as rustfmt writes a `use`.
fn braced(names: &[String]) -> String {
    match names {
        [single] => single.clone(),
        _ => format!("{{{}}}", names.join(", ")),
    }
}

/// `presentacion/worker/src/consumidores/<name>.rs`: a `ManejadorEvento` that matches each subscribed event,
/// decodes its snapshot (dead-lettering what does not decode), logs and acks — with its tests.
pub fn generate_consumer(
    name: &str,
    events: &[DeclaredEvent],
    uses: &[UsedUseCase],
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(first) = events.first() else {
        return Err(format!("el consumidor '{name}' no se suscribe a ningún evento").into());
    };
    let mut by_entity: BTreeMap<&str, (Vec<String>, &str)> = BTreeMap::new();
    for event in events {
        let (names, pascal) = by_entity
            .entry(&event.entity_snake)
            .or_insert((vec![], &event.entity_pascal));
        names.push(event.constant.clone());
        if !names.contains(&format!("Snapshot{pascal}")) {
            names.push(format!("Snapshot{pascal}"));
        }
    }
    let imports: Vec<String> = by_entity
        .into_iter()
        .map(|(snake, (mut names, _))| {
            names.sort();
            format!("dominio_{snake}::eventos::{}", braced(&names))
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("name", name);
    ctx.insert("pascal", &to_pascal_case(name));
    ctx.insert("events", events);
    ctx.insert("first", first);
    ctx.insert("imports", &imports);
    ctx.insert("uses", uses);
    // `cableado::<c>::construir` once per context, in the order the use cases name them.
    let mut contexts: Vec<(&str, bool)> = vec![];
    for u in uses {
        if !contexts.iter().any(|(c, _)| *c == u.entity_snake) {
            contexts.push((&u.entity_snake, u.db));
        }
    }
    ctx.insert("contexts", &contexts);
    ctx.insert("actor", &uses.iter().any(|u| u.actor));
    ctx.insert("list", &uses.iter().find(|u| u.list));
    Ok(render("worker/consumer.tera", &ctx)?)
}

/// Rewrites `registry.rs` of a project that has a worker — after `generar bootstrap`, a scaffold
/// and `generar consumidor`, like `bootstrap.rs`.
pub fn regenerate_registry(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join(layout::WORKER_GENERATED_DIR);
    if !has_worker(config) || !dir.is_dir() {
        return Ok(());
    }
    write_file(&base.join(layout::REGISTRY_RS), &generate_registry(config)?)?;
    add_worker_dependencies(base, config)
}

/// The project crates the worker's code names: the domain of every context whose events a
/// consumer decodes (its Object Mother too, in the consumers' tests), and the `correo` context
/// once a mail is sent on events. Added to `presentacion/worker/Cargo.toml`, never removed.
fn add_worker_dependencies(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::generators::bootstrap::add_crate_dependency;
    let declared = declared_events(config);
    let mut contexts: Vec<String> = vec![];
    let subscriptions =
        config
            .consumer
            .iter()
            .cloned()
            .chain(
                config
                    .mail
                    .iter()
                    .filter(|m| !m.on.is_empty())
                    .map(|m| Consumer {
                        name: format!("correo_{}", m.name),
                        events: m.on.clone(),
                    }),
            );
    for consumer in subscriptions {
        for event in subscribed(&consumer, &declared)? {
            if !contexts.contains(&event.entity_snake) {
                contexts.push(event.entity_snake);
            }
        }
    }
    contexts.sort();
    for c in &contexts {
        let name = layout::crate_name(c, Layer::Domain);
        add_crate_dependency(
            base,
            layout::WORKER_MANIFEST,
            "dependencies",
            &name,
            "{ workspace = true }",
        )?;
        add_crate_dependency(
            base,
            layout::WORKER_MANIFEST,
            "dev-dependencies",
            &name,
            "{ workspace = true, features = [\"pruebas\"] }",
        )?;
    }
    if config.mail.iter().any(|m| !m.on.is_empty()) {
        let correo = layout::MAIL_CONTEXT;
        for layer in [Layer::Domain, Layer::Infrastructure] {
            add_crate_dependency(
                base,
                layout::WORKER_MANIFEST,
                "dependencies",
                &layout::crate_name(correo, layer),
                "{ workspace = true }",
            )?;
        }
        add_crate_dependency(
            base,
            layout::WORKER_MANIFEST,
            "dev-dependencies",
            &layout::crate_name(correo, Layer::Domain),
            "{ workspace = true, features = [\"pruebas\"] }",
        )?;
    }
    Ok(())
}

/// `consumidores/comun.rs` and its `pub mod`, unless they are there: `puerto nuevo --eventos`, and
/// `generar consumidor` in a worker written before them.
fn write_shared_module(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(layout::consumer(SHARED_CONSUMER_MODULE));
    if !path.exists() {
        write_file(&path, &render("worker/shared.tera", &tera::Context::new())?)?;
    }
    let modules = base.join(layout::CONSUMERS_RS);
    fs::write(
        &modules,
        with_module(&fs::read_to_string(&modules)?, SHARED_CONSUMER_MODULE),
    )?;
    Ok(())
}

/// What a consumer that holds use cases names: `cableado`, and the application crate of each (its
/// doubles in the tests). Added to `presentacion/worker/Cargo.toml`, never removed.
fn add_used_dependencies(
    base: &Path,
    uses: &[UsedUseCase],
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::generators::bootstrap::add_crate_dependency;
    if uses.is_empty() {
        return Ok(());
    }
    let manifest = layout::WORKER_MANIFEST;
    add_crate_dependency(
        base,
        manifest,
        "dependencies",
        "cableado",
        "{ workspace = true }",
    )?;
    for u in uses {
        let name = layout::crate_name(&u.entity_snake, Layer::Application);
        add_crate_dependency(
            base,
            manifest,
            "dependencies",
            &name,
            "{ workspace = true }",
        )?;
        add_crate_dependency(
            base,
            manifest,
            "dev-dependencies",
            &name,
            "{ workspace = true, features = [\"pruebas\"] }",
        )?;
    }
    Ok(())
}

/// `consumers.rs` with `pub mod {name};` among its module declarations, sorted as rustfmt keeps
/// them; every other line stays where it was, ahead of them.
pub(crate) fn with_module(src: &str, name: &str) -> String {
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
.PHONY: worker eventos/descartados

worker: docker-compose/up ## Arranca el worker: hace de relay del outbox y ejecuta los consumidores (health en :8081)
	@echo "${{GREEN}}Arrancando el worker...${{NC}}"
	$(CARGO) run -p worker

eventos/descartados: docker-compose/up ## Lista los 20 eventos descartados más recientes
	$(DOCKER_COMPOSE) exec -T postgres psql -U devuser -d {db_name} -c \
		"SELECT registrado_en, consumidor, tipo_evento, intentos, motivo, detalle FROM descartados ORDER BY registrado_en DESC LIMIT 20"
"#
    )
}

/// The `worker/` crate of a fresh `events` project: the member, its runtime dependencies, the
/// files, `make worker` / `make eventos/descartados` and `WORKER_PORT` — and on the `kafka`
/// transport `puerto-kafka`, Redpanda in `docker-compose.yml` and `KAFKA_BROKERS`.
pub fn apply_worker_to_new_project(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let kafka = on_kafka(&config);
    crate::generators::project::add_workspace_dependency(base, "puerto-eventos")?;
    if kafka {
        crate::generators::project::add_workspace_dependency(base, "puerto-kafka")?;
    }

    let mut ctx = tera::Context::new();
    ctx.insert("kafka", &kafka);
    for (path, template) in [
        (layout::WORKER_MANIFEST, "worker/cargo_toml.tera"),
        (
            &format!("{}/AGENTS.md", layout::WORKER_DIR),
            "worker/agents.tera",
        ),
        (
            &format!("{}/main.rs", layout::WORKER_SRC),
            "worker/main.tera",
        ),
        (
            &format!("{}/generado.rs", layout::WORKER_SRC),
            "worker/generated.tera",
        ),
        (layout::CONSUMERS_RS, "worker/consumers.tera"),
    ] {
        write_file(&base.join(path), &render(template, &ctx)?)?;
    }
    write_shared_module(base)?;
    write_file(
        &base.join(layout::REGISTRY_RS),
        &generate_registry(&config)?,
    )?;
    crate::generators::crates::sync_workspace_dependencies(base)?;

    let makefile = base.join("Makefile");
    if makefile.exists() {
        let mut src = fs::read_to_string(&makefile)?;
        if !src.contains("eventos/descartados") {
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
        }
        if kafka && !content.contains("KAFKA_BROKERS") {
            // KAFKA_TOPIC_PREFIX defaults to the project name.
            content.push_str(
                "\n# Kafka (Redpanda, en docker-compose.yml)\nKAFKA_BROKERS=localhost:9092\n",
            );
        }
        fs::write(&path, content)?;
    }
    if kafka {
        add_redpanda_to_compose(base, &config.project.name)?;
    }
    Ok(())
}

/// The Redpanda service the `kafka` transport runs against in development, next to Postgres.
fn redpanda_service(project_name: &str) -> String {
    format!(
        r#"  redpanda:
    image: docker.redpanda.com/redpandadata/redpanda:v24.2.7
    container_name: {project_name}-redpanda
    command:
      - redpanda
      - start
      - --mode
      - dev-container
      - --smp
      - "1"
      - --kafka-addr
      - PLAINTEXT://0.0.0.0:9092
      - --advertise-kafka-addr
      - PLAINTEXT://localhost:${{KAFKA_HOST_PORT:-9092}}
    ports:
      - "${{KAFKA_HOST_PORT:-9092}}:9092"
    volumes:
      - redpanda-data:/var/lib/redpanda/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "rpk cluster health | grep -q 'Healthy:.*true'"]
      interval: 5s
      timeout: 5s
      retries: 10
"#
    )
}

/// Adds Redpanda to the `docker-compose.yml` `puerto nuevo --db` wrote: its service after the
/// others, its volume after theirs.
fn add_redpanda_to_compose(
    base: &Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("docker-compose.yml");
    let src = fs::read_to_string(&path)?;
    if src.contains("redpanda:") {
        return Ok(());
    }
    let Some(at) = src.find("\nvolumes:\n") else {
        return Err(format!(
            "{}: no tiene `volumes:` en el nivel superior",
            path.display()
        )
        .into());
    };
    let out = format!(
        "{}{}{}  redpanda-data:\n",
        &src[..=at],
        redpanda_service(project_name),
        &src[at..]
    );
    fs::write(&path, out)?;
    Ok(())
}

/// The module of what several consumers share (27.14): written once, never a consumer.
pub const SHARED_CONSUMER_MODULE: &str = "comun";

#[cfg(test)]
pub fn run_generate_consumer(
    name: &str,
    events: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    run_generate_consumer_with(name, events, &[], base)
}

/// `puerto generar consumidor <name> --tras <event_type>[,…] [--usa <use_case>[,…]]`: declares the
/// consumer, writes its handler (holding the `uses` use cases, built from `cableado`), declares its
/// module and regenerates the registry. Refuses an existing name, an event no entity declares, a
/// use case no entity declares, and a file already on disk (it is never overwritten).
pub fn run_generate_consumer_with(
    name: &str,
    events: &[String],
    uses: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    if !has_worker(&config) {
        return Err(
            "este proyecto no tiene worker: los consumidores necesitan un proyecto creado con \
             `puerto nuevo --db --eventos db|kafka`"
                .into(),
        );
    }
    crate::validation::validate_consumer_name(name)?;
    if name == SHARED_CONSUMER_MODULE {
        return Err(format!(
            "'{name}' es el módulo de lo que comparten los consumidores              (presentacion/worker/src/consumidores/{name}.rs). Elige otro nombre"
        )
        .into());
    }
    let pascal = to_pascal_case(name);
    let taken = RESERVED_CONSUMER_TYPES.contains(&pascal.as_str())
        || config.entity.iter().any(|e| {
            [
                format!("Evento{}", e.name),
                format!("Snapshot{}", e.name),
                format!("Madre{}", e.name),
            ]
            .contains(&pascal)
        });
    if taken {
        return Err(format!(
            "el consumidor '{name}' daría el tipo `{pascal}`, que el worker ya usa"
        )
        .into());
    }
    if config.consumer.iter().any(|c| c.name == name) {
        return Err(format!("el consumidor '{name}' ya está declarado").into());
    }
    if let Some(mail) = name
        .strip_prefix("correo_")
        .filter(|mail| config.mail.iter().any(|m| m.name == *mail))
    {
        return Err(format!(
            "el consumidor '{name}' está reservado: el worker ejecuta el correo '{mail}' como el \
             consumidor 'correo_{mail}'. Elige otro nombre"
        )
        .into());
    }
    if events.is_empty() {
        return Err("un consumidor necesita al menos un evento: --tras <tipo_evento>".into());
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
    let uses = used_use_cases(&config, uses)?;

    let path = base.join(layout::consumer(name));
    if path.exists() {
        return Err(format!(
            "{} ya existe, y nunca se sobrescribe. Bórralo o elige otro nombre",
            path.display()
        )
        .into());
    }
    let content = generate_consumer(name, &subscribed, &uses)?;

    crate::puerto_toml::add_consumer(base, consumer)?;
    write_file(&path, &content)?;
    let modules = base.join(layout::CONSUMERS_RS);
    fs::write(&modules, with_module(&fs::read_to_string(&modules)?, name))?;
    write_shared_module(base)?;
    add_used_dependencies(base, &uses)?;
    regenerate_registry(base, &crate::puerto_toml::read(base)?)?;

    println!(
        "✓ Consumidor '{name}' añadido, tras: {}",
        subscribed
            .iter()
            .map(|e| e.event_type.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  presentacion/worker/src/consumidores/{name}.rs es tuyo: hazlo idempotente, puede recibir \
         un evento dos veces."
    );
    println!(
        "  presentacion/worker/src/generado/registro.rs regenerado. Arráncalo con `make worker`."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_sort_only_module_declarations_when_a_consumer_is_added() {
        let src = "pub mod zeta;\npub mod helpers {}\n";

        let out = with_module(src, "alpha");

        assert_eq!(out, "pub mod helpers {}\npub mod alpha;\npub mod zeta;\n");
    }

    #[test]
    fn should_not_declare_a_module_twice() {
        assert_eq!(with_module("pub mod audit;\n", "audit"), "pub mod audit;\n");
    }
}
