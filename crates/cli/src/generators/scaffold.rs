use crate::generators::layout;
use std::path::Path;

use crate::generators::conflict::{OverwritePolicy, guard_entity_overwrite};
use crate::generators::{
    application::write_application_files,
    domain::write_domain_files,
    infrastructure::write_repository_files,
    migration::run_create_table_migration,
    naming::{crud_use_cases, pascal_to_snake, to_pascal_case},
    presentation::write_presentation_files,
    web::write_web_files,
};
use crate::puerto_toml::{Field, ValueObjectDefinition};

/// Low-level scaffold: the four crates of the entity's context (ADR-0023), its web slice, its
/// `[[entidad]]` block and the regenerated wiring. Always the five CRUD use cases.
pub fn run(
    name: &str,
    base: &Path,
    db: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);
    let web = crate::puerto_toml::read(base).is_ok_and(|c| c.project.web);

    write_domain_files(&pascal, &snake, base, fields, shared_vos)?;
    write_application_files(&pascal, &snake, base, fields, shared_vos)?;
    write_repository_files(&pascal, &snake, base, db, fields, shared_vos)?;
    write_presentation_files(&pascal, &snake, base, fields)?;
    if web {
        write_web_files(&pascal, base, fields)?;
    }

    crate::puerto_toml::add_entity(base, &pascal, crud_use_cases(&snake), db, fields.to_vec())?;
    crate::patchers::routes_rs::patch_routes_from_manifest(base, &pascal, &snake)?;
    crate::generators::bootstrap::regenerate_bootstrap(base)?;

    let repo_name = if db {
        format!("Repositorio{pascal}Postgres")
    } else {
        format!("Repositorio{pascal}EnMemoria")
    };
    let dir = layout::context_dir(&snake);
    println!("✓ {dir}/dominio/         modelo, errores, puerto del repositorio, Madre{pascal}");
    println!("✓ {dir}/aplicacion/      crear, obtener, listar, actualizar, borrar");
    println!("✓ {dir}/infraestructura/ {repo_name}");
    println!("✓ {dir}/rest/            rutas, dto, respuestas, errores");
    if web {
        let width = format!("{dir}/infraestructura/").len();
        println!(
            "✓ {:<width$} pantallas de lista, detalle y formulario",
            "web/"
        );
    }
    println!("✓ Hecho. Sin cablear nada a mano.");
    if web {
        println!(
            "\nLa API cambió. Siguiente: make web/sync   (exporta la especificación y regenera los \
             tipos de la web)"
        );
    }

    Ok(())
}

/// `--publico` and `--rol` of `puerto generar scaffold` in an auth project (ADR-0021 § 4): the
/// CRUD use cases whose routes take no `BearerAuth`, and the roles a route requires.
#[derive(Debug, Default, Clone)]
pub struct ScaffoldAccess {
    pub public: Vec<String>,
    pub roles: std::collections::BTreeMap<String, Vec<String>>,
}

impl ScaffoldAccess {
    /// `--publico list_products,get_product` and `--rol create_product=admin,…` as the CLI got them.
    pub fn from_args(public: &[String], roles: &[String]) -> Result<Self, String> {
        let mut access = Self {
            public: public.to_vec(),
            ..Default::default()
        };
        for arg in roles {
            let (use_case, role) = arg.split_once('=').ok_or_else(|| {
                format!(
                    "--rol inválido '{arg}'. El formato es <caso_de_uso>=<rol>, p. ej. \
                     borrar_producto=admin"
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

/// What a scaffold says about the entity beyond its field list: its uniques over several fields
/// (`--unico a,b`, repeatable), the order of its list (`--orden=-a,b`) and its private fields
/// (`--privado a,b`, auth projects). A re-scaffold without one keeps what the block says.
#[derive(Default)]
pub struct EntityShape {
    pub uniques: Vec<Vec<String>>,
    pub order: Vec<String>,
    pub private: Vec<String>,
}

impl EntityShape {
    /// `--unico jugador_id,partido_id` (one group each), `--orden` and `--privado` as the CLI got
    /// them.
    pub fn from_args(uniques: &[String], order: &[String], private: &[String]) -> Self {
        let trimmed = |args: &[String]| args.iter().map(|a| a.trim().to_string()).collect();
        Self {
            uniques: uniques
                .iter()
                .map(|group| group.split(',').map(|f| f.trim().to_string()).collect())
                .collect(),
            order: trimmed(order),
            private: trimmed(private),
        }
    }

    fn is_empty(&self) -> bool {
        self.uniques.is_empty() && self.order.is_empty()
    }

    /// `fields` with `--privado` applied: the named ones private. Without `--privado`, a field keeps
    /// what the block (`declared`) said of it — a re-scaffold that passes its fields again. A name
    /// that is no field is refused, before anything is written.
    pub(crate) fn mark_private(
        &self,
        pascal: &str,
        fields: Vec<Field>,
        declared: &[Field],
    ) -> Result<Vec<Field>, String> {
        if let Some(unknown) = self
            .private
            .iter()
            .find(|name| !fields.iter().any(|f| &&f.name == name))
        {
            return Err(format!(
                "{pascal}: privado: '{unknown}' no es un campo de {pascal}"
            ));
        }
        Ok(fields
            .into_iter()
            .map(|f| Field {
                private: if self.private.is_empty() {
                    f.private || declared.iter().any(|d| d.name == f.name && d.private)
                } else {
                    self.private.contains(&f.name)
                },
                ..f
            })
            .collect())
    }
}

/// `shape` checked against the entity's fields, before anything is written.
pub(crate) fn check_shape(
    pascal: &str,
    fields: &[Field],
    shape: &EntityShape,
) -> Result<(), Box<dyn std::error::Error>> {
    let errors = crate::validation::validate_entity_shape(&crate::puerto_toml::Entity {
        name: pascal.to_string(),
        fields: fields.to_vec(),
        uniques: shape.uniques.clone(),
        order: shape.order.clone(),
        ..Default::default()
    });
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{pascal}: {}", errors.join("\n  ")).into())
    }
}

/// Writes a checked `shape` to the entity's `[[entidad]]` block (creating the block) before any
/// file is rendered: the generators read it back from there.
pub(crate) fn record_shape(
    base: &Path,
    pascal: &str,
    db: bool,
    fields: &[Field],
    shape: &EntityShape,
) -> Result<(), Box<dyn std::error::Error>> {
    if shape.is_empty() {
        return Ok(());
    }
    let mut config = crate::puerto_toml::read(base)?;
    match config.entity.iter_mut().find(|e| e.name == pascal) {
        // What is not given stays as the block says.
        Some(entity) => {
            if !shape.uniques.is_empty() {
                entity.uniques = shape.uniques.clone();
            }
            if !shape.order.is_empty() {
                entity.order = shape.order.clone();
            }
        }
        None => config.entity.push(crate::puerto_toml::Entity {
            name: pascal.to_string(),
            use_cases: crud_use_cases(&pascal_to_snake(pascal)),
            db,
            fields: fields.to_vec(),
            uniques: shape.uniques.clone(),
            order: shape.order.clone(),
            ..Default::default()
        }),
    }
    crate::puerto_toml::write(base, &config)
}

/// Checks `access` against the entity's CRUD use cases and writes it to its `[[entidad]]` block,
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
                "--publico y --rol necesitan un proyecto con auth (`puerto nuevo --auth oidc`)"
                    .into(),
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

/// `puerto generar scaffold <Name>` without `--publico` / `--rol`.
/// Reads `project.db` from `puerto.toml`, always generates full CRUD.
/// `cli_fields` are fields passed from the CLI (e.g. `title:String price:i64`).
///
/// Refuses to overwrite an already-generated entity unless `--forzar`; see `conflict.rs`.
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

/// [`run_scaffold`] with `--publico` / `--rol`. In an auth project every CRUD route takes
/// `BearerAuth` except the `public` ones; outside one, `access` must be empty.
#[cfg(test)]
pub fn run_scaffold_with_access(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
    overwrite: OverwritePolicy,
    access: &ScaffoldAccess,
) -> Result<(), Box<dyn std::error::Error>> {
    run_scaffold_with_options(
        name,
        base,
        cli_fields,
        overwrite,
        access,
        &EntityShape::default(),
    )
}

/// [`run_scaffold_with_access`] with `--unico`.
pub fn run_scaffold_with_options(
    name: &str,
    base: &Path,
    cli_fields: &[Field],
    overwrite: OverwritePolicy,
    access: &ScaffoldAccess,
    shape: &EntityShape,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let db = config.project.db;
    let shared_vos = config.value_object.clone();
    let pascal = to_pascal_case(name);
    let declared = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();
    let fields = if !cli_fields.is_empty() {
        cli_fields.to_vec()
    } else {
        declared.clone()
    };
    let fields = shape.mark_private(&pascal, fields, &declared)?;
    if !config.project.has_auth() && fields.iter().any(|f| f.private) {
        return Err(
            "--privado necesita un proyecto con auth (`puerto nuevo --auth oidc`): sin sesión, \
             nadie vería nunca el campo"
                .into(),
        );
    }

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
    // Checked before the access is written: a refused shape leaves nothing behind.
    check_shape(&pascal, &fields, shape)?;
    record_access(base, &pascal, db, &fields, access)?;
    record_shape(base, &pascal, db, &fields, shape)?;

    run(name, base, db, &fields, &shared_vos)?;
    if db {
        let snake = pascal_to_snake(&pascal);
        let groups = crate::puerto_toml::read(base)?
            .entity
            .into_iter()
            .find(|e| e.name == pascal)
            .map(|e| e.uniques)
            .unwrap_or_default();
        let migration_sql =
            crate::generators::infrastructure::create_table_sql_with(&snake, &fields, &groups);
        run_create_table_migration(&snake, base, &migration_sql)?;
    }
    Ok(())
}

/// CLI entry point for `puerto generar objeto-valor <Name> <type>`.
///
/// Validated here, at the command that writes it: an invalid declaration used to land in
/// puerto.toml and only surface later, in `puerto validar` or as uncompilable generated code.
/// `example` (27.15, `--ejemplo`): the value its fields' tests use, checked against `inner_type`.
pub fn run_value_object(
    name: &str,
    inner_type: &str,
    example: Option<&str>,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let inner_type = &crate::puerto_toml::expand_type(inner_type);
    crate::validation::validate_vo_name(name)?;
    crate::validation::validate_shared_vo_inner_type(inner_type)?;
    if let Some(example) = example {
        crate::validation::validate_vo_example(inner_type, example)?;
    }

    let config = crate::puerto_toml::read(base)?;
    if config.value_object.iter().any(|vo| vo.name == name) {
        return Err(format!(
            "el objeto valor compartido '{name}' ya está declarado en puerto.toml"
        )
        .into());
    }
    crate::puerto_toml::add_value_object(base, name, inner_type)?;
    if let Some(example) = example {
        let mut config = crate::puerto_toml::read(base)?;
        if let Some(vo) = config.value_object.iter_mut().find(|vo| vo.name == name) {
            vo.example = Some(example.to_string());
        }
        crate::puerto_toml::write(base, &config)?;
    }
    println!("✓ Objeto valor compartido '{name}' añadido a puerto.toml");
    Ok(())
}
