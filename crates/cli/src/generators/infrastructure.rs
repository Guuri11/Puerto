use std::path::Path;

use crate::generators::domain::push_reference_imports;
use crate::generators::effective_fields;
use crate::generators::migration::run_create_table_migration;
use crate::generators::naming::{
    entity_ctx, pascal_to_snake, pluralize, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, is_reference, is_shared_vo, is_value_object, literal_names_vo,
    reference_id, resolve_type, seed_literal,
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

/// The `WHERE` of the list's page (`first` = 3, after `LIMIT` and `OFFSET`) or of its `COUNT`
/// (`first` = 1): live rows, then one `($n::uuid IS NULL OR field = $n)` per reference (18.4).
fn sql_list_where(eff: &[Field], first: usize) -> String {
    let filters: String = eff
        .iter()
        .filter(|f| is_reference(f))
        .enumerate()
        .map(|(i, f)| {
            let n = first + i;
            format!(" AND (${n}::uuid IS NULL OR {} = ${n})", f.name)
        })
        .collect();
    format!("deleted = false{filters}")
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
    push_reference_imports(&mut imports, &eff, "business");

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

    // SQL text stays a Rust computation: the `$n` numbering is index-dependent.
    let mut ctx = seed_ctx(pascal, snake, fields, shared_vos);
    // A row's references must exist: it is seeded with a target seeded first, through the
    // target's own repository tests (transitively, for the target's own references).
    let mut field_ctxs = field_ctxs(&eff, pascal, shared_vos);
    for f in field_ctxs
        .iter_mut()
        .filter(|f| f.is_ref && !f.field_type.starts_with("Option<"))
    {
        f.seed_literal = format!(
            "crate::{}::repository::integration_tests::seed(pool).await.id",
            f.ref_target_snake
        );
    }
    ctx.insert("fields", &field_ctxs);
    let table = pluralize(snake);
    let references: Vec<serde_json::Value> = eff
        .iter()
        .filter(|f| is_reference(f))
        .map(|f| {
            serde_json::json!({
                "constraint": foreign_key_name(&table, &f.name),
                "field": f.name,
                "id": reference_id(f),
                "import": crate::generators::types::reference_import(f, "business"),
                "optional": f.field_type.starts_with("Option<"),
            })
        })
        .collect();
    ctx.insert("references", &references);
    let mut reference_imports = vec![];
    push_reference_imports(&mut reference_imports, &eff, "business");
    ctx.insert("reference_imports", &reference_imports);
    ctx.insert(
        "seeds_references",
        &eff.iter()
            .any(|f| is_reference(f) && !f.field_type.starts_with("Option<")),
    );
    ctx.insert("all_cols", &sql_col_list(&eff));
    ctx.insert("select_cols", &sql_select_list(&eff));
    ctx.insert("has_map", &eff.iter().any(|f| f.field_type == MAP_TYPE));
    ctx.insert("all_params", &sql_params_list(5 + eff.len()));
    ctx.insert("all_updates", &sql_conflict_set(&eff));
    ctx.insert("page_where", &sql_list_where(&eff, 3));
    ctx.insert("count_where", &sql_list_where(&eff, 1));
    ctx
}

/// The entity's fields and what a repository test needs to build one from their seeds.
fn seed_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> tera::Context {
    seed_ctx_with(pascal, snake, fields, shared_vos, &[])
}

/// [`seed_ctx`] with more test imports.
fn seed_ctx_with(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    more_test_imports: &[String],
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
    for import in more_test_imports {
        if !test_imports.contains(import) {
            test_imports.push(import.clone());
        }
    }

    let mut ctx = entity_ctx(pascal, snake);
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

/// The name of a reference's foreign key — the one the repository maps back to its field.
fn foreign_key_name(table: &str, field: &str) -> String {
    format!("{table}_{field}_fkey")
}

pub fn create_table_sql(snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);
    let table = pluralize(snake);
    let custom_cols: String = eff
        .iter()
        .map(|f| match f.references.as_deref() {
            // ADR-0022: the database checks the target exists, under a name the repository maps.
            Some(target) => format!(
                "{} CONSTRAINT {} REFERENCES {} (id),",
                sql_ddl_col(&f.name, &f.field_type),
                foreign_key_name(&table, &f.name),
                pluralize(&pascal_to_snake(target)),
            ),
            None => format!("{},", sql_ddl_col(&f.name, &f.field_type)),
        })
        .collect::<Vec<_>>()
        .join("\n");

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

    // A reference is what its entity is looked up by; a unique one already has its index.
    let reference_indexes: String = eff
        .iter()
        .filter(|f| f.references.is_some() && !f.unique)
        .map(|f| {
            let name = &f.name;
            format!("\nCREATE INDEX {table}_{name}_idx ON {table} ({name});\n")
        })
        .collect();
    sql.push_str(&reference_indexes);
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
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &render(
                "infrastructure/repository_inmemory_crud.tera",
                // A reference is seeded, and filtered by, `CustomerId::generate()`: nothing
                // checks it.
                &seed_ctx_with(pascal, snake, fields, shared_vos, &{
                    let mut imports = vec![];
                    push_reference_imports(&mut imports, &effective_fields(fields), "business");
                    imports
                }),
            )?,
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
        run_create_table_migration(&snake, base, &create_table_sql(&snake, &fields))?;
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
