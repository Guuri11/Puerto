use std::{fs, path::Path};

use crate::generators::naming::crud_use_cases;
use crate::patchers::api_rs::patch_api_rs;

/// Find `pub mod <block_name> { ... }` and insert `content` just before the closing `}`.
pub fn insert_before_block_end(
    source: &str,
    block_name: &str,
    content: &str,
) -> Result<String, String> {
    let marker = format!("pub mod {block_name} {{");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("block '{block_name}' not found"))?;

    let after_open = start + marker.len();
    let mut depth = 1usize;
    let mut close = None;

    for (i, ch) in source[after_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(after_open + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close = close.ok_or_else(|| format!("unclosed block '{block_name}'"))?;
    Ok(format!(
        "{}{}{}",
        &source[..close],
        content,
        &source[close..]
    ))
}

/// Navigate nested `pub mod` blocks and insert `content` before the innermost closing `}`.
/// `path = &["domain", "product", "use_cases"]` finds `domain { ... product { ... use_cases { <here> } } }`.
pub fn patch_lib_block(source: &str, path: &[&str], content: &str) -> Result<String, String> {
    match path {
        [] => Err("empty path".to_string()),
        [name] => insert_before_block_end(source, name, content),
        [name, rest @ ..] => {
            let marker = format!("pub mod {name} {{");
            let start = source
                .find(&marker)
                .ok_or_else(|| format!("block '{name}' not found"))?;
            let after_open = start + marker.len();
            let mut depth = 1usize;
            let mut close = None;
            for (i, ch) in source[after_open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(after_open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.ok_or_else(|| format!("unclosed block '{name}'"))?;
            let inner = &source[after_open..close];
            let new_inner = patch_lib_block(inner, rest, content)?;
            Ok(format!(
                "{}{}{}",
                &source[..after_open],
                new_inner,
                &source[close..]
            ))
        }
    }
}

pub fn patch_business_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    let domain_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {{\n            pub mod create_{snake};\n        }}\n    }}\n"
    );
    let after_domain = insert_before_block_end(&src, "domain", &domain_mod)?;

    let app_mod = format!("\n    pub mod {snake} {{\n        pub mod create_{snake};\n    }}\n");
    let after_app = insert_before_block_end(&after_domain, "application", &app_mod)?;

    fs::write(&path, after_app)?;
    Ok(())
}

/// `{indent}pub mod {name};\n` for each name.
fn mod_lines(names: &[String], indent: &str) -> String {
    names
        .iter()
        .map(|name| format!("{indent}pub mod {name};\n"))
        .collect()
}

pub fn patch_business_lib_domain_crud(
    base: &Path,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    if src.contains(&format!("pub mod {snake} {{\n        pub mod errors;")) {
        return Ok(());
    }
    let use_case_mods = mod_lines(&crud_use_cases(snake), "            ");
    let domain_mod = if crate::puerto_toml::events_enabled(base) {
        format!(
            "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod events;\n        pub mod model;\n        pub mod repository;\n        pub mod unit_of_work;\n        pub mod use_cases {{\n{use_case_mods}        }}\n    }}\n"
        )
    } else {
        format!(
            "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {{\n{use_case_mods}        }}\n    }}\n"
        )
    };
    let patched = insert_before_block_end(&src, "domain", &domain_mod)?;
    fs::write(&path, patched)?;
    Ok(())
}

pub fn patch_business_lib_application_crud(
    base: &Path,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    let app_block_marker = format!("pub mod {snake} {{\n        pub mod create_{snake};");
    if src.contains(&app_block_marker) {
        return Ok(());
    }
    let use_case_mods = mod_lines(&crud_use_cases(snake), "        ");
    let app_mod = format!("\n    pub mod {snake} {{\n{use_case_mods}    }}\n");
    let patched = insert_before_block_end(&src, "application", &app_mod)?;
    fs::write(&path, patched)?;
    Ok(())
}

pub fn patch_business_lib_crud(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    patch_business_lib_domain_crud(base, snake)?;
    patch_business_lib_application_crud(base, snake)?;
    Ok(())
}

/// `crud`: the entity has the five scaffolded use cases — in an `events` project its writes go
/// through `unit_of_work.rs`.
pub fn patch_infra_lib(
    base: &Path,
    snake: &str,
    db: bool,
    crud: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("infrastructure/src/lib.rs");
    let mut src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake} {{")) {
        return Ok(());
    }

    if !src.ends_with('\n') {
        src.push('\n');
    }
    if db && crud && crate::puerto_toml::events_enabled(base) {
        src.push_str(&format!(
            "pub mod {snake} {{\n    pub mod entity;\n    pub mod repository;\n    pub mod unit_of_work;\n}}\n"
        ));
        if !src.contains("pub mod db;") {
            src.push_str("pub mod db;\n");
        }
    } else if db {
        src.push_str(&format!(
            "pub mod {snake} {{\n    pub mod entity;\n    pub mod repository;\n}}\n"
        ));
        // Ensure `pub mod db;` is declared (idempotent)
        if !src.contains("pub mod db;") {
            src.push_str("pub mod db;\n");
        }
    } else {
        src.push_str(&format!(
            "pub mod {snake} {{\n    pub mod repository;\n}}\n"
        ));
    }

    fs::write(&path, src)?;
    Ok(())
}

pub fn patch_business_lib_use_case(
    base: &Path,
    snake: &str,
    uc: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    // Idempotency: if already registered, skip
    let line = format!("pub mod {uc};");
    if src.contains(&line) {
        return Ok(());
    }

    let uc_content = format!("\n            pub mod {uc};\n");
    let after_uc = patch_lib_block(&src, &["domain", snake, "use_cases"], &uc_content)?;

    let app_content = format!("\n        pub mod {uc};\n");
    let final_src = patch_lib_block(&after_uc, &["application", snake], &app_content)?;

    fs::write(&path, final_src)?;
    Ok(())
}

/// Adds `pub mod shared;` inside the `domain` block of `business/src/lib.rs`.
pub fn patch_business_lib_shared(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    if src.contains("pub mod shared;") {
        return Ok(());
    }
    let content = "\n    pub mod shared;\n";
    let patched = insert_before_block_end(&src, "domain", content)?;
    fs::write(&path, patched)?;
    Ok(())
}

pub fn patch_business_lib_value_objects(
    base: &Path,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    // Check within this entity's block specifically — a previous entity may already
    // have `pub mod value_objects;` which would cause a false-positive global check.
    if let Some(block) = domain_entity_block(&src, snake)
        && src[block].contains("pub mod value_objects;")
    {
        return Ok(());
    }

    let content = "\n        pub mod value_objects;\n".to_string();
    let patched = patch_lib_block(&src, &["domain", snake], &content)?;
    fs::write(&path, patched)?;
    Ok(())
}

/// The byte range of `pub mod {snake} { … }` — the first one, the domain's: lib.rs declares
/// `domain` before `application`.
fn domain_entity_block(src: &str, snake: &str) -> Option<std::ops::Range<usize>> {
    let start = src.find(&format!("pub mod {snake} {{"))?;
    let mut depth = 0usize;
    for (i, ch) in src[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start..start + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Undoes [`patch_business_lib_value_objects`]: a forced re-scaffold whose entity no longer has a
/// local value object deletes `value_objects.rs`, so lib.rs must stop declaring it.
pub fn unpatch_business_lib_value_objects(
    base: &Path,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    let Some(block) = domain_entity_block(&src, snake) else {
        return Ok(());
    };
    let Some(offset) = src[block.clone()].find("pub mod value_objects;") else {
        return Ok(());
    };
    let at = block.start + offset;
    let line_start = src[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[at..].find('\n').map_or(src.len(), |i| at + i + 1);
    let mut patched = src;
    patched.replace_range(line_start..line_end, "");
    fs::write(&path, patched)?;
    Ok(())
}

/// Declares the new entity's modules in the three crates. Each error names the file it was patching:
/// a project whose lib.rs could not be patched does not compile, so it must not end in `✓ Done`.
pub fn patch_libs(
    snake: &str,
    base: &Path,
    db: bool,
    crud: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let in_file = |file: &str| {
        let file = file.to_string();
        move |e: Box<dyn std::error::Error>| -> Box<dyn std::error::Error> {
            format!("cannot patch {file}: {e}").into()
        }
    };
    if crud {
        patch_business_lib_crud(base, snake)
    } else {
        patch_business_lib(base, snake)
    }
    .map_err(in_file("business/src/lib.rs"))?;
    patch_infra_lib(base, snake, db, crud).map_err(in_file("infrastructure/src/lib.rs"))?;
    patch_api_rs(base, snake).map_err(in_file("presentation/src/api.rs"))?;
    Ok(())
}

/// Declares `{snake}_mother` in the `tests::mothers` block of business/src/lib.rs, creating the
/// block when the project has none yet. The block is compiled under `test-utils` too, so the route
/// tests in `presentation` can build entities with the same mothers.
pub fn patch_mothers_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake}_mother;")) {
        return Ok(());
    }

    let new_mod = format!("\n        pub mod {snake}_mother;\n");

    if let Ok(patched) = patch_lib_block(&src, &["tests", "mothers"], &new_mod) {
        fs::write(&path, patched)?;
        return Ok(());
    }

    let mut content = src;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n#[cfg(any(test, feature = \"test-utils\"))]\npub mod tests {{\n    pub mod mothers {{\n        pub mod {snake}_mother;\n    }}\n}}\n"
    ));
    fs::write(&path, content)?;
    Ok(())
}
