//! Formats a project after a generator wrote Rust into it.
//!
//! Templates and patchers produce valid Rust, not rustfmt's layout: blank-line runs, imports in
//! insertion order, lines past 100 columns once an entity name is long. The generated project's
//! `make format` (`cargo fmt --check`) would fail on code nobody wrote by hand. So every command
//! that writes Rust ends with `cargo fmt --all`, run with the project's own pinned toolchain and
//! `rustfmt.toml`, if any.
//!
//! The whole workspace, not the written files: `rustfmt` follows `mod` declarations, so formatting
//! a patched `lib.rs` already formats its crate.

use std::path::Path;
use std::process::Command;

/// Runs `cargo fmt --all` in `root`. `Err` carries what to tell the user.
pub fn format_project(root: &Path) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["fmt", "--all"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run `cargo fmt`: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first_line = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    Err(format!("`cargo fmt` failed: {first_line}"))
}

/// [`format_project`] for the CLI: a project that cannot be formatted (no `rustfmt`, a declared
/// module whose file does not exist yet) is still generated — the user is told how to finish.
pub fn format_after_generation(root: &Path) {
    if let Err(e) = format_project(root) {
        eprintln!("⚠ generated code left unformatted — {e}");
        eprintln!("  Run `make format/fix` once the project builds.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn crate_with(lib: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"fmt-probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), lib).unwrap();
        dir
    }

    #[test]
    fn should_format_generated_rust_when_the_project_parses() {
        // Arrange
        let dir = crate_with("pub mod domain {\n}\n\n\npub fn  greet( ) {}\n");

        // Act
        let result = format_project(dir.path());

        // Assert
        assert_eq!(result, Ok(()));
        let lib = fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();
        assert_eq!(lib, "pub mod domain {}\n\npub fn greet() {}\n");
    }

    #[test]
    fn should_report_instead_of_failing_when_a_declared_module_is_missing() {
        // Arrange
        let dir = crate_with("pub mod missing;\n");

        // Act
        let result = format_project(dir.path());

        // Assert
        let message = result.unwrap_err();
        assert!(message.starts_with("`cargo fmt` failed"), "{message}");
    }
}
