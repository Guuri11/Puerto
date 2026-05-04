use std::{fs, path::Path};

/// Add a new SQLx migration file.
///
/// `sqlx_bin` overrides the binary name/path — used in tests to pass a stub binary.
/// Pass `None` to use the default `"sqlx"` from `$PATH`.
///
/// `content` overrides the body written into the migration file. Pass `None` to leave
/// a generic placeholder (used by `puerto generate migration`). Pass `Some(sql)` to
/// write a pre-filled schema (used by `puerto generate scaffold --db`).
pub fn run_migration(
    name: &str,
    base: &Path,
    sqlx_bin: Option<&str>,
    content: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bin = sqlx_bin.unwrap_or("sqlx");

    // Pre-flight 1: check sqlx binary is reachable.
    let sqlx_check = std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if sqlx_check.is_err() {
        return Err(
            "sqlx CLI not found\ninstall it with: cargo install sqlx-cli --no-default-features --features postgres"
                .to_string()
                .into(),
        );
    }

    // Ensure migrations directory exists — create it if needed.
    let migrations_dir = base.join("infrastructure/migrations");
    fs::create_dir_all(&migrations_dir)?;

    // Normalise name: spaces → underscores, lowercase.
    let normalised = name.replace(' ', "_").to_lowercase();

    // Delegate to sqlx CLI.
    let status = std::process::Command::new(bin)
        .args([
            "migrate",
            "add",
            &normalised,
            "--source",
            migrations_dir
                .to_str()
                .unwrap_or("infrastructure/migrations"),
        ])
        .current_dir(base)
        .status()?;

    if !status.success() {
        return Err(format!("sqlx migrate add failed (exit {:?})", status.code()).into());
    }

    // Find the newly created file and write Puerto header + body.
    if let Some(entry) = fs::read_dir(&migrations_dir)?
        .flatten()
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.contains(&normalised) && s.ends_with(".sql")
        })
        .max_by_key(|e| e.file_name())
    {
        let path = entry.path();
        let header = format!(
            "-- Puerto migration: {normalised}\n-- Run `make sqlx/prepare` after editing this file.\n\n"
        );
        let body = content.unwrap_or("-- Add migration script here\n");
        fs::write(&path, format!("{header}{body}"))?;
    }

    println!("✓ Migration '{normalised}' created in infrastructure/migrations/");
    if content.is_some() {
        println!("  Schema pre-filled — run: make sqlx/migrate");
    } else {
        println!("  Edit the file, then run: make sqlx/migrate");
    }

    Ok(())
}
