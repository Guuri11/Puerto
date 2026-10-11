use std::{fs, path::Path};

use serde::Serialize;

use crate::generators::naming::{pascal_to_snake, to_pascal_case};
use crate::generators::render::{RenderError, render};

#[derive(Serialize)]
struct BootstrapUseCase {
    action: String,
    uc_pascal: String,
}

#[derive(Serialize)]
struct BootstrapEntity {
    pascal: String,
    snake: String,
    db: bool,
    use_cases: Vec<BootstrapUseCase>,
}

fn build_context(entities: &[crate::puerto_toml::Entity], web: bool) -> tera::Context {
    let ctx_entities: Vec<BootstrapEntity> = entities
        .iter()
        .map(|entity| BootstrapEntity {
            pascal: entity.name.clone(),
            snake: pascal_to_snake(&entity.name),
            db: entity.db,
            use_cases: entity
                .use_cases
                .iter()
                .map(|uc| BootstrapUseCase {
                    action: uc.clone(),
                    uc_pascal: to_pascal_case(uc),
                })
                .collect(),
        })
        .collect();

    let mut ctx = tera::Context::new();
    ctx.insert("entities", &ctx_entities);
    ctx.insert("has_db", &entities.iter().any(|e| e.db));
    ctx.insert("web", &web);
    ctx.insert("apis", &api_expression(entities));
    ctx
}

/// What `OpenApiService` serves: `()` with no entity, the one `{snake}_api`, or a tuple of them.
fn api_expression(entities: &[crate::puerto_toml::Entity]) -> String {
    let apis: Vec<String> = entities
        .iter()
        .map(|e| format!("{}_api", pascal_to_snake(&e.name)))
        .collect();
    match apis.as_slice() {
        [single] => single.clone(),
        _ => format!("({})", apis.join(", ")),
    }
}

/// `web`: the project has a frontend, so `bootstrap.rs` also carries the test that exports the
/// OpenAPI spec to `docs/api/openapi.json` (ADR-0013).
pub fn generate_bootstrap_content(
    entities: &[crate::puerto_toml::Entity],
    web: bool,
) -> Result<String, RenderError> {
    let ctx = build_context(entities, web);
    render("bootstrap/bootstrap.tera", &ctx)
}

pub fn generate_tags_content(
    entities: &[crate::puerto_toml::Entity],
) -> Result<String, RenderError> {
    let tags: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
    let mut ctx = tera::Context::new();
    ctx.insert("tags_entities", &tags);
    render("bootstrap/tags.tera", &ctx)
}

/// `puerto generate bootstrap` — refuses when the manifest describes code that is not on disk.
///
/// The write itself is unconditional in `regenerate_bootstrap()` (scaffold calls it right after
/// writing the very files in question). The refusal belongs to the standalone command, which is
/// the one a user runs against a manifest they hand-edited.
pub fn run_bootstrap_command(
    base: &Path,
    allow_missing: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !allow_missing {
        let config = crate::puerto_toml::read(base)?;
        let drifts = crate::generators::consistency::missing_entity_files(base, &config);
        if !drifts.is_empty() {
            let lines = crate::generators::consistency::drift_lines(&drifts);
            return Err(format!(
                "refusing to write a bootstrap.rs that will not compile:\n  {}\n\nPass \
                 --allow-missing to write it anyway.",
                lines.join("\n  ")
            )
            .into());
        }
    }
    regenerate_bootstrap(base)?;
    println!("✓ bootstrap.rs regenerated.");
    Ok(())
}

/// Read puerto.toml and overwrite `presentation/src/generated/bootstrap.rs`
/// and `presentation/src/api/tags.rs`.
///
/// Warns — rather than refuses — on manifest/filesystem drift: this runs inside `scaffold`, where
/// the entity being generated is fine and it is some *other* declared entity that is missing.
pub fn regenerate_bootstrap(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;

    for line in crate::generators::consistency::drift_lines(
        &crate::generators::consistency::missing_entity_files(base, &config),
    ) {
        eprintln!("⚠ {line}");
    }

    let bootstrap_path = base.join("presentation/src/generated/bootstrap.rs");
    if let Some(parent) = bootstrap_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        bootstrap_path,
        generate_bootstrap_content(&config.entity, config.project.web)?,
    )?;

    let tags_path = base.join("presentation/src/api/tags.rs");
    if let Some(parent) = tags_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(tags_path, generate_tags_content(&config.entity)?)?;

    Ok(())
}
