use crate::generators::layout::{self, Layer};
use std::path::Path;

use crate::generators::crates::ContextDeps;
use crate::generators::effective_fields;
use crate::generators::merge::merge_use_case;
use crate::generators::naming::{
    crud_use_case, entity_ctx, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{FieldCtx, field_ctx, is_value_object, vo_import_path};
use crate::puerto_toml::{Field, ValueObjectDefinition};

// ── Context ───────────────────────────────────────────────────────────────────

/// The context every application template renders from.
///
/// The impls construct VOs (`Name::new(params.name)?`) but never name a primitive type — the params
/// carry those — so the file level gets VO imports only; a primitive import there is dead in a
/// non-test build (`unused_imports`, `-D warnings` in the generated project's `make lint`). The
/// tests write no literal of their own: their values come from the mother (Phase 20.5).
fn use_case_context(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> tera::Context {
    let eff = effective_fields(fields);

    let imports_of = |keep: &dyn Fn(&Field) -> bool| {
        let mut imports: Vec<String> = vec![];
        for import in eff
            .iter()
            .filter(|f| is_value_object(f) && keep(f))
            .map(|f| vo_import_path(f, snake, shared_vos, false))
        {
            if !imports.contains(&import) {
                imports.push(import);
            }
        }
        imports
    };
    let vo_imports = imports_of(&|_| true);
    // `actualizar` keeps a generated field (27.17): it never builds its value object.
    let update_vo_imports = imports_of(&|f| !f.generated);

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    crate::generators::types::insert_field_lists(&mut ctx, &field_ctxs);
    ctx.insert("vo_imports", &vo_imports);
    ctx.insert("update_vo_imports", &update_vo_imports);
    // Every reference's id: the list's filter test names the optional ones too.
    let mut reference_imports = vec![];
    crate::generators::domain::push_reference_imports(&mut reference_imports, &eff);
    ctx.insert("reference_imports", &reference_imports);
    ctx
}

#[cfg(test)]
fn render_use_case(
    action: &str,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case_as(
        action,
        pascal,
        snake,
        fields,
        shared_vos,
        false,
        &Owner::default(),
    )
}

/// `actor`: a protected use case of an auth project — its tests pass a `PrincipalMother` caller.
fn render_use_case_as(
    action: &str,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    actor: bool,
    owner: &Owner,
) -> Result<String, RenderError> {
    let mut ctx = use_case_context(pascal, snake, fields, shared_vos);
    ctx.insert("actor", &actor);
    insert_owner(&mut ctx, owner);
    render(&format!("application/{action}.tera"), &ctx)
}

/// 27.18: the owner field and the roles of the route that also reach a row — for `obtener`,
/// `actualizar` and `borrar` of an owned entity; nothing otherwise.
#[derive(Default)]
struct Owner {
    field: Option<String>,
    roles: Vec<String>,
}

fn insert_owner(ctx: &mut tera::Context, owner: &Owner) {
    ctx.insert("owner", &owner.field.clone().unwrap_or_default());
    ctx.insert("owner_roles", &owner.roles);
}

/// The writes of an `events` project: through the unit of work, recording their event.
fn render_write_use_case_with_events(
    action: &str,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    actor: bool,
    owner: &Owner,
) -> Result<String, RenderError> {
    let mut ctx = use_case_context(pascal, snake, fields, shared_vos);
    ctx.insert("actor", &actor);
    insert_owner(&mut ctx, owner);
    render(&format!("application/{action}_events.tera"), &ctx)
}

// ── Public generators ─────────────────────────────────────────────────────────

#[cfg(test)]
pub fn generate_create_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("create", pascal, snake, fields, shared_vos)
}

#[cfg(test)]
pub fn generate_get_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("get", pascal, snake, fields, shared_vos)
}

#[cfg(test)]
pub fn generate_list_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("list", pascal, snake, fields, shared_vos)
}

#[cfg(test)]
pub fn generate_update_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("update", pascal, snake, fields, shared_vos)
}

#[cfg(test)]
pub fn generate_delete_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("delete", pascal, snake, fields, shared_vos)
}

// ── File writers ──────────────────────────────────────────────────────────────

/// The contract of a CRUD use case: params, trait and double. `actor`: a protected use case of an
/// auth project — its params carry the caller.
fn render_contract(
    action: &str,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    actor: bool,
) -> Result<String, RenderError> {
    let template = format!("domain/use_case_{action}.tera");
    if matches!(action, "create" | "update") {
        return crate::generators::domain::generate_use_case_trait(
            &template,
            &[],
            pascal,
            snake,
            fields,
            actor,
        );
    }
    let eff = effective_fields(fields);
    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut reference_imports = vec![];
    crate::generators::domain::push_reference_imports(&mut reference_imports, &eff);
    let mut ctx = entity_ctx(pascal, snake);
    crate::generators::types::insert_field_lists(&mut ctx, &field_ctxs);
    ctx.insert("reference_imports", &reference_imports);
    ctx.insert("actor", &actor);
    render(&template, &ctx)
}

/// The context's application crate: one file per CRUD use case — contract, implementation and
/// tests (ADR-0023 § 4) — then its `Cargo.toml` and `lib.rs`. A custom use case's file is kept.
pub fn write_application_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let access = crate::generators::auth::Access::of(base, pascal);
    let config = crate::puerto_toml::read(base).ok();
    let events = config.as_ref().is_some_and(|c| c.project.has_events());
    for action in ["create", "get", "list", "update", "delete"] {
        let use_case = crud_use_case(action, snake);
        let actor = access.is_protected(&use_case);
        let owner = match fields.iter().find(|f| f.owner) {
            Some(field) if actor && matches!(action, "get" | "update" | "delete") => Owner {
                field: Some(field.name.clone()),
                roles: access.roles(&use_case),
            },
            _ => Owner::default(),
        };
        // An `events` project's writes go through the unit of work, recording their event.
        let implementation = if events && matches!(action, "create" | "update" | "delete") {
            render_write_use_case_with_events(
                action, pascal, snake, fields, shared_vos, actor, &owner,
            )?
        } else {
            render_use_case_as(action, pascal, snake, fields, shared_vos, actor, &owner)?
        };
        let contract = render_contract(action, pascal, snake, fields, shared_vos, actor)?;
        write_file(
            &base.join(layout::use_case(snake, &use_case)),
            &merge_use_case(&contract, &implementation),
        )?;
    }
    // 27.18: an owned entity also lists its caller's rows.
    if let Some(owner) = fields.iter().find(|f| f.owner) {
        let mut ctx = entity_ctx(pascal, snake);
        ctx.insert("owner", &owner.name);
        // Every field of the filter, in order: the owner's is the caller, the references' none.
        let filters: Vec<(&str, bool)> = fields
            .iter()
            .filter(|f| f.owner || f.references.is_some())
            .map(|f| (f.name.as_str(), f.owner))
            .collect();
        ctx.insert("filters", &filters);
        write_file(
            &base.join(layout::use_case(
                snake,
                &crate::generators::naming::mine_use_case(snake),
            )),
            &render("application/list_mine.tera", &ctx)?,
        )?;
    }
    crate::generators::crates::finish(
        base,
        snake,
        Layer::Application,
        &ContextDeps::read(base, fields),
    )
}

pub fn run_generate_application(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} no está en puerto.toml. Ejecuta antes `puerto generar dominio {pascal}`."
        )
        .into());
    }

    let fields: Vec<Field> = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();
    let shared_vos = config.value_object.clone();

    write_application_files(&pascal, &snake, base, &fields, &shared_vos)?;

    println!(
        "✓ {}/: 5 casos de uso (crear, obtener, listar, actualizar, borrar)",
        layout::crate_dir(&snake, Layer::Application)
    );
    println!();
    println!("  Siguiente: puerto generar repositorio {pascal}");
    Ok(())
}
