use std::{fs, path::Path};

use serde_json::json;

use crate::generators::naming::{pascal_to_snake, to_pascal_case, write_file};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, is_enum_vo, is_shared_vo, is_value_object, literal_import, resolve_type,
    vo_import_path, vo_inner_type,
};
use crate::patchers::lib_rs::{
    patch_business_lib_domain_crud, patch_business_lib_shared, patch_business_lib_value_objects,
    patch_lib_block,
};
use crate::puerto_toml::{Field, ValueObjectDefinition};

use crate::generators::effective_fields;

/// Imports `domain/model.tera` writes itself, for the always-present system fields. Field-derived
/// imports matching these must not be emitted again.
const MODEL_TEMPLATE_IMPORTS: &[&str] = &["chrono::{DateTime, Utc}", "uuid::Uuid"];

/// Imports `domain/use_case_update.tera` writes itself, for the always-present `id: Uuid` param.
/// A `Uuid` *field* resolves to the same import — emitting it again is a duplicate import (E0252).
const UPDATE_USE_CASE_TEMPLATE_IMPORTS: &[&str] = &["use uuid::Uuid;"];

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

    let field_ctxs: Vec<FieldCtx> = eff.iter().map(|f| field_ctx(f, shared_vos)).collect();
    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
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

    let field_ctxs: Vec<FieldCtx> = eff.iter().map(|f| field_ctx(f, shared_vos)).collect();
    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
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

    let mut file_imports: Vec<String> = vec![format!("use super::errors::{}Error;", pascal)];
    for f in &vo_fields {
        if is_enum_vo(f) {
            continue;
        }
        let inner = vo_inner_type(f);
        let imp = match inner.as_str() {
            "Uuid" => Some("use uuid::Uuid;".to_string()),
            "DateTime<Utc>" => Some("use chrono::{DateTime, Utc};".to_string()),
            _ => None,
        };
        if let Some(i) = imp {
            if !file_imports.contains(&i) {
                file_imports.push(i);
            }
        }
    }

    let mut vo_structs: Vec<String> = vec![];
    for f in &vo_fields {
        let vo = f.value_object.as_deref().unwrap();
        if is_enum_vo(f) {
            let variants = f.enum_variants.as_deref().unwrap();
            let variant_lines = variants
                .iter()
                .map(|v| format!("    {},", v))
                .collect::<Vec<_>>()
                .join("\n");
            let from_str_arms = variants
                .iter()
                .map(|v| format!("            \"{}\" => Ok(Self::{}),", v, v))
                .collect::<Vec<_>>()
                .join("\n");
            let as_str_arms = variants
                .iter()
                .map(|v| format!("            Self::{} => \"{}\",", v, v))
                .collect::<Vec<_>>()
                .join("\n");
            let unknown_arm = format!("            _ => Err({pascal}Error::Invalid{vo}),");
            vo_structs.push(format!(
                "#[derive(Debug, Clone, PartialEq)]\npub enum {vo} {{\n{variant_lines}\n}}\n\nimpl std::str::FromStr for {vo} {{\n    type Err = {pascal}Error;\n\n    fn from_str(s: &str) -> Result<Self, Self::Err> {{\n        match s {{\n{from_str_arms}\n{unknown_arm}\n        }}\n    }}\n}}\n\nimpl {vo} {{\n    pub fn as_str(&self) -> &'static str {{\n        match self {{\n{as_str_arms}\n        }}\n    }}\n}}"
            ));
            continue;
        }
        let inner_type = vo_inner_type(f);
        let struct_code = match inner_type.as_str() {
            "String" => format!(
                "#[derive(Debug, Clone, PartialEq)]\npub struct {vo} {{\n    value: String,\n}}\n\nimpl {vo} {{\n    pub fn new(value: String) -> Result<Self, {pascal}Error> {{\n        let trimmed = value.trim().to_string();\n        if trimmed.is_empty() {{\n            return Err({pascal}Error::Invalid{vo});\n        }}\n        Ok(Self {{ value: trimmed }})\n    }}\n\n    pub fn value(&self) -> &str {{\n        &self.value\n    }}\n}}"
            ),
            "i64" | "f64" | "bool" => format!(
                "#[derive(Debug, Clone, Copy, PartialEq)]\npub struct {vo}({inner_type});\n\nimpl {vo} {{\n    pub fn new(value: {inner_type}) -> Result<Self, {pascal}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> {inner_type} {{\n        self.0\n    }}\n}}"
            ),
            "Uuid" => format!(
                "#[derive(Debug, Clone, Copy, PartialEq)]\npub struct {vo}(Uuid);\n\nimpl {vo} {{\n    pub fn new(value: Uuid) -> Result<Self, {pascal}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> Uuid {{\n        self.0\n    }}\n}}"
            ),
            "DateTime<Utc>" => format!(
                "#[derive(Debug, Clone, PartialEq)]\npub struct {vo}(DateTime<Utc>);\n\nimpl {vo} {{\n    pub fn new(value: DateTime<Utc>) -> Result<Self, {pascal}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> DateTime<Utc> {{\n        self.0\n    }}\n}}"
            ),
            _ => continue,
        };
        vo_structs.push(struct_code);
    }

    let file_imports_str = file_imports.join("\n");
    let vo_structs_str = vo_structs.join("\n\n");
    let mut ctx = tera::Context::new();
    ctx.insert("file_imports", &file_imports_str);
    ctx.insert("vo_structs", &vo_structs_str);
    render("domain/value_objects.tera", &ctx)
}

pub fn generate_create_use_case_trait(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }

    let params_fields: Vec<serde_json::Value> = eff
        .iter()
        .map(|f| json!({ "name": f.name, "field_type": f.field_type }))
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("extra_imports", &extra_imports);
    ctx.insert("params_fields", &params_fields);

    render("domain/use_case_create.tera", &ctx)
}

pub fn generate_update_use_case_trait(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if UPDATE_USE_CASE_TEMPLATE_IMPORTS.contains(&stmt.as_str()) {
                    continue;
                }
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }

    let params_fields: Vec<serde_json::Value> = eff
        .iter()
        .map(|f| json!({ "name": f.name, "field_type": f.field_type }))
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("extra_imports", &extra_imports);
    ctx.insert("params_fields", &params_fields);

    render("domain/use_case_update.tera", &ctx)
}

pub fn generate_shared_value_objects(vos: &[ValueObjectDefinition]) -> Result<String, RenderError> {
    if vos.is_empty() {
        return Ok(String::new());
    }

    let mut vo_structs: Vec<String> = vec![];
    for vo_def in vos {
        let vo = &vo_def.name;
        let inner = &vo_def.inner_type;
        let struct_code = match inner.as_str() {
            "String" => format!(
                "use super::errors::{vo}Error;\n\n#[derive(Debug, Clone, PartialEq)]\npub struct {vo} {{\n    value: String,\n}}\n\nimpl {vo} {{\n    pub fn new(value: String) -> Result<Self, {vo}Error> {{\n        let trimmed = value.trim().to_string();\n        if trimmed.is_empty() {{\n            return Err({vo}Error::Invalid);\n        }}\n        Ok(Self {{ value: trimmed }})\n    }}\n\n    pub fn value(&self) -> &str {{\n        &self.value\n    }}\n}}"
            ),
            "i64" | "f64" | "bool" => format!(
                "use super::errors::{vo}Error;\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct {vo}({inner});\n\nimpl {vo} {{\n    pub fn new(value: {inner}) -> Result<Self, {vo}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> {inner} {{\n        self.0\n    }}\n}}"
            ),
            "Uuid" => format!(
                "use super::errors::{vo}Error;\nuse uuid::Uuid;\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct {vo}(Uuid);\n\nimpl {vo} {{\n    pub fn new(value: Uuid) -> Result<Self, {vo}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> Uuid {{\n        self.0\n    }}\n}}"
            ),
            "DateTime<Utc>" => format!(
                "use super::errors::{vo}Error;\nuse chrono::{{DateTime, Utc}};\n\n#[derive(Debug, Clone, PartialEq)]\npub struct {vo}(DateTime<Utc>);\n\nimpl {vo} {{\n    pub fn new(value: DateTime<Utc>) -> Result<Self, {vo}Error> {{\n        Ok(Self(value))\n    }}\n\n    pub fn value(&self) -> DateTime<Utc> {{\n        self.0\n    }}\n}}"
            ),
            _ => continue,
        };
        vo_structs.push(struct_code);
    }

    let vo_structs_str = vo_structs.join("\n\n");
    let mut ctx = tera::Context::new();
    ctx.insert("vo_structs", &vo_structs_str);
    render("domain/shared_value_objects.tera", &ctx)
}

#[allow(dead_code)]
pub fn generate_shared_errors(vos: &[ValueObjectDefinition]) -> String {
    if vos.is_empty() {
        return String::new();
    }

    let mut error_variants: Vec<String> = vec![];
    for vo_def in vos {
        let vo = &vo_def.name;
        let vo_snake = pascal_to_snake(vo);
        error_variants.push(format!(
            "    #[error(\"shared.value_object.{}.invalid\")]\n    Invalid,",
            vo_snake
        ));
    }

    let variants_str = error_variants.join("\n");

    format!(
        r#"use thiserror::Error;

#[derive(Debug, Error)]
pub enum {vo}Error {{
{variants}
}}"#,
        vo = if vos.len() == 1 {
            vos[0].name.clone()
        } else {
            "Shared".to_string()
        },
        variants = variants_str,
    )
}

pub fn generate_shared_errors_combined(
    vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    if vos.is_empty() {
        return Ok(String::new());
    }

    let error_defs = vos
        .iter()
        .map(|vo_def| {
            let vo = &vo_def.name;
            let vo_snake = pascal_to_snake(vo);
            format!(
                "#[derive(Debug, Error)]\npub enum {vo}Error {{\n    #[error(\"shared.value_object.{vo_snake}.invalid\")]\n    Invalid,\n}}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut ctx = tera::Context::new();
    ctx.insert("error_defs", &error_defs);
    render("domain/shared_errors.tera", &ctx)
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

    write_file(
        &shared_dir.join("mod.rs"),
        "pub mod errors;\npub mod value_objects;\n",
    )?;

    Ok(())
}

pub fn write_domain_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &generate_model(pascal, snake, fields, shared_vos)?,
    )?;

    let eff = effective_fields(fields);
    let has_local_vo = eff
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, shared_vos));

    let vo_fields: Vec<serde_json::Value> = eff
        .iter()
        .filter(|f| is_value_object(f))
        .map(|f| {
            let vo = f.value_object.as_deref().unwrap();
            json!({ "vo_pascal": vo, "vo_snake": pascal_to_snake(vo) })
        })
        .collect();
    let mut errors_ctx = tera::Context::new();
    errors_ctx.insert("pascal", pascal);
    errors_ctx.insert("snake", snake);
    errors_ctx.insert("vo_fields", &vo_fields);
    write_file(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &render("domain/errors.tera", &errors_ctx)?,
    )?;

    if has_local_vo {
        let vo_content = generate_value_objects(pascal, snake, fields, shared_vos)?;
        write_file(
            &base.join(format!("business/src/domain/{snake}/value_objects.rs")),
            &vo_content,
        )?;
    }

    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    write_file(
        &base.join(format!("business/src/domain/{snake}/repository.rs")),
        &render("domain/repository.tera", &ctx)?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &generate_create_use_case_trait(pascal, snake, fields)?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/get_{snake}.rs"
        )),
        &render("domain/use_case_get.tera", &ctx)?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/list_{snake}.rs"
        )),
        &render("domain/use_case_list.tera", &ctx)?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/update_{snake}.rs"
        )),
        &generate_update_use_case_trait(pascal, snake, fields)?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/delete_{snake}.rs"
        )),
        &render("domain/use_case_delete.tera", &ctx)?,
    )?;
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

pub fn patch_mothers_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake}_mother;")) {
        return Ok(());
    }

    let new_mod = format!("\n        pub mod {snake}_mother;\n");

    if let Ok(patched) = patch_lib_block(&src, &["tests", "mothers"], &new_mod) {
        fs::write(&path, patched)?;
        return Ok(());
    }

    let mut content = src;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n#[cfg(test)]\npub mod tests {{\n    pub mod mothers {{\n        pub mod {snake}_mother;\n    }}\n}}\n"
    ));
    fs::write(&path, content)?;
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

    write_domain_files(&pascal, &snake, base, &fields, &shared_vos)?;
    write_mother(&pascal, &snake, base, &fields, &shared_vos)?;
    patch_business_lib_domain_crud(base, &snake)?;

    let has_vo = fields.iter().any(is_value_object);
    let has_local_vo = fields
        .iter()
        .any(|f| is_value_object(f) && !is_shared_vo(f, &shared_vos));
    if has_local_vo {
        patch_business_lib_value_objects(base, &snake)?;
    }

    if !shared_vos.is_empty() {
        write_shared_vo_files(base, &shared_vos)?;
        patch_business_lib_shared(base)?;
    }

    patch_mothers_lib(base, &snake)?;

    let use_cases = vec![
        format!("create_{snake}"),
        format!("get_{snake}"),
        format!("list_{snake}"),
        format!("update_{snake}"),
        format!("delete_{snake}"),
    ];
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
