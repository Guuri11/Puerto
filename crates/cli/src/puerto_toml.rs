use std::path::Path;

use serde::{Deserialize, Serialize};

// ── Config types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct PuertoConfig {
    pub project: Project,
    #[serde(default)]
    pub entity: Vec<Entity>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity {
    /// PascalCase entity name, e.g. "Product"
    pub name: String,
    /// snake_case use case action names, e.g. ["create_product"]
    pub use_cases: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Field {
    /// snake_case field name, e.g. "name"
    pub name: String,
    /// Rust type string, e.g. "String", "i64", "Option<String>"
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unique: bool,
}

/// Parse a field argument in "name:Type" format.
/// Supports optional "unique" suffix: "sku:String!" marks the field as unique.
pub fn parse_field_arg(s: &str) -> Result<Field, String> {
    let (raw, unique) = if let Some(stripped) = s.strip_suffix('!') {
        (stripped, true)
    } else {
        (s, false)
    };
    let (name, field_type) = raw.split_once(':').ok_or_else(|| {
        format!(
            "invalid field argument '{}'. Expected format: name:Type (e.g. title:String)",
            s
        )
    })?;
    let name = name.to_string();
    if !is_valid_field_name(&name) {
        return Err(format!(
            "invalid field name '{}'. Must be snake_case (lowercase letters, digits, underscores, not starting with a digit)",
            name
        ));
    }
    let field_type = field_type.to_string();
    if field_type.is_empty() {
        return Err(format!(
            "invalid field argument '{}'. Type cannot be empty",
            s
        ));
    }
    Ok(Field {
        name,
        field_type,
        unique,
    })
}

fn is_valid_field_name(name: &str) -> bool {
    is_valid_field_name_pub(name)
}

pub fn is_valid_field_name_pub(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

// ── I/O helpers ───────────────────────────────────────────────────────────────

pub fn read(base: &Path) -> Result<PuertoConfig, Box<dyn std::error::Error>> {
    let path = base.join("puerto.toml");
    let src = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&src)?)
}

pub fn write(base: &Path, config: &PuertoConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("puerto.toml");
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

/// Append a new entity to puerto.toml. No-op if entity already present.
pub fn add_entity(
    base: &Path,
    name: &str,
    use_cases: Vec<String>,
    db: bool,
    fields: Vec<Field>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    if config.entity.iter().any(|e| e.name == name) {
        return Ok(());
    }
    config.entity.push(Entity {
        name: name.to_string(),
        use_cases,
        db,
        fields,
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
        .ok_or_else(|| format!("entity '{entity_name}' not found in puerto.toml"))?;
    if !entity.use_cases.contains(&action.to_string()) {
        entity.use_cases.push(action.to_string());
        write(base, &config)?;
    }
    Ok(())
}
