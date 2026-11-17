use crate::generators::layout::{self, Layer};
use std::path::Path;

use crate::generators::crates::ContextDeps;
use crate::generators::domain::push_reference_imports;
use crate::generators::effective_fields;
use crate::generators::migration::run_create_table_migration;
use crate::generators::naming::{
    entity_ctx, pascal_to_snake, pluralize, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::scaffold::EntityShape;
use crate::generators::types::{
    FieldCtx, field_ctx, is_enum_vo, is_reference, is_value_object, literal_names_vo, reference_id,
    resolve_type, seed_literal, vo_import_path,
};
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
    format!("id, creado_en, actualizado_en, borrado, borrado_en{custom}")
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
    format!("id, creado_en, actualizado_en, borrado, borrado_en{custom}")
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
    format!("borrado = false{filters}")
}

fn sql_conflict_set(eff: &[Field]) -> String {
    let custom: String = eff
        .iter()
        .enumerate()
        .map(|(i, f)| format!(", {} = ${}", f.name, 6 + i))
        .collect();
    format!("actualizado_en = $3, borrado = $4, borrado_en = $5{custom}")
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
        let import = vo_import_path(f, snake, shared_vos, false);
        if !imports.contains(&import) {
            imports.push(import);
        }
    }
    push_reference_imports(&mut imports, &eff);

    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("imports", &imports);
    ctx.insert("fields", &field_ctxs(&eff, pascal, shared_vos));
    render("infrastructure/entity.tera", &ctx)
}

#[cfg(test)]
pub fn generate_crud_infra_db_repository(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    events: bool,
) -> Result<String, RenderError> {
    let mut ctx = pg_crud_ctx(pascal, snake, fields, shared_vos, &EntityShape::default());
    ctx.insert("events", &events);
    render("infrastructure/repository_pg_crud.tera", &ctx)
}

/// `pruebas/siembra.rs` of a Postgres infrastructure crate: `build` and `seed`, for its tests and
/// for those of the contexts that reference it (ADR-0023 § 5).
#[cfg(test)]
pub fn generate_pg_seed(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    events: bool,
) -> Result<String, RenderError> {
    let mut ctx = pg_crud_ctx(pascal, snake, fields, shared_vos, &EntityShape::default());
    ctx.insert("events", &events);
    render("infrastructure/siembra.tera", &ctx)
}

/// What the Pg repository and the Pg unit of work render from: the SQL of the entity's table,
/// and the seed its integration tests build.
fn pg_crud_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    shape: &EntityShape,
) -> tera::Context {
    let eff = effective_fields(fields);

    // SQL text stays a Rust computation: the `$n` numbering is index-dependent.
    let mut ctx = seed_ctx(pascal, snake, fields, shared_vos, shape);
    let groups = &shape.uniques;
    // A row's references must exist: it is seeded with a target seeded first, through the
    // target's own repository tests (transitively, for the target's own references).
    let mut field_ctxs = seeded_field_ctxs(&eff, pascal, shared_vos, groups);
    let mut seeded: Vec<String> = eff
        .iter()
        .filter(|f| seeds_its_own_value(f, groups))
        .map(|f| f.name.clone())
        .collect();
    for f in field_ctxs
        .iter_mut()
        .filter(|f| f.is_ref && !f.field_type.starts_with("Option<"))
    {
        f.seed_literal = format!(
            "infra_{}::pruebas::siembra::sembrar(pool).await.id",
            f.ref_target_snake
        );
        if !seeded.contains(&f.name) {
            seeded.push(f.name.clone());
        }
    }
    ctx.insert("fields", &field_ctxs);
    ctx.insert("seeded", &seeded);
    let table = pluralize(snake);
    let references: Vec<serde_json::Value> = eff
        .iter()
        .filter(|f| is_reference(f))
        .map(|f| {
            serde_json::json!({
                "constraint": foreign_key_name(&table, &f.name),
                "field": f.name,
                "id": reference_id(f),
                "import": crate::generators::types::reference_import(f),
                "optional": f.field_type.starts_with("Option<"),
            })
        })
        .collect();
    ctx.insert("references", &references);
    let mut reference_imports = vec![];
    push_reference_imports(&mut reference_imports, &eff);
    ctx.insert("reference_imports", &reference_imports);
    ctx.insert(
        "seeds_references",
        &eff.iter()
            .any(|f| is_reference(f) && !f.field_type.starts_with("Option<")),
    );
    // The VO the update test sets, when the field it updates is one (the seed's own imports are
    // `pruebas::siembra`'s).
    let update_vo_import = eff
        .iter()
        .find(|f| f.field_type == "String" && !is_value_object(f) && !is_reference(f))
        .is_none()
        .then(|| {
            eff.iter()
                .find(|f| is_value_object(f) && !is_enum_vo(f) && f.field_type == "String")
        })
        .flatten()
        .map(|f| vo_import_path(f, snake, shared_vos, false));
    ctx.insert("update_vo_import", &update_vo_import);
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
    shape: &EntityShape,
) -> tera::Context {
    seed_ctx_with(pascal, snake, fields, shared_vos, &[], shape)
}

/// [`seed_ctx`] with more test imports.
fn seed_ctx_with(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    more_test_imports: &[String],
    shape: &EntityShape,
) -> tera::Context {
    let eff = effective_fields(fields);
    let groups = &shape.uniques;
    let seeded: Vec<&Field> = eff
        .iter()
        .filter(|f| seeds_its_own_value(f, groups))
        .collect();

    // The seed names every plain and enum VO it constructs (`Name::new(…)`, `Status::Active`);
    // Option/Vec VOs are seeded as `None` / `vec![]` and need no import.
    let mut test_imports: Vec<String> = vec![];
    for f in seeded.iter().filter(|f| literal_names_vo(f)) {
        let import = vo_import_path(f, snake, shared_vos, false);
        if !test_imports.contains(&import) {
            test_imports.push(import);
        }
    }
    // A `Uuid` field and every `unique` seed draw on `Uuid::new_v4()`; the file itself names no
    // `Uuid` since ids are `{Entity}Id`.
    if seeded
        .iter()
        .any(|f| seed_literal(&as_seeded(f, groups)).contains("Uuid::"))
    {
        test_imports.push("uuid::Uuid".to_string());
    }
    for import in more_test_imports {
        if !test_imports.contains(import) {
            test_imports.push(import.clone());
        }
    }

    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("test_imports", &test_imports);
    ctx.insert(
        "seeded",
        &seeded.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
    );
    ctx.insert(
        "fields",
        &seeded_field_ctxs(&eff, pascal, shared_vos, groups),
    );
    insert_unique_groups(&mut ctx, &eff, pascal, shared_vos, groups);
    insert_order(&mut ctx, &eff, &shape.order);
    ctx
}

/// What `puerto.toml` says about the entity's rows beyond its fields (`[[entidad]] unicos`,
/// `orden`).
fn shape_of(base: &Path, pascal: &str) -> EntityShape {
    crate::puerto_toml::read(base)
        .ok()
        .and_then(|c| c.entity.into_iter().find(|e| e.name == pascal))
        .map(|e| EntityShape {
            uniques: e.uniques,
            order: e.order,
        })
        .unwrap_or_default()
}

/// The order of the entity's list (`[[entidad]] orden`, then the newest first): `order_by` for
/// Postgres and `order_chain`, the in-memory repository's comparison of `a` and `b`.
fn insert_order(ctx: &mut tera::Context, eff: &[Field], order: &[String]) {
    let mut sql: Vec<String> = vec![];
    let mut chain: Vec<String> = vec![];
    for key in order {
        let (name, descending) = match key.strip_prefix('-') {
            Some(name) => (name, true),
            None => (key.as_str(), false),
        };
        let Some(field) = eff.iter().find(|f| f.name == name) else {
            continue;
        };
        sql.push(if descending {
            format!("{name} DESC")
        } else {
            name.to_string()
        });
        let ascending = order_comparison(field);
        chain.push(if descending {
            format!("{ascending}.reverse()")
        } else {
            ascending
        });
    }
    sql.push("creado_en DESC, id".to_string());
    chain.push("b.creado_en.cmp(&a.creado_en)".to_string());
    chain.push("a.id.valor().cmp(&b.id.valor())".to_string());
    let (first, rest) = chain
        .split_first()
        .expect("the chain ends in creado_en and id");
    let chain = rest.iter().fold(first.clone(), |all, next| {
        format!("{all}.then_with(|| {next})")
    });
    ctx.insert("order_by", &sql.join(", "));
    ctx.insert("order_chain", &chain);
}

/// `a.{field}` compared with `b.{field}`, ascending, by the primitive it holds: a VO by its value
/// (an enum by its name), a reference by its `Uuid`, a float with `total_cmp`.
fn order_comparison(field: &Field) -> String {
    let name = &field.name;
    let inner = crate::generators::types::vo_inner_type(field);
    if is_reference(field) {
        format!("a.{name}.valor().cmp(&b.{name}.valor())")
    } else if crate::generators::types::is_enum_vo(field) {
        format!("a.{name}.as_str().cmp(b.{name}.as_str())")
    } else if is_value_object(field) && inner == "String" {
        format!("a.{name}.valor().cmp(b.{name}.valor())")
    } else if is_value_object(field) && inner == "f64" {
        format!("a.{name}.valor().total_cmp(&b.{name}.valor())")
    } else if is_value_object(field) {
        format!("a.{name}.valor().cmp(&b.{name}.valor())")
    } else if field.field_type == "f64" {
        format!("a.{name}.total_cmp(&b.{name})")
    } else {
        format!("a.{name}.cmp(&b.{name})")
    }
}

/// What the repositories and their tests say about the uniques: `unique_groups` (one test each,
/// `deberia_rechazar_un_duplicado_de_<a>_y_<b>`) and `unique_conditions`, the in-memory
/// repository's check — a single unique field, or every field of a group.
fn insert_unique_groups(
    ctx: &mut tera::Context,
    eff: &[Field],
    pascal: &str,
    shared_vos: &[ValueObjectDefinition],
    groups: &[Vec<String>],
) {
    let field_of = |name: &String| {
        eff.iter()
            .find(|f| &f.name == name)
            .map(|f| field_ctx(f, pascal, shared_vos))
    };
    let unique_groups: Vec<serde_json::Value> = groups
        .iter()
        .map(|group| {
            let fields: Vec<FieldCtx> = group.iter().filter_map(field_of).collect();
            serde_json::json!({
                "suffix": group.join("_y_"),
                "first": group.first(),
                "fields": fields,
            })
        })
        .collect();
    let mut conditions: Vec<String> = eff
        .iter()
        .filter(|f| f.unique)
        .map(|f| format!("fila.{0} == entidad.{0}", f.name))
        .collect();
    conditions.extend(groups.iter().map(|group| {
        let all = group
            .iter()
            .map(|name| format!("fila.{name} == entidad.{name}"))
            .collect::<Vec<_>>()
            .join(" && ");
        format!("({all})")
    }));
    ctx.insert("unique_groups", &unique_groups);
    ctx.insert("unique_conditions", &conditions);
}

/// A field of a unique group is seeded like a unique one: a fresh value per row.
fn as_seeded(field: &Field, groups: &[Vec<String>]) -> Field {
    Field {
        unique: field.unique || groups.iter().flatten().any(|name| name == &field.name),
        ..field.clone()
    }
}

/// The field contexts a seed renders: a field of a unique group seeded as a unique one.
fn seeded_field_ctxs(
    eff: &[Field],
    pascal: &str,
    shared_vos: &[ValueObjectDefinition],
    groups: &[Vec<String>],
) -> Vec<FieldCtx> {
    eff.iter()
        .map(|f| FieldCtx {
            seed_literal: seed_literal(&as_seeded(f, groups)),
            ..field_ctx(f, pascal, shared_vos)
        })
        .collect()
}

/// A seeded entity takes the mother's values (Phase 20.5) but where one would not do for a stored
/// row: a unique field (or one of a unique group) gets a fresh value per row, a timestamp a whole
/// second Postgres stores exactly (see [`seed_literal`]).
fn seeds_its_own_value(field: &Field, groups: &[Vec<String>]) -> bool {
    let seeded = as_seeded(field, groups);
    seeded.unique || seed_literal(&seeded).contains("from_timestamp")
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

#[cfg(test)]
pub fn create_table_sql(snake: &str, fields: &[Field]) -> String {
    create_table_sql_with(snake, fields, &[])
}

/// The `CREATE TABLE` of an entity, with its indexes: one per unique field, one per unique group
/// (`[[entidad]] unicos`), one per reference.
pub fn create_table_sql_with(snake: &str, fields: &[Field], groups: &[Vec<String>]) -> String {
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
        "CREATE TABLE {table} (\n    id UUID PRIMARY KEY,\n{custom_cols}\n    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    actualizado_en TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    borrado BOOLEAN NOT NULL DEFAULT FALSE,\n    borrado_en TIMESTAMPTZ\n);\n",
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
                "\nCREATE UNIQUE INDEX {table}_{name}_key ON {table} ({name}) WHERE borrado = false;\n"
            )
        })
        .collect();
    sql.push_str(&unique_indexes);
    for group in groups {
        sql.push_str(&format!(
            "\nCREATE UNIQUE INDEX {table}_{}_key ON {table} ({}) WHERE borrado = false;\n",
            group.join("_"),
            group.join(", ")
        ));
    }

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

/// The context's infrastructure crate: the in-memory repository, or the Postgres row, repository,
/// unit of work (`events` projects) and test seeds — then its `Cargo.toml` and `lib.rs`.
pub fn write_repository_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let events = crate::puerto_toml::events_enabled(base);
    let shape = shape_of(base, pascal);
    if db {
        write_file(
            &base.join(layout::infra_file(snake, "fila")),
            &generate_infra_entity(pascal, snake, fields, shared_vos)?,
        )?;
        let mut ctx = pg_crud_ctx(pascal, snake, fields, shared_vos, &shape);
        ctx.insert("events", &events);
        write_file(
            &base.join(layout::infra_file(snake, "repositorio")),
            &render("infrastructure/repository_pg_crud.tera", &ctx)?,
        )?;
        if events {
            write_file(
                &base.join(layout::infra_file(snake, "unit_of_work")),
                &render("infrastructure/unit_of_work_pg.tera", &ctx)?,
            )?;
        }
        write_file(
            &base.join(layout::infra_file(snake, "pruebas/siembra")),
            &render("infrastructure/siembra.tera", &ctx)?,
        )?;
    } else {
        write_file(
            &base.join(layout::infra_file(snake, "repositorio")),
            &render(
                "infrastructure/repository_inmemory_crud.tera",
                // A reference is seeded, and filtered by, `CustomerId::generate()`: nothing
                // checks it.
                &seed_ctx_with(
                    pascal,
                    snake,
                    fields,
                    shared_vos,
                    &{
                        let mut imports = vec![];
                        push_reference_imports(&mut imports, &effective_fields(fields));
                        imports
                    },
                    &shape,
                ),
            )?,
        )?;
    }
    crate::generators::crates::finish(
        base,
        snake,
        Layer::Infrastructure,
        &ContextDeps::read(base, fields),
    )
}

pub fn run_generate_repository(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generar dominio {pascal}` first."
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

    if db {
        run_create_table_migration(
            &snake,
            base,
            &create_table_sql_with(&snake, &fields, &shape_of(base, &pascal).uniques),
        )?;
    }

    let repo_label = if db {
        format!("Repositorio{pascal}Postgres")
    } else {
        format!("Repositorio{pascal}EnMemoria")
    };
    println!(
        "✓ {}/ — {repo_label}",
        layout::crate_dir(&snake, Layer::Infrastructure)
    );
    println!();
    println!("  Next: puerto generar presentacion {pascal}");
    Ok(())
}
