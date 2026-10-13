use std::{fs, path::Path};

use serde_json::json;

use crate::generators::naming::{
    crud_use_case, crud_use_cases, entity_ctx, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, is_enum_vo, is_shared_vo, is_value_object, literal_import, resolve_type,
    vo_import_path, vo_inner_type,
};
use crate::patchers::lib_rs::{
    patch_business_lib_domain_crud, patch_business_lib_shared, patch_business_lib_value_objects,
    patch_mothers_lib, unpatch_business_lib_value_objects,
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
        if let Some(imp) = resolve_type(&f.field_type)
            .ok()
            .and_then(|m| m.needs_import)
        {
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
            format!("crate::domain::shared::value_objects::{vo}")
        } else {
            format!("super::value_objects::{vo}")
        };
        if !imports.contains(&import) {
            imports.push(import);
        }
    }

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs);
    render("domain/model.tera", &ctx)
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
            resolve_type(&f.field_type)
                .ok()
                .and_then(|m| m.needs_import)
        }
    });
    let vo_imports = eff
        .iter()
        .filter(|f| is_value_object(f))
        .map(|f| vo_import_path(f, snake, shared_vos));
    for import in primitive_imports.map(str::to_string).chain(vo_imports) {
        if !imports.contains(&import) {
            imports.push(import);
        }
    }

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

    let mut imports = vec![format!("super::errors::{pascal}Error")];
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
fn generate_use_case_trait(
    template: &str,
    template_imports: &[&str],
    pascal: &str,
    snake: &str,
    fields: &[Field],
    actor: bool,
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);
    let mut extra_imports: Vec<&str> = vec![];
    for imp in eff.iter().filter_map(|f| {
        resolve_type(&f.field_type)
            .ok()
            .and_then(|m| m.needs_import)
    }) {
        if !template_imports.contains(&imp) && !extra_imports.contains(&imp) {
            extra_imports.push(imp);
        }
    }
    let params_fields: Vec<serde_json::Value> = eff
        .iter()
        .map(|f| json!({ "name": f.name, "field_type": f.field_type }))
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

pub fn write_shared_vo_files(
    base: &Path,
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    if shared_vos.is_empty() {
        return Ok(());
    }

    let shared_dir = base.join("business/src/domain/shared");
    fs::create_dir_all(&shared_dir)?;

    write_file(
        &shared_dir.join("value_objects.rs"),
        &generate_shared_value_objects(shared_vos)?,
    )?;

    write_file(
        &shared_dir.join("errors.rs"),
        &generate_shared_errors_combined(shared_vos)?,
    )?;

    // Declared in `shared.rs` (created here by the first shared VO), never a `shared/mod.rs`.
    let shared_rs = base.join("business/src/domain/shared.rs");
    let mut declarations = fs::read_to_string(&shared_rs).unwrap_or_default();
    for module in ["errors", "value_objects"] {
        let line = format!("pub mod {module};");
        if !declarations.lines().any(|l| l.trim() == line) {
            if !declarations.is_empty() && !declarations.ends_with('\n') {
                declarations.push('\n');
            }
            declarations.push_str(&line);
            declarations.push('\n');
        }
    }
    fs::write(&shared_rs, declarations)?;

    Ok(())
}

pub fn write_domain_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join(format!("business/src/domain/{snake}"));
    let write = |file: String, content: String| write_file(&dir.join(file), &content);

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
    let ctx = entity_ctx(pascal, snake);
    let config = crate::puerto_toml::read(base).ok();
    let events = config.as_ref().is_some_and(|c| c.project.has_events());
    let mut repository_ctx = entity_ctx(pascal, snake);
    repository_ctx.insert("events", &events);

    write(
        "model.rs".into(),
        generate_model(pascal, snake, fields, shared_vos)?,
    )?;
    write(
        "errors.rs".into(),
        render("domain/errors.tera", &errors_ctx)?,
    )?;
    if eff
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, shared_vos))
    {
        let vos = generate_value_objects(pascal, snake, fields, shared_vos)?;
        write("value_objects.rs".into(), vos)?;
    } else if dir.join("value_objects.rs").exists() {
        // A forced re-scaffold without local VOs: the old file names error variants that the new
        // errors.rs no longer has.
        std::fs::remove_file(dir.join("value_objects.rs"))?;
    }
    write(
        "repository.rs".into(),
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
            "events.rs".into(),
            crate::generators::events::generate_events(pascal, snake, fields, shared_vos, &custom)?,
        )?;
        write(
            "unit_of_work.rs".into(),
            crate::generators::events::generate_domain_unit_of_work(pascal, snake)?,
        )?;
    }
    let actor = |action: &str| access.is_protected(&crud_use_case(action, snake));
    let use_case = |action: &str| format!("use_cases/{}.rs", crud_use_case(action, snake));
    for (action, template) in [
        ("create", "domain/use_case_create.tera"),
        ("update", "domain/use_case_update.tera"),
    ] {
        write(
            use_case(action),
            generate_use_case_trait(template, &[], pascal, snake, fields, actor(action))?,
        )?;
    }
    for action in ["get", "list", "delete"] {
        let mut ctx = ctx.clone();
        ctx.insert("actor", &actor(action));
        write(
            use_case(action),
            render(&format!("domain/use_case_{action}.tera"), &ctx)?,
        )?;
    }
    Ok(())
}

pub fn write_mother(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/tests/mothers/{snake}_mother.rs")),
        &generate_mother(pascal, snake, fields, shared_vos)?,
    )?;
    Ok(())
}

/// `puerto generate domain <Name> [-- fields]` — the first step of the domain-first workflow.
///
/// `cli_fields` are the trailing `-- name:Type` arguments, already parsed and type-checked.
/// They are persisted to puerto.toml here, which is what makes the later layered commands
/// (`application`, `repository`, `presentation`) field-aware: they all read their fields back
/// from the manifest.
pub fn run_generate_domain(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    crate::validation::validate_entity_name(&pascal)?;

    if config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} is already in puerto.toml. Use `puerto generate use-case` to add a use case."
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

    write_domain_files(&pascal, &snake, base, &fields, &shared_vos)?;
    write_mother(&pascal, &snake, base, &fields, &shared_vos)?;
    patch_business_lib_domain_crud(base, &snake)?;

    let has_vo = fields.iter().any(is_value_object);
    let has_local_vo = fields
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, &shared_vos));
    if has_local_vo {
        patch_business_lib_value_objects(base, &snake)?;
    } else {
        unpatch_business_lib_value_objects(base, &snake)?;
    }

    if !shared_vos.is_empty() {
        write_shared_vo_files(base, &shared_vos)?;
        patch_business_lib_shared(base)?;
    }

    patch_mothers_lib(base, &snake)?;

    let use_cases = crud_use_cases(&snake);
    crate::puerto_toml::add_entity(base, &pascal, use_cases, config.project.db, fields.clone())?;

    if has_local_vo {
        println!(
            "✓ business/domain/    — model, errors, value_objects, repository trait, 5 use case traits"
        );
    } else if has_vo {
        println!(
            "✓ business/domain/    — model, errors (shared VOs), repository trait, 5 use case traits"
        );
    } else {
        println!("✓ business/domain/    — model, errors, repository trait, 5 use case traits");
    }
    if !shared_vos.is_empty() {
        println!("✓ business/domain/shared/ — shared value objects + errors");
    }
    println!("✓ business/tests/     — {pascal}Mother (Object Mother)");
    println!("✓ puerto.toml         — {pascal} registered");
    println!();
    println!("  Next: puerto generate application {pascal}");
    Ok(())
}
