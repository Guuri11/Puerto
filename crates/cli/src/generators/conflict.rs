//! Overwrite guard for entity generation.
//!
//! `puerto generar scaffold <Name>` writes ~15 files across four layers. Run twice on the same
//! entity it used to overwrite all of them without a word — silently discarding any business rule,
//! custom use case body or test the user had written. Rails prompts on conflict; so do we.
//!
//! Files Puerto declares auto-generated (`generated/bootstrap.rs`, `api/tags.rs`) are deliberately
//! **not** guarded: they are regenerated from puerto.toml by design and carry a header saying so.

use crate::generators::layout;
use std::path::{Path, PathBuf};

use crate::generators::naming::crud_use_case;

/// What to do when generation would overwrite existing files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwritePolicy {
    /// Ask when a terminal is attached; abort otherwise.
    Ask,
    /// Overwrite without asking (`--forzar`).
    Force,
}

/// The files `scaffold` writes for one entity, relative to the project root.
///
/// Kept here rather than inferred from the writers so the check runs *before* the first byte is
/// written: a half-overwritten project is worse than a refused command.
///
/// `web`: the entity also gets a frontend slice (`web::web_entity_files`).
pub fn entity_files(snake: &str, db: bool, web: bool) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = vec![
        layout::domain_file(snake, "model"),
        layout::domain_file(snake, "errors"),
        layout::domain_file(snake, "repository"),
        layout::domain_file(snake, "value_objects"),
        layout::use_case_contract(snake, &format!("create_{snake}")),
        layout::use_case_contract(snake, &format!("get_{snake}")),
        layout::use_case_contract(snake, &crud_use_case("list", snake)),
        layout::use_case_contract(snake, &format!("update_{snake}")),
        layout::use_case_contract(snake, &format!("delete_{snake}")),
        layout::use_case_impl(snake, &format!("create_{snake}")),
        layout::use_case_impl(snake, &format!("get_{snake}")),
        layout::use_case_impl(snake, &crud_use_case("list", snake)),
        layout::use_case_impl(snake, &format!("update_{snake}")),
        layout::use_case_impl(snake, &format!("delete_{snake}")),
        layout::mother(snake),
        layout::infra_file(snake, "repository"),
        layout::api_module(snake),
        layout::api_file(snake, "dto"),
        layout::api_file(snake, "routes"),
        layout::api_file(snake, "responses"),
        layout::api_file(snake, "error_mapper"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();

    if db {
        files.push(PathBuf::from(layout::infra_file(snake, "entity")));
    }
    if web {
        files.extend(crate::generators::web::web_entity_files(snake));
    }

    files
}

/// Which of `files` already exist under `base`.
pub fn existing(base: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|f| base.join(f).exists())
        .cloned()
        .collect()
}

/// Aborts (or asks) when generating `snake` would overwrite user-owned files.
///
/// Returns `Ok(())` when generation may proceed.
pub fn guard_entity_overwrite(
    base: &Path,
    snake: &str,
    db: bool,
    web: bool,
    policy: OverwritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    guard_files_overwrite(base, snake, &entity_files(snake, db, web), policy)
}

/// Aborts (or asks) when writing `files` for `snake` would overwrite any of them.
pub fn guard_files_overwrite(
    base: &Path,
    snake: &str,
    files: &[PathBuf],
    policy: OverwritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    if policy == OverwritePolicy::Force {
        return Ok(());
    }

    let conflicts = existing(base, files);
    if conflicts.is_empty() {
        return Ok(());
    }

    eprintln!(
        "'{snake}' is already generated — this would overwrite {} file(s):",
        conflicts.len()
    );
    for f in conflicts.iter().take(10) {
        eprintln!("  {}", f.display());
    }
    if conflicts.len() > 10 {
        eprintln!("  … and {} more", conflicts.len() - 10);
    }
    eprintln!();

    if dialoguer::console::user_attended() {
        let proceed = dialoguer::Confirm::new()
            .with_prompt("Overwrite these files? Any hand-written changes in them are lost")
            .default(false)
            .interact()?;
        if proceed {
            return Ok(());
        }
        return Err("aborted — no files were written".into());
    }

    Err(format!(
        "refusing to overwrite existing files for '{snake}'. Re-run with --forzar to overwrite, \
         or remove the files first. Nothing was written."
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_files_includes_db_entity_only_with_db() {
        assert!(
            !entity_files("product", false, false)
                .iter()
                .any(|f| f.ends_with("entity.rs"))
        );
        assert!(
            entity_files("product", true, false)
                .iter()
                .any(|f| f.ends_with("entity.rs"))
        );
    }

    #[test]
    fn entity_files_excludes_auto_generated_files() {
        let files = entity_files("product", true, true);
        assert!(!files.iter().any(|f| f.ends_with("bootstrap.rs")));
        assert!(!files.iter().any(|f| f.ends_with("tags.rs")));
        assert!(
            !files
                .iter()
                .any(|f| f.to_string_lossy().contains("generated"))
        );
        assert!(!files.iter().any(|f| f.ends_with("client.ts")));
    }

    #[test]
    fn force_policy_never_blocks() {
        let dir = std::env::temp_dir();
        assert!(
            guard_entity_overwrite(&dir, "product", false, false, OverwritePolicy::Force).is_ok()
        );
    }

    #[test]
    fn entity_files_include_the_web_slice_only_in_a_web_project() {
        let has_web = |files: Vec<PathBuf>| files.iter().any(|f| f.starts_with("web"));
        assert!(!has_web(entity_files("order_item", false, false)));
        assert!(
            entity_files("order_item", false, true)
                .contains(&PathBuf::from("web/src/test-support/order-item.ts"))
        );
    }
}
