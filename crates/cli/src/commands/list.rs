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
    println!("Project: {}", config.project.name);
    if config.project.db {
        println!("Database: enabled");
    }
    println!();
    if config.entity.is_empty() {
        println!("No entities defined.");
        return Ok(());
    }
    for entity in &config.entity {
        let repo_kind = if entity.db { "SQLx" } else { "InMemory" };
        println!("  {} [{}]", entity.name, repo_kind);
        for uc in &entity.use_cases {
            println!("    · {uc}");
        }
    }
    Ok(())
}
