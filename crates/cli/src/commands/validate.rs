use std::path::Path;

use crate::generators::types::resolve_type;
use crate::puerto_toml;

fn is_valid_entity_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

fn is_valid_use_case_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn run_validate(cwd: &Path) -> Result<(), Box<dyn std::error::Error>> {
    crate::commands::list::require_puerto_project(cwd)?;
    let config = puerto_toml::read(cwd)?;

    let mut errors: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    if config.project.name.is_empty() {
        errors.push("project.name is empty".into());
    }

    let mut seen_entities: Vec<String> = vec![];
    for entity in &config.entity {
        if !is_valid_entity_name(&entity.name) {
            errors.push(format!(
                "entity '{}': name must be PascalCase (uppercase letter followed by alphanumeric characters)",
                entity.name
            ));
        }

        if seen_entities.contains(&entity.name) {
            errors.push(format!("duplicate entity name '{}'", entity.name));
        }
        seen_entities.push(entity.name.clone());

        if entity.use_cases.is_empty() {
            warnings.push(format!(
                "entity '{}': has no use cases defined",
                entity.name
            ));
        }

        for uc in &entity.use_cases {
            if !is_valid_use_case_name(uc) {
                errors.push(format!(
                    "entity '{}': use case '{}' is not valid snake_case",
                    entity.name, uc
                ));
            }
        }

        let mut seen_fields: Vec<String> = vec![];
        for field in &entity.fields {
            if field.name.is_empty() {
                errors.push(format!("entity '{}': field has empty name", entity.name));
                continue;
            }

            if !crate::puerto_toml::is_valid_field_name_pub(&field.name) {
                errors.push(format!(
                    "entity '{}': field '{}' is not valid snake_case (lowercase letters, digits, underscores; cannot start with digit)",
                    entity.name, field.name
                ));
            }

            if seen_fields.contains(&field.name) {
                errors.push(format!(
                    "entity '{}': duplicate field name '{}'",
                    entity.name, field.name
                ));
            }
            seen_fields.push(field.name.clone());

            if let Err(e) = resolve_type(&field.field_type) {
                errors.push(format!(
                    "entity '{}': field '{}' has {}",
                    entity.name, field.name, e
                ));
            }

            if field.field_type.starts_with("Option<") && field.unique {
                warnings.push(format!(
                    "entity '{}': field '{}' is Option<> and marked unique — nullable unique fields may not be intended",
                    entity.name, field.name
                ));
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("Validation failed:");
        for e in &errors {
            eprintln!("  ✗ {e}");
        }
        if !warnings.is_empty() {
            for w in &warnings {
                eprintln!("  ⚠ {w}");
            }
        }
        let msg = errors.join("\n  ");
        return Err(format!("{}\n  {} error(s) found", msg, errors.len()).into());
    }

    if !warnings.is_empty() {
        eprintln!("Warnings:");
        for w in &warnings {
            eprintln!("  ⚠ {w}");
        }
    }

    println!(
        "✓ puerto.toml is valid — {} entit{} validated",
        config.entity.len(),
        if config.entity.len() == 1 { "y" } else { "ies" }
    );
    Ok(())
}
