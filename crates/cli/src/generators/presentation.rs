use serde_json::json;
use std::path::Path;

use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::effective_fields;
use crate::generators::naming::{entity_ctx, pascal_to_snake, to_pascal_case, write_file};
use crate::generators::render::render;
use crate::generators::types::{FieldCtx, field_ctx, literal_import, resolve_type};
use crate::patchers::api_rs::patch_api_rs;
use crate::puerto_toml::Field;

pub(crate) fn build_dto_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    is_crud: bool,
) -> tera::Context {
    let eff = effective_fields(fields);

    // DTO fields keep the primitive type of the field, so the DTO file needs the same imports the
    // domain model does — except `uuid::Uuid`, which `dto.tera` imports itself for the
    // always-present `id`. Without them a `DateTime<Utc>` or `HashMap` field names a type that was
    // never in scope (E0425).
    let mut imports: Vec<&str> = vec![];
    for import in eff.iter().filter_map(|f| {
        resolve_type(&f.field_type)
            .ok()
            .and_then(|m| m.needs_import)
    }) {
        if import != "uuid::Uuid" && !imports.contains(&import) {
            imports.push(import);
        }
    }

    let mut ctx = simple_ctx(pascal, snake, &eff);
    ctx.insert("is_crud", &is_crud);
    ctx.insert("imports", &imports);
    ctx
}

pub(crate) fn build_error_mapper_ctx(pascal: &str, snake: &str, fields: &[Field]) -> tera::Context {
    let eff = effective_fields(fields);
    let mut vos: Vec<serde_json::Value> = vec![];
    for vo in eff.iter().filter_map(|f| f.value_object.as_deref()) {
        let entry = json!({ "vo": vo, "vo_snake": pascal_to_snake(vo) });
        if !vos.contains(&entry) {
            vos.push(entry);
        }
    }
    let mut ctx = build_simple_ctx(pascal, snake);
    ctx.insert("vos", &vos);
    ctx
}

/// `pascal`, `snake` and one [`FieldCtx`] per field.
fn simple_ctx(pascal: &str, snake: &str, eff: &[Field]) -> tera::Context {
    let fields: Vec<FieldCtx> = eff.iter().map(|f| field_ctx(f, pascal, &[])).collect();
    let mut ctx = build_simple_ctx(pascal, snake);
    ctx.insert("fields", &fields);
    ctx
}

pub(crate) fn build_simple_ctx(pascal: &str, snake: &str) -> tera::Context {
    entity_ctx(pascal, snake)
}

fn build_routes_crud_ctx(pascal: &str, snake: &str, fields: &[Field]) -> tera::Context {
    let eff = effective_fields(fields);
    // The route tests build request bodies from the registry defaults (`Utc::now()`,
    // `HashMap::new()`, …). `uuid::Uuid` is already imported by the routes themselves.
    let mut test_imports: Vec<&str> = vec![];
    for import in eff.iter().filter_map(literal_import) {
        if import != "uuid::Uuid" && !test_imports.contains(&import) {
            test_imports.push(import);
        }
    }
    let mut ctx = simple_ctx(pascal, snake, &eff);
    ctx.insert("test_imports", &test_imports);
    ctx
}

#[cfg(test)]
pub fn generate_crud_dto(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, crate::generators::render::RenderError> {
    render(
        "presentation/dto.tera",
        &build_dto_ctx(pascal, snake, fields, true),
    )
}

#[cfg(test)]
pub fn generate_crud_routes(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, crate::generators::render::RenderError> {
    render(
        "presentation/routes_crud.tera",
        &build_routes_crud_ctx(pascal, snake, fields),
    )
}

pub fn write_presentation_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("presentation/src/api/{snake}.rs")),
        "pub mod dto;\npub mod error_mapper;\npub mod responses;\npub mod routes;\n",
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/dto.rs")),
        &render(
            "presentation/dto.tera",
            &build_dto_ctx(pascal, snake, fields, true),
        )?,
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/responses.rs")),
        &render(
            "presentation/responses_crud.tera",
            &build_simple_ctx(pascal, snake),
        )?,
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
        &render(
            "presentation/error_mapper.tera",
            &build_error_mapper_ctx(pascal, snake, fields),
        )?,
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/routes.rs")),
        &render(
            "presentation/routes_crud.tera",
            &build_routes_crud_ctx(pascal, snake, fields),
        )?,
    )?;
    Ok(())
}

pub fn run_generate_presentation(
    name: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate domain {pascal}` first."
        )
        .into());
    }

    let fields = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_presentation_files(&pascal, &snake, base, &fields)?;
    patch_api_rs(base, &snake)?;
    crate::patchers::routes_rs::patch_routes_from_manifest(base, &pascal, &snake)?;
    println!("✓ presentation/        — routes, dto, responses, error_mapper");

    // API first is allowed: the routes only need the domain. `bootstrap.rs` also imports the use
    // case implementations and the repository adapter, so it waits for them — written now, it
    // would not compile.
    let entity = config.entity.iter().find(|e| e.name == pascal);
    let missing: Vec<_> = entity
        .map(crate::generators::consistency::bootstrap_required_files)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| !base.join(path).exists())
        .collect();
    if missing.is_empty() {
        regenerate_bootstrap(base)?;
        println!("✓ bootstrap.rs         — regenerated");
        println!();
        println!("  All layers wired. Run `make run` to start.");
        return Ok(());
    }

    // The routes tag their endpoints `ApiTags::{pascal}`.
    write_file(
        &base.join("presentation/src/api/tags.rs"),
        &crate::generators::bootstrap::generate_tags_content(&config.entity)?,
    )?;
    let next = if missing.iter().any(|p| p.starts_with("business")) {
        "application"
    } else {
        "repository"
    };
    println!("✓ tags.rs              — regenerated");
    println!();
    println!("  bootstrap.rs not regenerated yet: {pascal} has no {next} layer.");
    println!("  Next: puerto generate {next} {pascal}, then `puerto generate bootstrap`.");
    Ok(())
}
