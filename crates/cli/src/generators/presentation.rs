use serde_json::json;
use std::path::Path;

use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::effective_fields;
use crate::generators::naming::{pascal_to_snake, to_pascal_case, write_file};
use crate::generators::render::render;
use crate::generators::types::{
    field_needs_clone, field_value_accessor, is_value_object, resolve_type,
};
use crate::patchers::api_rs::patch_api_rs;
use crate::puerto_toml::Field;

/// Imports `presentation/dto.tera` writes itself.
const DTO_TEMPLATE_IMPORTS: &[&str] = &["use uuid::Uuid;"];

fn build_dto_from_expr(f: &Field) -> String {
    format!(
        "            {}: {},",
        f.name,
        field_value_accessor(f, "entity")
    )
}

pub(crate) fn build_dto_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    is_crud: bool,
) -> tera::Context {
    let eff = effective_fields(fields);

    let dto_fields_str = eff
        .iter()
        .map(|f| format!("    pub {}: {},", f.name, f.field_type))
        .collect::<Vec<_>>()
        .join("\n");

    let dto_from_str = eff
        .iter()
        .map(build_dto_from_expr)
        .collect::<Vec<_>>()
        .join("\n");

    let request_fields_str = eff
        .iter()
        .map(|f| format!("    pub {}: {},", f.name, f.field_type))
        .collect::<Vec<_>>()
        .join("\n");

    // DTO fields keep the primitive type of the field, so the DTO file needs the same imports the
    // domain model does. `dto.tera` only imports `uuid::Uuid` itself (for the always-present `id`),
    // so a `DateTime<Utc>` or `HashMap<String, String>` field was written against a type that was
    // never in scope (E0425).
    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if DTO_TEMPLATE_IMPORTS.contains(&stmt.as_str()) {
                    continue;
                }
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }
    let extra_imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("is_crud", &is_crud);
    ctx.insert("dto_fields_str", &dto_fields_str);
    ctx.insert("dto_from_str", &dto_from_str);
    ctx.insert("request_fields_str", &request_fields_str);
    ctx.insert("extra_imports", &extra_imports_str);
    ctx
}

pub(crate) fn build_error_mapper_ctx(pascal: &str, snake: &str, fields: &[Field]) -> tera::Context {
    let eff = effective_fields(fields);
    let mut vo_names: Vec<&str> = vec![];
    for f in eff.iter().filter(|f| is_value_object(f)) {
        let vo = f.value_object.as_deref().unwrap();
        if !vo_names.contains(&vo) {
            vo_names.push(vo);
        }
    }
    let vo_arms_str: String = vo_names
        .iter()
        .map(|vo| {
            format!("\n            {pascal}Error::Invalid{vo} => StatusCode::UNPROCESSABLE_ENTITY,")
        })
        .collect();

    // One test per arm, so a status cannot drift unnoticed.
    let mut test_cases = vec![
        json!({
            "test_name": "should_map_validation_error_to_422_with_its_code",
            "error": format!("{pascal}Error::ValidationError(\"name_empty\".into())"),
            "status": "UNPROCESSABLE_ENTITY",
            "code": format!("{snake}.validation_error.name_empty"),
        }),
        json!({
            "test_name": "should_map_not_found_to_404_with_its_code",
            "error": format!("{pascal}Error::NotFound"),
            "status": "NOT_FOUND",
            "code": format!("{snake}.not_found"),
        }),
        json!({
            "test_name": "should_map_duplicate_to_409_with_its_code",
            "error": format!("{pascal}Error::Duplicate"),
            "status": "CONFLICT",
            "code": format!("{snake}.duplicate"),
        }),
        json!({
            "test_name": "should_map_repository_error_to_500_with_its_code",
            "error": format!("{pascal}Error::RepositoryError"),
            "status": "INTERNAL_SERVER_ERROR",
            "code": format!("{snake}.repository_error"),
        }),
        json!({
            "test_name": "should_map_unknown_to_500_with_its_code",
            "error": format!("{pascal}Error::Unknown"),
            "status": "INTERNAL_SERVER_ERROR",
            "code": format!("{snake}.unknown"),
        }),
    ];
    for vo in &vo_names {
        let vo_snake = pascal_to_snake(vo);
        test_cases.push(json!({
            "test_name": format!("should_map_invalid_{vo_snake}_to_422_with_its_code"),
            "error": format!("{pascal}Error::Invalid{vo}"),
            "status": "UNPROCESSABLE_ENTITY",
            "code": format!("{snake}.invalid_{vo_snake}"),
        }));
    }

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("vo_arms_str", &vo_arms_str);
    ctx.insert("test_cases", &test_cases);
    ctx
}

pub(crate) fn build_simple_ctx(pascal: &str, snake: &str) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx
}

fn build_routes_crud_ctx(pascal: &str, snake: &str, fields: &[Field]) -> tera::Context {
    let eff = effective_fields(fields);

    let create_params_str = eff
        .iter()
        .map(|f| {
            if field_needs_clone(&f.field_type) {
                format!("                {}: body.{}.clone(),", f.name, f.name)
            } else {
                format!("                {}: body.{},", f.name, f.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut update_lines = vec!["                id: id.0,".to_string()];
    for f in eff.iter() {
        if field_needs_clone(&f.field_type) {
            update_lines.push(format!(
                "                {}: body.{}.clone(),",
                f.name, f.name
            ));
        } else {
            update_lines.push(format!("                {}: body.{},", f.name, f.name));
        }
    }
    let update_params_str = update_lines.join("\n");

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("create_params_str", &create_params_str);
    ctx.insert("update_params_str", &update_params_str);
    ctx
}

#[cfg(test)]
pub fn generate_crud_dto(pascal: &str, snake: &str, fields: &[Field]) -> String {
    render(
        "presentation/dto.tera",
        &build_dto_ctx(pascal, snake, fields, true),
    )
    .expect("presentation/dto.tera render failed")
}

#[cfg(test)]
pub fn generate_crud_routes(pascal: &str, snake: &str, fields: &[Field]) -> String {
    render(
        "presentation/routes_crud.tera",
        &build_routes_crud_ctx(pascal, snake, fields),
    )
    .expect("presentation/routes_crud.tera render failed")
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
        )
        .expect("presentation/dto.tera render failed"),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/responses.rs")),
        &render(
            "presentation/responses_crud.tera",
            &build_simple_ctx(pascal, snake),
        )
        .expect("presentation/responses_crud.tera render failed"),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
        &render(
            "presentation/error_mapper.tera",
            &build_error_mapper_ctx(pascal, snake, fields),
        )
        .expect("presentation/error_mapper.tera render failed"),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/routes.rs")),
        &render(
            "presentation/routes_crud.tera",
            &build_routes_crud_ctx(pascal, snake, fields),
        )
        .expect("presentation/routes_crud.tera render failed"),
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
    regenerate_bootstrap(base)?;

    println!("✓ presentation/        — routes, dto, responses, error_mapper");
    println!("✓ bootstrap.rs         — regenerated");
    println!();
    println!("  All layers wired. Run `make run` to start.");
    Ok(())
}
