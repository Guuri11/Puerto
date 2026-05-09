use std::{fs, path::Path};

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

pub fn patch_business_lib_domain_crud(
    base: &Path,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    if src.contains(&format!("pub mod {snake} {{\n        pub mod errors;")) {
        return Ok(());
    }
    let domain_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {{\n            pub mod create_{snake};\n            pub mod get_{snake};\n            pub mod list_{snake};\n            pub mod update_{snake};\n            pub mod delete_{snake};\n        }}\n    }}\n"
    );
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
    let app_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod create_{snake};\n        pub mod get_{snake};\n        pub mod list_{snake};\n        pub mod update_{snake};\n        pub mod delete_{snake};\n    }}\n"
    );
    let patched = insert_before_block_end(&src, "application", &app_mod)?;
    fs::write(&path, patched)?;
    Ok(())
}

pub fn patch_business_lib_crud(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    patch_business_lib_domain_crud(base, snake)?;
    patch_business_lib_application_crud(base, snake)?;
    Ok(())
}

pub fn patch_infra_lib(
    base: &Path,
    snake: &str,
    db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("infrastructure/src/lib.rs");
    let mut src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake} {{")) {
        return Ok(());
    }

    if !src.ends_with('\n') {
        src.push('\n');
    }
    if db {
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
    let entity_marker = format!("pub mod {snake} {{");
    if let Some(start) = src.find(&entity_marker) {
        let rest = &src[start..];
        let mut depth = 0usize;
        let mut block_end = rest.len();
        for (i, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        block_end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if rest[..block_end].contains("pub mod value_objects;") {
            return Ok(());
        }
    }

    let content = "\n        pub mod value_objects;\n".to_string();
    let patched = patch_lib_block(&src, &["domain", snake], &content)?;
    fs::write(&path, patched)?;
    Ok(())
}

pub fn try_patch_libs(snake: &str, base: &Path, db: bool, crud: bool) -> bool {
    let business_ok = if crud {
        patch_business_lib_crud(base, snake).is_ok()
    } else {
        patch_business_lib(base, snake).is_ok()
    };
    business_ok && patch_infra_lib(base, snake, db).is_ok() && patch_api_rs(base, snake).is_ok()
}
