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

    for uc in use_cases {
        let uc_pascal = to_pascal_case(uc);
        add_api_field(&mut src, pascal, snake, uc, &uc_pascal);
        add_test_literal_field(&mut src, pascal, snake, uc, &uc_pascal);
    }

    if src != original {
        fs::write(&path, src)?;
    }
    Ok(())
}

fn add_api_field(src: &mut String, pascal: &str, snake: &str, uc: &str, uc_pascal: &str) {
    let struct_open = format!("pub struct {pascal}Api {{\n");
    let Some(struct_start) = src.find(&struct_open) else {
        return;
    };
    let body_start = struct_start + struct_open.len();
    let Some(body_len) = src[body_start..].find("\n}") else {
        return;
    };
    if src[body_start..body_start + body_len].contains(&format!("pub {uc}:")) {
        return;
    }

    let field = format!(
        "    /// Wired by bootstrap.rs. Call it from an endpoint in the `#[OpenApi]` impl below.\n    #[allow(dead_code)]\n    pub {uc}: Arc<dyn {uc_pascal}UseCaseTrait>,\n"
    );
    // Appended as the last field, right before the struct's closing brace.
    src.insert_str(body_start + body_len + 1, &field);

    let import =
        format!("use business::domain::{snake}::use_cases::{uc}::{uc_pascal}UseCaseTrait;\n");
    if !src.contains(&import) {
        src.insert_str(0, &import);
    }
}

/// The route tests build `{Pascal}Api` with a struct literal (`let api = {Pascal}Api {`): a new
/// field makes it stop compiling (E0063). It gets a mock with no expectations — a route that
/// calls it panics. Spelled out in full: the `mocks` module exists only under test, so a
/// file-level import would break the non-test build.
fn add_test_literal_field(src: &mut String, pascal: &str, snake: &str, uc: &str, uc_pascal: &str) {
    let literal_open = format!("let api = {pascal}Api {{\n");
    let Some(open) = src.find(&literal_open) else {
        return;
    };
    let body_start = open + literal_open.len();
    let Some(close) = src[body_start..].find("};").map(|i| body_start + i) else {
        return;
    };
    let body = &src[body_start..close];
    if body
        .lines()
        .any(|l| l.trim_start().starts_with(&format!("{uc}:")))
    {
        return;
    }

    let line_start = src[..close].rfind('\n').map_or(0, |i| i + 1);
    let indent = format!("{}    ", &src[line_start..close]);
    let field = format!(
        "{indent}{uc}: Arc::new(\n{indent}    business::domain::{snake}::use_cases::{uc}::mocks::Mock{uc_pascal}UseCase::new(),\n{indent}),\n"
    );
    src.insert_str(line_start, &field);
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
