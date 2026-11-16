//! The crates of a context (ADR-0023): their `Cargo.toml`, their flat `lib.rs` and their entry in
//! the root `[workspace.dependencies]`.

use std::{fs, path::Path};

use crate::generators::layout::{self, Layer};
use crate::generators::naming::{pascal_to_snake, write_file};
use crate::generators::render::render;
use crate::puerto_toml::{Field, ValueObjectDefinition};

/// What a context's crates depend on besides their own layers.
#[derive(Debug, Default, Clone)]
pub struct ContextDeps {
    /// The contexts its fields reference (ADR-0022): `dominio_<r>`, and `infra_<r>` for the seeds.
    pub refs: Vec<String>,
    /// It uses a shared value object: `nucleo`.
    pub shared: bool,
    /// Its writes go through a unit of work and record events (ADR-0018).
    pub events: bool,
    pub db: bool,
    /// An auth project: the tests' callers come from `puerto-nucleo`'s doubles.
    pub auth: bool,
}

impl ContextDeps {
    pub fn of(
        fields: &[Field],
        shared_vos: &[ValueObjectDefinition],
        db: bool,
        events: bool,
    ) -> Self {
        let mut refs: Vec<String> = fields
            .iter()
            .filter_map(|f| f.references.as_deref())
            .map(pascal_to_snake)
            .collect();
        refs.sort();
        refs.dedup();
        Self {
            refs,
            shared: fields
                .iter()
                .any(|f| crate::generators::types::is_shared_vo(f, shared_vos)),
            events,
            db,
            auth: false,
        }
    }

    /// The dependencies of a CRUD context with these `fields`, in the project of `base`.
    pub fn read(base: &Path, fields: &[Field]) -> Self {
        let config = crate::puerto_toml::read(base).ok();
        let project = config
            .as_ref()
            .map(|c| c.project.clone())
            .unwrap_or_default();
        let shared_vos = config.map(|c| c.value_object).unwrap_or_default();
        Self {
            auth: project.has_auth(),
            ..Self::of(fields, &shared_vos, project.db, project.has_events())
        }
    }
}

/// Writes the crate's `Cargo.toml` and `lib.rs` and declares it in the workspace: what a layer
/// generator does once its files are on disk.
pub fn finish(
    base: &Path,
    c: &str,
    layer: Layer,
    deps: &ContextDeps,
) -> Result<(), Box<dyn std::error::Error>> {
    write_manifest(base, c, layer, deps)?;
    write_lib(base, &layout::crate_dir(c, layer), "")?;
    add_internal_dependency(
        base,
        &layout::crate_name(c, layer),
        &layout::crate_dir(c, layer),
    )?;
    sync_workspace_dependencies(base)?;
    Ok(())
}

pub fn write_manifest(
    base: &Path,
    c: &str,
    layer: Layer,
    deps: &ContextDeps,
) -> Result<(), Box<dyn std::error::Error>> {
    let template = match layer {
        Layer::Domain => "contexto/dominio_cargo.tera",
        Layer::Application => "contexto/aplicacion_cargo.tera",
        Layer::Infrastructure => "contexto/infra_cargo.tera",
        Layer::Rest => "contexto/rest_cargo.tera",
    };
    let mut ctx = tera::Context::new();
    ctx.insert("c", c);
    ctx.insert("refs", &deps.refs);
    ctx.insert("shared", &deps.shared);
    ctx.insert("events", &deps.events);
    ctx.insert("db", &deps.db);
    ctx.insert("auth", &deps.auth);
    write_file(
        &base.join(layout::crate_manifest(c, layer)),
        &render(template, &ctx)?,
    )?;
    Ok(())
}

/// The crate's `lib.rs`, written whole from what is on disk: one `pub mod` per file of `src/`,
/// sorted, and the files of `src/pruebas/` behind the `pruebas` feature (ADR-0023 § 5).
/// `header` goes first (a doc comment, or nothing).
pub fn write_lib(base: &Path, crate_dir: &str, header: &str) -> std::io::Result<()> {
    let src = base.join(crate_dir).join("src");
    let modules: Vec<String> = modules_in(&src)?
        .into_iter()
        .filter(|m| m != "lib" && m != "main")
        .collect();
    let tests = modules_in(&src.join("pruebas"))?;
    let mut out = header.to_string();
    if !header.is_empty() && !(modules.is_empty() && tests.is_empty()) {
        out.push('\n');
    }
    for module in &modules {
        out.push_str(&format!("pub mod {module};\n"));
    }
    if !tests.is_empty() {
        if !modules.is_empty() {
            out.push('\n');
        }
        out.push_str("#[cfg(any(test, feature = \"pruebas\"))]\npub mod pruebas {\n");
        for module in tests {
            out.push_str(&format!("    pub mod {module};\n"));
        }
        out.push_str("}\n");
    }
    write_file(&src.join("lib.rs"), &out)
}

/// The `.rs` files directly in `dir`, without extension and sorted; none when it does not exist.
fn modules_in(dir: &Path) -> std::io::Result<Vec<String>> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(vec![]);
    };
    let mut modules = vec![];
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "rs")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            modules.push(stem.to_string());
        }
    }
    modules.sort();
    Ok(modules)
}

/// Adds `pub mod <module>;` to a crate's `lib.rs` when it is not there: `generar caso-de-uso`.
pub fn add_lib_module(
    base: &Path,
    lib: &str,
    module: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(lib);
    let src = fs::read_to_string(&path)?;
    let line = format!("pub mod {module};");
    if src.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    // After the last top-level `pub mod x;`, so a `pruebas` block stays last.
    let mut lines: Vec<&str> = src.lines().collect();
    let at = lines
        .iter()
        .rposition(|l| l.starts_with("pub mod ") && l.ends_with(';'))
        .map_or(0, |i| i + 1);
    lines.insert(at, &line);
    fs::write(&path, lines.join("\n") + "\n")?;
    Ok(())
}

/// Declares one of the project's crates in the root `[workspace.dependencies]`, at the end of the
/// table, so every `Cargo.toml` names it with `{ workspace = true }`.
pub fn add_internal_dependency(
    base: &Path,
    name: &str,
    dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if dir.starts_with(layout::CONTEXTS_DIR) {
        add_contexts_member(base)?;
    }
    crate::generators::project::insert_workspace_dependency(
        base,
        name,
        &format!("{{ path = \"{dir}\" }}"),
        None,
    )
}

/// The workspace member glob of the contexts. Cargo reads a glob that matches nothing as a path
/// and fails to load it, so a project without contexts (`--sin-demo`) lists none until the first.
pub const CONTEXTS_MEMBER: &str = "\"contextos/*/*\"";

fn add_contexts_member(base: &Path) -> std::io::Result<()> {
    let path = base.join(layout::WORKSPACE_MANIFEST);
    let src = fs::read_to_string(&path)?;
    if src.contains(CONTEXTS_MEMBER) {
        return Ok(());
    }
    fs::write(
        &path,
        src.replacen(
            "\"presentacion/*\"]",
            &format!("{CONTEXTS_MEMBER}, \"presentacion/*\"]"),
            1,
        ),
    )
}

/// Drops the contexts' member glob: `--sin-demo` removes the only context.
pub fn remove_contexts_member(base: &Path) -> std::io::Result<()> {
    let path = base.join(layout::WORKSPACE_MANIFEST);
    let src = fs::read_to_string(&path)?;
    fs::write(&path, src.replacen(&format!("{CONTEXTS_MEMBER}, "), "", 1))
}

/// Removes one of the project's crates from the root `[workspace.dependencies]` (`--sin-demo`).
pub fn remove_internal_dependency(base: &Path, name: &str) -> std::io::Result<()> {
    remove_dependency_line(base, layout::WORKSPACE_MANIFEST, name)
}

/// Removes the `name = …` lines of the manifest `manifest`, whatever its section.
pub fn remove_dependency_line(base: &Path, manifest: &str, name: &str) -> std::io::Result<()> {
    let path = base.join(manifest);
    let src = fs::read_to_string(&path)?;
    let prefix = format!("{name} = ");
    let out: String = src
        .split_inclusive('\n')
        .filter(|l| !l.starts_with(&prefix))
        .collect();
    fs::write(&path, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("puerto_crates_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("k/src/pruebas")).unwrap();
        dir
    }

    #[test]
    fn should_declare_every_file_and_put_the_test_helpers_behind_the_feature() {
        let dir = temp("lib");
        for f in ["modelo.rs", "errores.rs", "lib.rs", "pruebas/madres.rs"] {
            fs::write(dir.join("k/src").join(f), "").unwrap();
        }

        write_lib(&dir, "k", "").unwrap();

        assert_eq!(
            fs::read_to_string(dir.join("k/src/lib.rs")).unwrap(),
            "pub mod errores;\npub mod modelo;\n\n#[cfg(any(test, feature = \"pruebas\"))]\n\
             pub mod pruebas {\n    pub mod madres;\n}\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn should_add_a_module_before_the_test_helpers_once() {
        let dir = temp("add");
        let lib = "k/src/lib.rs";
        fs::write(
            dir.join(lib),
            "pub mod a;\n\n#[cfg(test)]\npub mod pruebas {\n    pub mod madres;\n}\n",
        )
        .unwrap();

        add_lib_module(&dir, lib, "b").unwrap();
        add_lib_module(&dir, lib, "b").unwrap();

        assert_eq!(
            fs::read_to_string(dir.join(lib)).unwrap(),
            "pub mod a;\npub mod b;\n\n#[cfg(test)]\npub mod pruebas {\n    pub mod madres;\n}\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// The third-party crates Puerto's manifests name, in the order the root `Cargo.toml` lists them,
/// each with the lines it is declared with.
const THIRD_PARTY: &[(&str, &str)] = &[
    ("async-trait", "async-trait = \"0.1\"\n"),
    ("thiserror", "thiserror = \"2\"\n"),
    ("tracing", "tracing = \"0.1\"\n"),
    (
        "uuid",
        "uuid = { version = \"1\", features = [\"v4\", \"serde\"] }\n",
    ),
    (
        "chrono",
        "chrono = { version = \"0.4\", features = [\"serde\"] }\n",
    ),
    (
        "serde",
        "serde = { version = \"1\", features = [\"derive\"] }\n",
    ),
    ("serde_json", "serde_json = \"1\"\n"),
    ("mockall", "mockall = \"0.13\"\n"),
    (
        "tokio",
        "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }\n",
    ),
    ("poem", "poem = \"3\"\n"),
    (
        "poem-openapi",
        "# Feature `chrono`: sin ella, poem-openapi no implementa `Type` para DateTime<Utc>.\n\
         poem-openapi = { version = \"5\", features = [\"swagger-ui\", \"uuid\", \"chrono\"] }\n",
    ),
    (
        "sqlx",
        "sqlx = { version = \"0.8\", features = [\"runtime-tokio-rustls\", \"postgres\", \"macros\", \
         \"migrate\", \"uuid\", \"chrono\", \"json\"] }\n",
    ),
];

const THIRD_PARTY_HEADER: &str = "# Crates de terceros: una sola versión para todo el proyecto.\n";

/// The root `[workspace.dependencies]` declares exactly the third-party crates (and `nucleo`) the
/// members name: Cargo warns about every declared one no member uses
/// (`cargo::unused_workspace_dependencies`), in every build. Run after anything writes a
/// manifest. Lines Puerto does not know are left alone.
pub fn sync_workspace_dependencies(base: &Path) -> std::io::Result<()> {
    let path = base.join(layout::WORKSPACE_MANIFEST);
    let Ok(src) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let used = names_used_by_members(base);
    let mut out = String::with_capacity(src.len());
    let mut lines = src.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        if line == THIRD_PARTY_HEADER {
            out.push_str(line);
            // The block runs up to the blank line: the known crates, rewritten; the rest, kept.
            let mut unknown = String::new();
            while let Some(l) = lines.peek() {
                if l.trim().is_empty() {
                    break;
                }
                let l = lines.next().unwrap_or_default();
                let name = l.split(" = ").next().unwrap_or("").trim();
                let known = l.starts_with('#') || THIRD_PARTY.iter().any(|(n, _)| *n == name);
                if !known {
                    unknown.push_str(l);
                }
            }
            for (name, lines) in THIRD_PARTY {
                if used.iter().any(|u| u == name) {
                    out.push_str(lines);
                }
            }
            out.push_str(&unknown);
            continue;
        }
        let name = line.split(" = ").next().unwrap_or("").trim();
        let managed = name == "nucleo" || crate::runtime::RUNTIME_CRATES.contains(&name);
        if managed && !used.iter().any(|u| u == name) {
            continue;
        }
        out.push_str(line);
    }
    if used.iter().any(|u| u == "nucleo") && !out.lines().any(|l| l.starts_with("nucleo = ")) {
        out = out.replacen(
            "cada `Cargo.toml` los nombra con `{ workspace = true }`.\n",
            "cada `Cargo.toml` los nombra con `{ workspace = true }`.\nnucleo = { path = \"nucleo\" }\n",
            1,
        );
    }
    if out != src {
        fs::write(&path, out)?;
    }
    // A runtime crate a member names again (the first context of a `--sin-demo` project).
    for name in crate::runtime::RUNTIME_CRATES {
        if used.iter().any(|u| u == name) {
            crate::generators::project::add_workspace_dependency(base, name)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
    }
    Ok(())
}

/// Every name a member's manifest takes `{ workspace = true … }`.
fn names_used_by_members(base: &Path) -> Vec<String> {
    let mut manifests = vec![
        base.join(layout::NUCLEO_DIR).join("Cargo.toml"),
        base.join(layout::MIGRADOR_DIR).join("Cargo.toml"),
    ];
    for dir in [layout::CONTEXTS_DIR, "presentacion"] {
        let Ok(entries) = fs::read_dir(base.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("Cargo.toml").exists() {
                manifests.push(path.join("Cargo.toml"));
            }
            if let Ok(layers) = fs::read_dir(&path) {
                manifests.extend(layers.flatten().map(|l| l.path().join("Cargo.toml")));
            }
        }
    }
    let mut used = vec![];
    for manifest in manifests {
        let Ok(src) = fs::read_to_string(&manifest) else {
            continue;
        };
        for line in src.lines() {
            if let Some((name, value)) = line.split_once(" = ")
                && value.contains("workspace = true")
                && !used.iter().any(|u| u == name.trim())
            {
                used.push(name.trim().to_string());
            }
        }
    }
    used
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn should_declare_only_the_crates_some_member_names() {
        let dir = std::env::temp_dir().join("puerto_crates_sync");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("contextos/p/dominio")).unwrap();
        fs::create_dir_all(dir.join("nucleo")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace.dependencies]\n# Crates de terceros: una sola versión para todo el proyecto.\n\
             thiserror = \"2\"\nmockall = \"0.13\"\nmine = \"1\"\n\n# The Puerto runtime\n\
             # on the same minor as the `puerto` CLI that generated this project.\n\
             puerto-web = \"0.1\"\n\n# Los crates de este proyecto: cada `Cargo.toml` los nombra con \
             `{ workspace = true }`.\nnucleo = { path = \"nucleo\" }\n",
        )
        .unwrap();
        fs::write(
            dir.join("contextos/p/dominio/Cargo.toml"),
            "[dependencies]\nuuid = { workspace = true }\nthiserror = { workspace = true }\n\
             puerto-nucleo = { workspace = true }\n",
        )
        .unwrap();
        fs::write(dir.join("nucleo/Cargo.toml"), "[dependencies]\n").unwrap();

        sync_workspace_dependencies(&dir).unwrap();

        let root = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(
            root.contains("thiserror = \"2\"\nuuid = { version"),
            "{root}"
        );
        assert!(!root.contains("mockall"), "{root}");
        assert!(
            root.contains("mine = \"1\""),
            "a line Puerto does not know stays: {root}"
        );
        assert!(!root.contains("puerto-web"), "{root}");
        assert!(root.contains("puerto-nucleo = \""), "{root}");
        assert!(!root.contains("nucleo = { path"), "{root}");
        let _ = fs::remove_dir_all(&dir);
    }
}
