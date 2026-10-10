use std::{fs, path::Path};

use crate::generators::naming::to_pascal_case;

/// Give `{Pascal}Api` a field for every use case in `use_cases` it does not already have.
///
/// `bootstrap.rs` builds `{Pascal}Api { <every use case in puerto.toml> }`, but `routes.rs`
/// is rendered from the CRUD template, which only knows the five CRUD use cases. A use case added
/// by `generate use-case` — or one that survived a `scaffold --force` — made bootstrap.rs name a
/// field the struct does not have (E0560).
///
/// Idempotent, and a no-op when `routes.rs` does not exist yet (domain-first flow before
/// `generate presentation`, which calls this again).
pub fn patch_routes_use_cases(
    base: &Path,
    pascal: &str,
    snake: &str,
    use_cases: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(format!("presentation/src/api/{snake}/routes.rs"));
    if !path.exists() {
        return Ok(());
    }
    let mut src = fs::read_to_string(&path)?;
    let original = src.clone();

    let struct_open = format!("pub struct {pascal}Api {{\n");

    for uc in use_cases {
        let Some(struct_start) = src.find(&struct_open) else {
            break;
        };
        let body_start = struct_start + struct_open.len();
        let Some(body_len) = src[body_start..].find("\n}") else {
            break;
        };
        let body = &src[body_start..body_start + body_len];
        if body.contains(&format!("pub {uc}:")) {
            continue;
        }

        let uc_pascal = to_pascal_case(uc);
        let field = format!(
            "    /// Wired by bootstrap.rs. Call it from an endpoint in the `#[OpenApi]` impl below.\n    #[allow(dead_code)]\n    pub {uc}: Arc<{uc_pascal}UseCaseImpl>,\n"
        );
        // Appended as the last field, right before the struct's closing brace.
        src.insert_str(body_start + body_len + 1, &field);

        let import = format!("use business::application::{snake}::{uc}::{uc_pascal}UseCaseImpl;\n");
        if !src.contains(&import) {
            src.insert_str(0, &import);
        }
    }

    if src != original {
        fs::write(&path, src)?;
    }
    Ok(())
}

/// [`patch_routes_use_cases`] with the entity's use cases read from puerto.toml.
pub fn patch_routes_from_manifest(
    base: &Path,
    pascal: &str,
    snake: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    if let Some(entity) = config.entity.iter().find(|e| e.name == pascal) {
        patch_routes_use_cases(base, pascal, snake, &entity.use_cases)?;
    }
    Ok(())
}
