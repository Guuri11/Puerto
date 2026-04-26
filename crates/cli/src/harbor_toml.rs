use std::path::Path;

use serde::{Deserialize, Serialize};

// ── Config types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct HarborConfig {
    pub project: Project,
    #[serde(default)]
    pub entity: Vec<Entity>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    /// true when the project was created with `harbor new --db`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity {
    /// PascalCase entity name, e.g. "Product"
    pub name: String,
    /// snake_case use case action names, e.g. ["create_product"]
    pub use_cases: Vec<String>,
    /// true when this entity uses a SQLx (Postgres) repository instead of InMemory
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
}

// ── I/O helpers ───────────────────────────────────────────────────────────────

pub fn read(base: &Path) -> Result<HarborConfig, Box<dyn std::error::Error>> {
    let path = base.join("harbor.toml");
    let src = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&src)?)
}

pub fn write(base: &Path, config: &HarborConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("harbor.toml");
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

/// Append a new entity to harbor.toml. No-op if entity already present.
pub fn add_entity(
    base: &Path,
    name: &str,
    use_cases: Vec<String>,
    db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    if config.entity.iter().any(|e| e.name == name) {
        return Ok(());
    }
    config.entity.push(Entity {
        name: name.to_string(),
        use_cases,
        db,
    });
    write(base, &config)
}

/// Append a use case action to an existing entity. Errors if entity not found. No-op if action already present.
pub fn add_use_case(
    base: &Path,
    entity_name: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    let entity = config
        .entity
        .iter_mut()
        .find(|e| e.name == entity_name)
        .ok_or_else(|| format!("entity '{entity_name}' not found in harbor.toml"))?;
    if !entity.use_cases.contains(&action.to_string()) {
        entity.use_cases.push(action.to_string());
        write(base, &config)?;
    }
    Ok(())
}
