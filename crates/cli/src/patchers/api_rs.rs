use std::{fs, path::Path};

pub fn patch_api_rs(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("presentation/src/api.rs");
    let mut src = fs::read_to_string(&path)?;

    let mod_line = format!("pub mod {snake};\n");
    if src.contains(&mod_line) {
        return Ok(());
    }

    if !src.ends_with('\n') {
        src.push('\n');
    }
    src.push_str(&mod_line);

    fs::write(&path, src)?;
    Ok(())
}
