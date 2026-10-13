use std::path::Path;

use crate::generators::conflict::{OverwritePolicy, guard_entity_overwrite};
use crate::generators::naming::entity_ctx;
use crate::generators::types::{is_shared_vo, is_value_object};
use crate::generators::{
    application::{generate_create_use_case_impl, write_application_files},
    domain::{generate_create_use_case_trait, generate_model, write_domain_files, write_mother},
    infrastructure::{generate_infra_entity, write_repository_files},
    migration::run_create_table_migration,
    naming::{crud_use_cases, pascal_to_snake, to_pascal_case, write_file},
    presentation::write_presentation_files,
    render::render,
    web::write_web_files,
};
use crate::patchers::lib_rs::{
    patch_business_lib_shared, patch_business_lib_value_objects, patch_libs, patch_mothers_lib,
    unpatch_business_lib_value_objects,
};
use crate::puerto_toml::{Field, ValueObjectDefinition};

// ── Single use case (non-CRUD) ────────────────────────────────────────────────

fn write_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &generate_model(pascal, snake, &[], &[])?,
    )?;
    let mut errors_ctx = entity_ctx(pascal, snake);
    errors_ctx.insert("vo_fields", &Vec::<String>::new());
    write_file(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &render("domain/errors.tera", &errors_ctx)?,
    )?;
    {
        let repo_ctx = entity_ctx(pascal, snake);
        write_file(
            &base.join(format!("business/src/domain/{snake}/repository.rs")),
            &render("domain/repository_simple.tera", &repo_ctx)?,
        )?;
    }
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &generate_create_use_case_trait(pascal, snake, &[])?,
    )?;
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/create_{snake}.rs"
        )),
        &generate_create_use_case_impl(pascal, snake, &[], &[])?,
    )?;
    let infra_ctx = entity_ctx(pascal, snake);
    if db {
        write_file(
            &base.join(format!("infrastructure/src/{snake}/entity.rs")),
            &generate_infra_entity(pascal, snake, &[], &[])?,
        )?;
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &render("infrastructure/repository_pg_simple.tera", &infra_ctx)?,
        )?;
    } else {
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &render("infrastructure/repository_inmemory_simple.tera", &infra_ctx)?,
        )?;
    }
    write_file(
        &base.join(format!("presentation/src/api/{snake}.rs")),
        "pub mod dto;\npub mod error_mapper;\npub mod responses;\npub mod routes;\n",
    )?;
    {
        use crate::generators::presentation::{
            build_dto_ctx, build_error_mapper_ctx, build_simple_ctx,
        };
        write_file(
            &base.join(format!("presentation/src/api/{snake}/dto.rs")),
            &render(
                "presentation/dto.tera",
                &build_dto_ctx(pascal, snake, &[], false),
            )?,
        )?;
        write_file(
            &base.join(format!("presentation/src/api/{snake}/responses.rs")),
            &render(
                "presentation/responses_simple.tera",
                &build_simple_ctx(pascal, snake),
            )?,
        )?;
        write_file(
            &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
            &render(
                "presentation/error_mapper.tera",
                &build_error_mapper_ctx(pascal, snake, &[]),
            )?,
        )?;
        write_file(
            &base.join(format!("presentation/src/api/{snake}/routes.rs")),
            &render(
                "presentation/routes_simple.tera",
                &build_simple_ctx(pascal, snake),
            )?,
        )?;
    }
    Ok(())
}

// ── CRUD (delegates to per-layer writers) ────────────────────────────────────

fn write_files_crud(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    write_domain_files(pascal, snake, base, fields, shared_vos)?;
    write_application_files(pascal, snake, base, fields, shared_vos)?;
    write_repository_files(pascal, snake, base, db, fields, shared_vos)?;
    write_presentation_files(pascal, snake, base, fields)?;
    Ok(())
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Low-level scaffold: write files + patch lib.rs files.
/// `crud = true` generates 5 use cases; `crud = false` generates only create.
pub fn run(
    name: &str,
    base: &Path,
    db: bool,
    crud: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);
    let web_project = crate::puerto_toml::read(base).is_ok_and(|c| c.project.web);
    let web_slice = web_project && crud;

    if crud {
        write_files_crud(&pascal, &snake, base, db, fields, shared_vos)?;
    } else {
        write_files(&pascal, &snake, base, db)?;
    }
    if web_slice {
        write_web_files(&pascal, base, fields)?;
    }
    patch_libs(&snake, base, db, crud)?;

    if !fields.is_empty()
        && fields
            .iter()
            .any(|f| is_value_object(f) && !is_shared_vo(f, shared_vos))
    {
        patch_business_lib_value_objects(base, &snake)?;
    } else {
        unpatch_business_lib_value_objects(base, &snake)?;
    }

    if !shared_vos.is_empty() {
        use crate::generators::domain::write_shared_vo_files;
        write_shared_vo_files(base, shared_vos)?;
        patch_business_lib_shared(base)?;
    }

    write_mother(&pascal, &snake, base, fields, shared_vos)?;
    patch_mothers_lib(base, &snake)?;

    let use_cases = if crud {
        crud_use_cases(&snake)
    } else {
        vec![format!("create_{snake}")]
    };

    crate::puerto_toml::add_entity(base, &pascal, use_cases, db, fields.to_vec())?;
    crate::patchers::routes_rs::patch_routes_from_manifest(base, &pascal, &snake)?;
    crate::generators::bootstrap::regenerate_bootstrap(base)?;

    let repo_name = if db {
        format!("Pg{pascal}Repository")
    } else {
        format!("InMemory{pascal}Repository")
    };
    let uc_label = if crud {
        "create, get, list, update, delete"
    } else {
        "create"
    };
    println!("✓ business/        — model, errors, repository trait, use cases ({uc_label})");
    println!("✓ infrastructure/  — {repo_name}");
    println!("✓ presentation/    — routes, dto, responses, error mapper");
    if web_slice {
        println!("✓ web/             — list, detail and form screens, HTTP repository");
    }
    println!("✓ Done. Zero manual wiring.");
    if web_project {
        println!(
            "\nThe API changed. Next: make web/sync   (exports the spec, regenerates the web types)"
        );
    }

    Ok(())
}

/// `--public` and `--role` of `puerto generate scaffold` in an auth project (ADR-0021 § 4): the
/// CRUD use cases whose routes take no `BearerAuth`, and the roles a route requires.
#[derive(Debug, Default, Clone)]
pub struct ScaffoldAccess {
    pub public: Vec<String>,
    pub roles: std::collections::BTreeMap<String, Vec<String>>,
}

impl ScaffoldAccess {
    /// `--public list_products,get_product` and `--role create_product=admin,…` as the CLI got them.
    pub fn from_args(public: &[String], roles: &[String]) -> Result<Self, String> {
        let mut access = Self {
            public: public.to_vec(),
            ..Default::default()
        };
        for arg in roles {
            let (use_case, role) = arg.split_once('=').ok_or_else(|| {
                format!(
                    "invalid --role '{arg}'. Expected <use_case>=<role>, e.g. delete_product=admin"
                )
            })?;
            let roles = access.roles.entry(use_case.to_string()).or_default();
            if !roles.iter().any(|r| r == role) {
                roles.push(role.to_string());
            }
        }
        Ok(access)
    }

    fn is_empty(&self) -> bool {
        self.public.is_empty() && self.roles.is_empty()
    }
}

/// Checks `access` against the entity's CRUD use cases and writes it to its `[[entity]]` block,
/// before any file is rendered: the generators read it back from there. A re-scaffold without
/// flags keeps what the block says.
fn record_access(
    base: &Path,
    pascal: &str,
    db: bool,
    fields: &[Field],
    access: &ScaffoldAccess,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = crate::puerto_toml::read(base)?;
    if !config.project.has_auth() {
        if !access.is_empty() {
            return Err(
                "--public and --role need an auth project (`puerto new --auth oidc`)".into(),
            );
        }
        return Ok(());
    }
    let snake = pascal_to_snake(pascal);
    let checked = crate::puerto_toml::Entity {
        name: pascal.to_string(),
        use_cases: crud_use_cases(&snake),
        public: access.public.clone(),
        roles: access.roles.clone(),
        ..Default::default()
    };
    let errors = crate::validation::validate_entity_access(&checked);
    if !errors.is_empty() {
        return Err(format!("{pascal}: {}", errors.join("\n  ")).into());
    }
    match config.entity.iter_mut().find(|e| e.name == pascal) {
        // A re-scaffold: the routes are rewritten as the block already says.
        Some(_) if access.is_empty() => return Ok(()),
        Some(entity) => {
            entity.public = checked.public;
            entity.roles = checked.roles;
        }
        None => config.entity.push(crate::puerto_toml::Entity {
            db,
            fields: fields.to_vec(),
            ..checked
        }),
    }
    crate::puerto_toml::write(base, &config)
}

/// `puerto generate scaffold <Name>` without `--public` / `--role`.
/// Reads `project.db` from `puerto.toml`, always generates full CRUD.
/// `cli_fields` are fields passed from the CLI (e.g. `title:String price:i64`).
///
/// Refuses to overwrite an already-generated entity unless `--force`; see `conflict.rs`.
#[cfg(test)]
pub fn run_scaffold(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
    overwrite: OverwritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    run_scaffold_with_access(
        name,
        base,
        cli_fields,
        overwrite,
        &ScaffoldAccess::default(),
    )
}

/// [`run_scaffold`] with `--public` / `--role`. In an auth project every CRUD route takes
/// `BearerAuth` except the `public` ones; outside one, `access` must be empty.
pub fn run_scaffold_with_access(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
    overwrite: OverwritePolicy,
    access: &ScaffoldAccess,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let db = config.project.db;
    let shared_vos = config.value_object.clone();
    let pascal = to_pascal_case(name);
    let fields = if !cli_fields.is_empty() {
        cli_fields.to_vec()
    } else {
        config
            .entity
            .iter()
            .find(|e| e.name == pascal)
            .map(|e| e.fields.clone())
            .unwrap_or_default()
    };

    // Checked before the first byte is written — a half-overwritten project is worse than a
    // refused command.
    crate::validation::validate_entity_name(&pascal)?;
    if config.project.has_auth() {
        crate::validation::validate_auth_entity_name(&pascal)?;
    }
    crate::validation::validate_unique_field_names(&fields)?;
    if db {
        for f in &fields {
            crate::validation::validate_sql_field_name(&f.name)?;
        }
    }
    let vo_conflicts = crate::validation::validate_vo_coherence(&fields, &shared_vos);
    if !vo_conflicts.is_empty() {
        return Err(vo_conflicts.join("\n  ").into());
    }
    crate::validation::validate_references_before_writing(&pascal, &fields, &config, base)?;
    guard_entity_overwrite(
        base,
        &pascal_to_snake(&pascal),
        db,
        config.project.web,
        overwrite,
    )?;
    record_access(base, &pascal, db, &fields, access)?;

    run(name, base, db, true, &fields, &shared_vos)?;
    if db {
        let snake = pascal_to_snake(&pascal);
        let migration_sql = crate::generators::infrastructure::create_table_sql(&snake, &fields);
        run_create_table_migration(&snake, base, &migration_sql)?;
    }
    Ok(())
}

/// CLI entry point for `puerto generate value-object <Name> <type>`.
///
/// Validated here, at the command that writes it: an invalid declaration used to land in
/// puerto.toml and only surface later, in `puerto validate` or as uncompilable generated code.
pub fn run_value_object(
    name: &str,
    inner_type: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let inner_type = &crate::puerto_toml::expand_type(inner_type);
    crate::validation::validate_vo_name(name)?;
    crate::validation::validate_shared_vo_inner_type(inner_type)?;

    let config = crate::puerto_toml::read(base)?;
    if config.value_object.iter().any(|vo| vo.name == name) {
        return Err(
            format!("shared value object '{name}' is already declared in puerto.toml").into(),
        );
    }
    crate::puerto_toml::add_value_object(base, name, inner_type)?;
    println!("✓ Shared value object '{name}' added to puerto.toml");
    Ok(())
}
