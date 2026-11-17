use crate::generators::crates::ContextDeps;
use crate::generators::layout::{self, Layer};
use std::{fs, path::Path};

use serde_json::json;

use crate::generators::naming::{
    crud_use_cases, entity_ctx, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, field_rust_type, is_enum_vo, is_reference, is_shared_vo, is_value_object,
    literal_import, reference_import, resolve_type, type_import, vo_import_path, vo_inner_type,
};
use crate::puerto_toml::{Field, ValueObjectDefinition};

use crate::generators::effective_fields;

/// Imports `domain/model.tera` writes itself, for the always-present system fields. Field-derived
/// imports matching these must not be emitted again.
const MODEL_TEMPLATE_IMPORTS: &[&str] = &["chrono::{DateTime, Utc}", "uuid::Uuid"];

pub fn generate_model(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    let mut imports: Vec<String> = vec![];
    for f in &eff {
        if let Some(imp) = type_import(f) {
            // `model.tera` already imports chrono and uuid for the system fields (`id`,
            // `created_at`, …). A `DateTime<Utc>` or `Uuid` field would otherwise re-import
            // them: E0252, the name is defined multiple times.
            if !MODEL_TEMPLATE_IMPORTS.contains(&imp) && !imports.iter().any(|i| i == imp) {
                imports.push(imp.to_string());
            }
        }
    }
    for f in eff.iter().filter(|f| is_value_object(f)) {
        let vo = f.value_object.as_deref().unwrap_or_default();
        let import = if is_shared_vo(f, shared_vos) {
            format!("nucleo::valores::{vo}")
        } else {
            format!("crate::valores::{vo}")
        };
        if !imports.contains(&import) {
            imports.push(import);
        }
    }
    push_reference_imports(&mut imports, &eff);

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs);
    render("domain/model.tera", &ctx)
}

/// Appends the id type of every reference among `fields`, once each.
pub(crate) fn push_reference_imports(imports: &mut Vec<String>, fields: &[Field]) {
    for import in fields.iter().filter_map(reference_import) {
        if !imports.contains(&import) {
            imports.push(import);
        }
    }
}

pub fn generate_mother(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    // A primitive field's *type* appears in the Mother struct (`Option<DateTime<Utc>>`); a VO
    // field's type is the VO, and only its default literal (`Joined::new(Utc::now())`) names a
    // primitive.
    let mut imports: Vec<String> = vec![];
    let primitive_imports = eff.iter().filter_map(|f| {
        if is_value_object(f) {
            literal_import(f)
        } else {
            type_import(f)
        }
    });
    let vo_imports = eff
        .iter()
        .filter(|f| is_value_object(f))
        .map(|f| vo_import_path(f, snake, shared_vos, true));
    for import in primitive_imports.map(str::to_string).chain(vo_imports) {
        if !imports.contains(&import) {
            imports.push(import);
        }
    }
    push_reference_imports(&mut imports, &eff);

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs);
    render("domain/mother.tera", &ctx)
}

pub fn generate_value_objects(
    pascal: &str,
    _snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);
    let vo_fields: Vec<&Field> = eff
        .iter()
        .filter(|f| is_value_object(f) && !is_shared_vo(f, shared_vos))
        .collect();

    if vo_fields.is_empty() {
        return Ok(String::new());
    }

    let mut imports = vec![format!("crate::errores::Error{pascal}")];
    for f in vo_fields.iter().filter(|f| !is_enum_vo(f)) {
        let Some(import) = resolve_type(&vo_inner_type(f))
            .ok()
            .and_then(|m| m.needs_import)
        else {
            continue;
        };
        if !imports.iter().any(|i| i == import) {
            imports.push(import.to_string());
        }
    }

    let vos: Vec<serde_json::Value> = vo_fields
        .iter()
        .map(|f| {
            json!({
                "vo": f.value_object,
                "inner": vo_inner_type(f),
                "variants": if is_enum_vo(f) { f.enum_variants.clone() } else { None },
            })
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("imports", &imports);
    ctx.insert("vos", &vos);
    render("domain/value_objects.tera", &ctx)
}

#[cfg(test)]
pub fn generate_create_use_case_trait(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, RenderError> {
    generate_use_case_trait(
        "domain/use_case_create.tera",
        &[],
        pascal,
        snake,
        fields,
        false,
    )
}

#[cfg(test)]
pub fn generate_update_use_case_trait(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, RenderError> {
    generate_use_case_trait(
        "domain/use_case_update.tera",
        &[],
        pascal,
        snake,
        fields,
        false,
    )
}

/// A use case trait whose Params carry every field as its primitive type. `template_imports` are
/// the imports the template writes itself. `actor`: a protected use case of an auth project — its
/// Params also carry the caller.
pub(crate) fn generate_use_case_trait(
    template: &str,
    template_imports: &[&str],
    pascal: &str,
    snake: &str,
    fields: &[Field],
    actor: bool,
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);
    let mut extra_imports: Vec<String> = vec![];
    for imp in eff.iter().filter_map(type_import) {
        if !template_imports.contains(&imp) && !extra_imports.iter().any(|i| i == imp) {
            extra_imports.push(imp.to_string());
        }
    }
    push_reference_imports(&mut extra_imports, &eff);
    // Params hold primitives — but a reference's id type, like the entity's own id.
    let params_fields: Vec<serde_json::Value> = eff
        .iter()
        .map(|f| {
            let ty = if is_reference(f) {
                field_rust_type(f)
            } else {
                f.field_type.clone()
            };
            json!({ "name": f.name, "field_type": ty })
        })
        .collect();

    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("extra_imports", &extra_imports);
    ctx.insert("params_fields", &params_fields);
    ctx.insert("actor", &actor);
    render(template, &ctx)
}

pub fn generate_shared_value_objects(vos: &[ValueObjectDefinition]) -> Result<String, RenderError> {
    render_shared("domain/shared_value_objects.tera", vos)
}

pub fn generate_shared_errors_combined(
    vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_shared("domain/shared_errors.tera", vos)
}

fn render_shared(template: &str, vos: &[ValueObjectDefinition]) -> Result<String, RenderError> {
    if vos.is_empty() {
        return Ok(String::new());
    }
    let vos: Vec<serde_json::Value> = vos
        .iter()
        .map(|v| json!({ "vo": v.name, "vo_snake": pascal_to_snake(&v.name), "inner": v.inner_type }))
        .collect();
    let mut ctx = tera::Context::new();
    ctx.insert("vos", &vos);
    render(template, &ctx)
}

/// `nucleo/src/{valores,errores}.rs` from the declared shared value objects, and `nucleo`'s
/// `lib.rs` declaring them.
pub fn write_shared_vo_files(
    base: &Path,
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    if shared_vos.is_empty() {
        return Ok(());
    }
    let src = base.join(layout::NUCLEO_SRC);
    write_file(
        &src.join("valores.rs"),
        &generate_shared_value_objects(shared_vos)?,
    )?;
    write_file(
        &src.join("errores.rs"),
        &generate_shared_errors_combined(shared_vos)?,
    )?;
    crate::generators::crates::write_lib(base, layout::NUCLEO_DIR, NUCLEO_HEADER)?;
    Ok(())
}

/// The doc comment of `nucleo/src/lib.rs` (the template's).
pub const NUCLEO_HEADER: &str = "//! Lo que comparten todos los contextos: los objetos valor declarados con\n\
     //! `puerto generar objeto-valor` (ADR-0023 de Puerto). Sin E/S y sin depender de ningún contexto.\n";

/// The context's domain crate: model, errors, value objects, repository port, events and unit of
/// work (`events` projects), the Object Mother, then its `Cargo.toml` and `lib.rs`.
pub fn write_domain_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let write = |module: &str, content: String| {
        write_file(&base.join(layout::domain_file(snake, module)), &content)
    };

    let eff = effective_fields(fields);
    let vo_fields: Vec<serde_json::Value> = eff
        .iter()
        .filter_map(|f| f.value_object.as_deref())
        .map(|vo| json!({ "vo_pascal": vo, "vo_snake": pascal_to_snake(vo) }))
        .collect();
    let access = crate::generators::auth::Access::of(base, pascal);
    let mut errors_ctx = entity_ctx(pascal, snake);
    errors_ctx.insert("vo_fields", &vo_fields);
    errors_ctx.insert("auth", &access.is_auth());
    errors_ctx.insert("references", &eff.iter().any(is_reference));
    if let Some(first) = eff.iter().find(|f| is_reference(f)) {
        errors_ctx.insert("first_reference", &first.name);
    }
    // The repository port filters the list by each reference (18.4).
    let mut ctx = entity_ctx(pascal, snake);
    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut reference_imports = vec![];
    push_reference_imports(&mut reference_imports, &eff);
    ctx.insert("fields", &field_ctxs);
    ctx.insert("reference_imports", &reference_imports);
    let config = crate::puerto_toml::read(base).ok();
    let events = config.as_ref().is_some_and(|c| c.project.has_events());
    let mut repository_ctx = ctx.clone();
    repository_ctx.insert("events", &events);

    write("modelo", generate_model(pascal, snake, fields, shared_vos)?)?;
    write("errores", render("domain/errors.tera", &errors_ctx)?)?;
    let valores = base.join(layout::domain_file(snake, "valores"));
    if eff
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, shared_vos))
    {
        write(
            "valores",
            generate_value_objects(pascal, snake, fields, shared_vos)?,
        )?;
    } else if valores.exists() {
        // A forced re-scaffold without local VOs: the old file names error variants that the new
        // errores.rs no longer has.
        fs::remove_file(valores)?;
    }
    write(
        "repositorio",
        render("domain/repository.tera", &repository_ctx)?,
    )?;
    if events {
        // Custom events survive a re-scaffold: they are the entity's block in puerto.toml.
        let custom = config
            .as_ref()
            .and_then(|c| c.entity.iter().find(|e| e.name == pascal))
            .map(|e| e.events.clone())
            .unwrap_or_default();
        write(
            "eventos",
            crate::generators::events::generate_events(pascal, snake, fields, shared_vos, &custom)?,
        )?;
        write(
            "unit_of_work",
            crate::generators::events::generate_domain_unit_of_work(pascal, snake)?,
        )?;
    }
    write_file(
        &base.join(layout::mother(snake)),
        &generate_mother(pascal, snake, fields, shared_vos)?,
    )?;
    write_shared_vo_files(base, shared_vos)?;
    crate::generators::crates::finish(base, snake, Layer::Domain, &ContextDeps::read(base, fields))
}

/// `puerto generar dominio <Name> [-- fields]` — the first step of the domain-first workflow.
///
/// `cli_fields` are the trailing `-- name:Type` arguments, already parsed and type-checked.
/// They are persisted to puerto.toml here, which is what makes the later layered commands
/// (`application`, `repository`, `presentation`) field-aware: they all read their fields back
/// from the manifest.
#[cfg(test)]
pub fn run_generate_domain(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    run_generate_domain_with(
        name,
        base,
        cli_fields,
        &crate::generators::scaffold::EntityShape::default(),
    )
}

/// [`run_generate_domain`] with `--unico`: written to the block for the layers that follow.
pub fn run_generate_domain_with(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
    shape: &crate::generators::scaffold::EntityShape,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    crate::validation::validate_entity_name(&pascal)?;

    if config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} is already in puerto.toml. Use `puerto generar caso-de-uso` to add a use case."
        )
        .into());
    }

    let fields: Vec<Field> = cli_fields.to_vec();
    let shared_vos = config.value_object.clone();

    crate::validation::validate_unique_field_names(&fields)?;
    if config.project.db {
        for f in &fields {
            crate::validation::validate_sql_field_name(&f.name)?;
        }
    }

    let vo_conflicts = crate::validation::validate_vo_coherence(&fields, &shared_vos);
    if !vo_conflicts.is_empty() {
        return Err(vo_conflicts.join("\n  ").into());
    }
    crate::validation::validate_references_before_writing(&pascal, &fields, &config, base)?;
    crate::generators::scaffold::check_shape(&pascal, &fields, shape)?;

    write_domain_files(&pascal, &snake, base, &fields, &shared_vos)?;
    let has_vo = fields.iter().any(is_value_object);
    let has_local_vo = fields
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, &shared_vos));

    let use_cases = crud_use_cases(&snake);
    crate::puerto_toml::add_entity(base, &pascal, use_cases, config.project.db, fields.clone())?;
    crate::generators::scaffold::record_shape(base, &pascal, config.project.db, &fields, shape)?;

    let dir = layout::crate_dir(&snake, Layer::Domain);
    if has_local_vo {
        println!("✓ {dir}/ — model, errors, value objects, repository port, Madre{pascal}");
    } else if has_vo {
        println!("✓ {dir}/ — model, errors (shared VOs), repository port, Madre{pascal}");
    } else {
        println!("✓ {dir}/ — model, errors, repository port, Madre{pascal}");
    }
    if !shared_vos.is_empty() {
        println!("✓ {}/ — shared value objects + errors", layout::NUCLEO_DIR);
    }
    println!("✓ puerto.toml         — {pascal} registered");
    println!();
    println!("  Next: puerto generar aplicacion {pascal}");
    Ok(())
}
