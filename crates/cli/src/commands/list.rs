use std::path::Path;

pub fn require_puerto_project(cwd: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !cwd.join("puerto.toml").exists() {
        return Err(
            "no se encuentra puerto.toml: ejecuta este comando desde la raíz de un proyecto Puerto"
                .into(),
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

/// What `puerto listar` prints: the project, its entities with their use cases and custom events,
/// the shared value objects, the worker's consumers and the mails. A section with nothing in it is left out.
pub fn list_report(config: &crate::puerto_toml::PuertoConfig) -> String {
    let mut out = format!("Proyecto: {}\n", config.project.name);
    if config.project.db {
        out.push_str("Base de datos: sí\n");
    }
    if let Some(transport) = &config.project.events {
        out.push_str(&format!("Eventos: {transport}\n"));
    }
    out.push('\n');
    if config.entity.is_empty() {
        out.push_str("No hay entidades.\n");
    }
    for entity in &config.entity {
        let repo_kind = if entity.db { "Postgres" } else { "en memoria" };
        out.push_str(&format!("  {} [{repo_kind}]\n", entity.name));
        for uc in &entity.use_cases {
            out.push_str(&format!("    · {uc}\n"));
        }
        if !entity.events.is_empty() {
            out.push_str(&format!("    eventos: {}\n", entity.events.join(", ")));
        }
    }
    if !config.value_object.is_empty() {
        out.push_str("\nObjetos valor compartidos:\n");
        for vo in &config.value_object {
            out.push_str(&format!("  {} ({})\n", vo.name, vo.inner_type));
        }
    }
    if !config.consumer.is_empty() {
        out.push_str("\nConsumidores:\n");
        for consumer in &config.consumer {
            out.push_str(&format!(
                "  {} ← {}\n",
                consumer.name,
                consumer.events.join(", ")
            ));
        }
    }
    if !config.mail.is_empty() {
        out.push_str("\nCorreos:\n");
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
