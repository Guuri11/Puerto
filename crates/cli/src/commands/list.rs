use std::path::Path;

pub fn require_puerto_project(cwd: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !cwd.join("puerto.toml").exists() {
        return Err(
            "puerto.toml not found — run this command from the root of a Puerto project".into(),
        );
    }
    Ok(())
}

pub fn run_list(cwd: &Path) -> Result<(), Box<dyn std::error::Error>> {
    require_puerto_project(cwd)?;
    let config = crate::puerto_toml::read(cwd)?;
    print!("{}", list_report(&config));
    Ok(())
}

/// What `puerto list` prints: the project, its entities with their use cases and custom events,
/// the shared value objects, the worker's consumers and the mails. A section with nothing in it is left out.
pub fn list_report(config: &crate::puerto_toml::PuertoConfig) -> String {
    let mut out = format!("Project: {}\n", config.project.name);
    if config.project.db {
        out.push_str("Database: enabled\n");
    }
    if let Some(transport) = &config.project.events {
        out.push_str(&format!("Events: {transport}\n"));
    }
    out.push('\n');
    if config.entity.is_empty() {
        out.push_str("No entities defined.\n");
    }
    for entity in &config.entity {
        let repo_kind = if entity.db { "SQLx" } else { "InMemory" };
        out.push_str(&format!("  {} [{repo_kind}]\n", entity.name));
        for uc in &entity.use_cases {
            out.push_str(&format!("    · {uc}\n"));
        }
        if !entity.events.is_empty() {
            out.push_str(&format!("    events: {}\n", entity.events.join(", ")));
        }
    }
    if !config.value_object.is_empty() {
        out.push_str("\nShared value objects:\n");
        for vo in &config.value_object {
            out.push_str(&format!("  {} ({})\n", vo.name, vo.inner_type));
        }
    }
    if !config.consumer.is_empty() {
        out.push_str("\nConsumers:\n");
        for consumer in &config.consumer {
            out.push_str(&format!(
                "  {} ← {}\n",
                consumer.name,
                consumer.events.join(", ")
            ));
        }
    }
    if !config.mail.is_empty() {
        out.push_str("\nMails:\n");
        for mail in &config.mail {
            if mail.on.is_empty() {
                out.push_str(&format!("  {}\n", mail.name));
            } else {
                out.push_str(&format!("  {} ← {}\n", mail.name, mail.on.join(", ")));
            }
        }
    }
    out
}
