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

pub fn new_project(
    name: Option<String>,
    db: bool,
    no_db: bool,
    no_demo: bool,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
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

    eprintln!("Constructing project skeleton...");

    let output = generate_new_project(name, destination)?;

    if resolved_db {
        crate::generators::project::apply_db_to_new_project(&output)?;
    }

    if resolved_no_demo {
        crate::generators::project::apply_no_demo(&output)?;
    }

    crate::snippets::apply(&output, None)?;

    Ok(output)
}
