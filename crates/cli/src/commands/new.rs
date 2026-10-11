use cargo_generate::{GenerateArgs, TemplatePath, generate};
use std::path::PathBuf;

pub fn extract_template() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    crate::TEMPLATE_DIR.extract(tmp.path())?;
    Ok(tmp)
}

pub fn generate_new_project(
    name: Option<String>,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let tmp = extract_template()?;

    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(tmp.path().to_string_lossy().into_owned()),
            ..Default::default()
        },
        name: name.clone(),
        destination,
        no_workspace: true,
        quiet: true,
        ..Default::default()
    };

    let output = generate(args)?;
    Ok(output)
}

/// A project name becomes the directory, the binary and the Postgres database name. cargo-generate
/// silently rewrites anything else (`"Bad Name"` → `bad-name`), so the project would not be called
/// what the user typed.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        return Err(format!(
            "invalid project name '{name}'. Use lowercase letters, digits, '-' and '_', starting \
             with a letter (e.g. 'my-app')"
        ));
    }
    Ok(())
}

pub fn new_project(
    name: Option<String>,
    db: bool,
    no_db: bool,
    no_demo: bool,
    no_web: bool,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(name) = &name {
        validate_project_name(name)?;
    }

    let resolved_db = if db {
        true
    } else if no_db {
        false
    } else if dialoguer::console::user_attended() {
        dialoguer::Confirm::new()
            .with_prompt("Include database support (SQLx + Postgres)?")
            .default(false)
            .interact()?
    } else {
        false
    };

    let resolved_no_demo = if no_demo {
        true
    } else if dialoguer::console::user_attended() {
        !dialoguer::Confirm::new()
            .with_prompt("Include Greeting demo entity?")
            .default(true)
            .interact()?
    } else {
        false
    };

    let resolved_no_web = if no_web {
        true
    } else if dialoguer::console::user_attended() {
        !dialoguer::Confirm::new()
            .with_prompt("Include the web frontend (React + daisyUI, needs Bun)?")
            .default(true)
            .interact()?
    } else {
        false
    };

    eprintln!("Constructing project skeleton...");

    let output = generate_new_project(name, destination)?;

    crate::generators::project::apply_base_env(&output)?;

    if resolved_db {
        crate::generators::project::apply_db_to_new_project(&output)?;
    }

    if resolved_no_demo {
        crate::generators::project::apply_no_demo(&output)?;
    }

    if resolved_no_web {
        crate::generators::project::apply_no_web(&output)?;
    }

    crate::snippets::apply(&output, None)?;

    Ok(output)
}
