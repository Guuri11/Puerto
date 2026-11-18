//! Domain events in generated projects (ADR-0018): `events.rs` (always regenerated from
//! puerto.toml), the unit of work ports, and `puerto generar evento`.

use crate::generators::layout;
use std::path::Path;

use serde::Serialize;

use crate::generators::bootstrap::{regenerate_bootstrap, uses_unit_of_work};
use crate::generators::effective_fields;
use crate::generators::naming::{
    crud_events, entity_ctx, event_variant, pascal_to_snake, snake_to_camel, to_pascal_case,
    write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{FieldCtx, field_ctx, resolve_type};
use crate::puerto_toml::{Field, PuertoConfig, ValueObjectDefinition};

/// Imports `domain/events.tera` writes itself.
const EVENTS_TEMPLATE_IMPORTS: &[&str] = &["uuid::Uuid"];

#[derive(Serialize)]
struct EventCtx {
    variant: String,
    event_type: String,
    /// The `pub const` naming the event type: `PRODUCT_CREATED`.
    constant: String,
}

/// `contextos/{snake}/dominio/src/eventos.rs`: the snapshot, the CRUD events plus `custom` ones.
pub fn generate_events(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    custom: &[String],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    let mut imports: Vec<&str> = vec![];
    for import in eff.iter().filter_map(|f| {
        resolve_type(&f.field_type)
            .ok()
            .and_then(|m| m.needs_import)
    }) {
        if !EVENTS_TEMPLATE_IMPORTS.contains(&import) && !imports.contains(&import) {
            imports.push(import);
        }
    }

    let events: Vec<EventCtx> = crud_events(snake)
        .into_iter()
        .chain(
            custom
                .iter()
                .map(|event| (event_variant(snake, event), event.clone())),
        )
        .map(|(variant, event_type)| EventCtx {
            variant,
            constant: event_type.to_uppercase(),
            event_type,
        })
        .collect();

    let mut json_keys: Vec<String> = std::iter::once("id".to_string())
        .chain(eff.iter().map(|f| snake_to_camel(&f.name)))
        .collect();
    json_keys.sort_unstable();

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs);
    ctx.insert("events", &events);
    ctx.insert("json_keys", &json_keys);
    render("domain/events.tera", &ctx)
}

/// The ports a CRUD entity's writes go through, and their recording fake.
pub fn generate_domain_unit_of_work(pascal: &str, snake: &str) -> Result<String, RenderError> {
    render("domain/unit_of_work.tera", &entity_ctx(pascal, snake))
}

/// Rewrites `events.rs` of every entity whose writes go through a unit of work — after a scaffold,
/// `generar bootstrap` and `generar evento`, like `bootstrap.rs`.
pub fn regenerate_events(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    for entity in config
        .entity
        .iter()
        .filter(|e| uses_unit_of_work(e, config.project.has_events()))
    {
        let snake = pascal_to_snake(&entity.name);
        let path = base.join(layout::domain_file(&snake, "eventos"));
        // An entity declared but not scaffolded yet: its drift is reported by bootstrap.
        if !base.join(layout::domain_file(&snake, "modelo")).exists() {
            continue;
        }
        write_file(
            &path,
            &generate_events(
                &entity.name,
                &snake,
                &entity.fields,
                &config.value_object,
                &entity.events,
            )?,
        )?;
    }
    Ok(())
}

/// `puerto generar evento <Entity> <name>`: declares a custom event and regenerates `events.rs`.
/// No use case records it until one is written to.
pub fn run_generate_event(
    entity: &str,
    event: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    if !config.project.has_events() {
        return Err(
            "este proyecto no tiene eventos de dominio: necesitan un proyecto creado con \
                    `puerto nuevo --db --eventos db`"
                .into(),
        );
    }
    let pascal = to_pascal_case(entity);
    let snake = pascal_to_snake(&pascal);
    let declared = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .ok_or_else(|| format!("la entidad '{pascal}' no está en puerto.toml"))?;
    if !uses_unit_of_work(declared, true) {
        return Err(format!(
            "{pascal} no es una entidad CRUD: solo las escrituras de las entidades del scaffold \
             registran eventos"
        )
        .into());
    }
    crate::validation::validate_event_name(&snake, event, &declared.events)?;

    crate::puerto_toml::add_event(base, &pascal, event)?;
    regenerate_bootstrap(base)?;

    println!(
        "✓ Evento '{event}' añadido a {pascal}: Evento{pascal}::{}",
        event_variant(&snake, event)
    );
    println!("  {} regenerado.", layout::domain_file(&snake, "eventos"));
    println!(
        "  Emítelo desde un método de {pascal} que devuelva Evento{pascal}::{}(Snapshot{pascal}::from_entidad(self)), y guárdalo con self.uow_factory.guardar(&entidad, evento)",
        event_variant(&snake, event)
    );
    Ok(())
}
