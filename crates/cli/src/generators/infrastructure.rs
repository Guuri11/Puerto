use std::path::Path;

use crate::generators::effective_fields;
use crate::generators::migration::run_migration;
use crate::generators::naming::{
    entity_ctx, pascal_to_snake, pluralize, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, is_shared_vo, is_value_object, literal_names_vo, resolve_type,
    seed_literal,
};
use crate::patchers::lib_rs::patch_infra_lib;
use crate::puerto_toml::{Field, ValueObjectDefinition};

// ── SQL helpers ───────────────────────────────────────────────────────────────

fn sql_ddl_col(name: &str, field_type: &str) -> String {
    let mapping = resolve_type(field_type).unwrap();
    let sql_base = match mapping.sql_type {
        "DOUBLE" => "DOUBLE PRECISION",
        other => other,
    };
    if mapping.sql_nullable {
        format!("    {name} {sql_base}")
    } else {
        let suffix = match mapping.sql_type {
            "TEXT[]" | "BIGINT[]" => " NOT NULL DEFAULT '{}'",
            "JSONB" => " NOT NULL DEFAULT '{}'",
            _ => " NOT NULL",
        };
        format!("    {name} {sql_base}{suffix}")
    }
}

fn sql_col_list(eff: &[Field]) -> String {
    let custom: String = eff.iter().map(|f| format!(", {}", f.name)).collect();
    format!("id, created_at, updated_at, deleted, deleted_at{custom}")
}

/// `HashMap<String, String>` maps to a JSONB column. sqlx has no `HashMap` codec: the row field is
/// `sqlx::types::Json<HashMap<…>>` and `query_as!` needs the column's type overridden to match.
const MAP_TYPE: &str = "HashMap<String, String>";

/// The `SELECT` column list — like [`sql_col_list`], with JSONB columns type-overridden.
fn sql_select_list(eff: &[Field]) -> String {
    let custom: String = eff
        .iter()
        .map(|f| {
            if f.field_type == MAP_TYPE {
                // Emitted inside the query's string literal, hence the escaped quotes.
                format!(r#", {name} as \"{name}: Json<{MAP_TYPE}>\""#, name = f.name)
            } else {
                format!(", {}", f.name)
            }
        })
        .collect();
    format!("id, created_at, updated_at, deleted, deleted_at{custom}")
}

fn sql_params_list(n: usize) -> String {
    let custom: String = (6..=n).map(|i| format!(", ${i}")).collect();
    format!("$1, $2, $3, $4, $5{custom}")
}

fn sql_conflict_set(eff: &[Field]) -> String {
    let custom: String = eff
        .iter()
        .enumerate()
        .map(|(i, f)| format!(", {} = ${}", f.name, 6 + i))
        .collect();
    format!("updated_at = $3, deleted = $4, deleted_at = $5{custom}")
}

// ── Dynamic generators ────────────────────────────────────────────────────────

pub fn generate_infra_entity(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    let eff = effective_fields(fields);

    let mut imports: Vec<String> = vec![];
    if eff.iter().any(|f| f.field_type == MAP_TYPE) {
        imports.push("sqlx::types::Json".to_string());
        imports.push("std::collections::HashMap".to_string());
    }
    for f in eff.iter().filter(|f| is_value_object(f)) {
        let import = business_vo_path(f, snake, shared_vos);
        if !imports.contains(&import) {
            imports.push(import);
        }
    }

    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs(&eff, pascal, shared_vos));
    render("infrastructure/entity.tera", &ctx)
}

pub fn generate_crud_infra_db_repository(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    events: bool,
) -> Result<String, RenderError> {
    let mut ctx = pg_crud_ctx(pascal, snake, fields, shared_vos);
    ctx.insert("events", &events);
    render("infrastructure/repository_pg_crud.tera", &ctx)
}

/// `infrastructure/src/{snake}/unit_of_work.rs`: the upsert the Pg repository has in a project
/// without events, inside a `puerto_sqlx::uow::PgTransaction`.
pub fn generate_pg_unit_of_work(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render(
        "infrastructure/unit_of_work_pg.tera",
        &pg_crud_ctx(pascal, snake, fields, shared_vos),
    )
}

/// What the Pg repository and the Pg unit of work render from: the SQL of the entity's table,
/// and the seed its integration tests build.
fn pg_crud_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> tera::Context {
    let eff = effective_fields(fields);

    // The seed names every plain and enum VO it constructs (`Name::new(…)`, `Status::Active`);
    // Option/Vec VOs are seeded as `None` / `vec![]` and need no import.
    let mut test_imports: Vec<String> = vec![];
    for f in eff.iter().filter(|f| literal_names_vo(f)) {
        let import = business_vo_path(f, snake, shared_vos);
        if !test_imports.contains(&import) {
            test_imports.push(import);
        }
    }
    // A `Uuid` field and every `unique` seed draw on `Uuid::new_v4()`; the file itself names no
    // `Uuid` since ids are `{Entity}Id`.
    if eff.iter().any(|f| seed_literal(f).contains("Uuid::")) {
        test_imports.push("uuid::Uuid".to_string());
    }

    // SQL text stays a Rust computation: the `$n` numbering is index-dependent.
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("all_cols", &sql_col_list(&eff));
    ctx.insert("select_cols", &sql_select_list(&eff));
    ctx.insert("has_map", &eff.iter().any(|f| f.field_type == MAP_TYPE));
    ctx.insert("all_params", &sql_params_list(5 + eff.len()));
    ctx.insert("all_updates", &sql_conflict_set(&eff));
    ctx.insert("test_imports", &test_imports);
    ctx.insert("fields", &field_ctxs(&eff, pascal, shared_vos));
    ctx
}

/// A VO's path from the infrastructure crate.
fn business_vo_path(f: &Field, snake: &str, shared_vos: &[ValueObjectDefinition]) -> String {
    let vo = f.value_object.as_deref().unwrap_or_default();
    if is_shared_vo(f, shared_vos) {
        format!("business::domain::shared::value_objects::{vo}")
    } else {
        format!("business::domain::{snake}::value_objects::{vo}")
    }
}

fn field_ctxs(eff: &[Field], pascal: &str, shared_vos: &[ValueObjectDefinition]) -> Vec<FieldCtx> {
    eff.iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect()
}

pub fn create_table_sql(snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);
    let custom_cols: String = eff
        .iter()
        .map(|f| format!("{},", sql_ddl_col(&f.name, &f.field_type)))
        .collect::<Vec<_>>()
        .join("\n");

    let table = pluralize(snake);
    let mut sql = format!(
        "CREATE TABLE {table} (\n    id UUID PRIMARY KEY,\n{custom_cols}\n    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    deleted BOOLEAN NOT NULL DEFAULT FALSE,\n    deleted_at TIMESTAMPTZ\n);\n",
    );

    // `unique = true` (the `!` suffix in the CLI) is documented as producing a DB constraint but
    // used to stop at puerto.toml. A named index rather than an inline `UNIQUE` on the column:
    // the column definitions stay uniform, and a later migration can drop the constraint by a
    // name it can predict.
    let unique_indexes: String = eff
        .iter()
        .filter(|f| f.unique)
        .map(|f| {
            let name = &f.name;
            // Partial: rows are soft-deleted, and a deleted row must not keep its value taken.
            format!(
                "\nCREATE UNIQUE INDEX {table}_{name}_key ON {table} ({name}) WHERE deleted = false;\n"
            )
        })
        .collect();
    sql.push_str(&unique_indexes);
    sql
}

pub fn write_repository_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    if db {
        write_file(
            &base.join(format!("infrastructure/src/{snake}/entity.rs")),
            &generate_infra_entity(pascal, snake, fields, shared_vos)?,
        )?;
        let events = crate::puerto_toml::events_enabled(base);
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &generate_crud_infra_db_repository(pascal, snake, fields, shared_vos, events)?,
        )?;
        if events {
            write_file(
                &base.join(format!("infrastructure/src/{snake}/unit_of_work.rs")),
                &generate_pg_unit_of_work(pascal, snake, fields, shared_vos)?,
            )?;
        }
    } else {
        let ctx = entity_ctx(pascal, snake);
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &render("infrastructure/repository_inmemory_crud.tera", &ctx)?,
        )?;
    }
    Ok(())
}

pub fn run_generate_repository(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate domain {pascal}` first."
        )
        .into());
    }

    let db = config.project.db;
    let shared_vos = config.value_object.clone();
    let fields = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_repository_files(&pascal, &snake, base, db, &fields, &shared_vos)?;
    patch_infra_lib(base, &snake, db, true)?;

    if db {
        run_migration(
            &format!("create_{snake}_table"),
            base,
            Some(&create_table_sql(&snake, &fields)),
        )?;
    }

    let repo_label = if db {
        format!("Pg{pascal}Repository")
    } else {
        format!("InMemory{pascal}Repository")
    };
    println!("✓ infrastructure/      — {repo_label}");
    println!();
    println!("  Next: puerto generate presentation {pascal}");
    Ok(())
}
