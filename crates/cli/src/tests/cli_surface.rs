use super::*;

// ── CLI surface ───────────────────────────────────────────────────────────

#[test]
fn cli_supports_version_flag() {
    // `puerto --version` is the first command the install docs tell a user to run. Without
    // `version` on `#[command(...)]` clap never generates the flag, and it exits 2 with
    // "unexpected argument '--version' found".
    use clap::CommandFactory;

    let version = crate::Cli::command().get_version().map(str::to_string);
    assert_eq!(version.as_deref(), Some(env!("CARGO_PKG_VERSION")));

    let err = crate::Cli::command()
        .try_get_matches_from(["puerto", "--version"])
        .expect_err("--version short-circuits parsing");
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
}

#[test]
fn clap_help_and_errors_are_in_spanish() {
    // The help and the errors are clap's: their headings and phrases are English until
    // `clap_en_castellano` replaces them.
    use clap::CommandFactory;
    let render = |args: &[&str]| {
        let err = crate::Cli::command()
            .try_get_matches_from(args)
            .expect_err("help and bad input stop parsing");
        crate::clap_en_castellano(&err.render().to_string())
    };

    let root = render(&["puerto", "--help"]);
    let scaffold = render(&["puerto", "generar", "scaffold", "--help"]);
    let typo = render(&["puerto", "generar", "scafold"]);
    let missing = render(&["puerto", "generar", "caso-de-uso"]);

    for (help, text) in [("root", &root), ("scaffold", &scaffold)] {
        for english in [
            "Usage:",
            "Commands:",
            "Arguments:",
            "Options:",
            "[OPTIONS]",
            "Print",
        ] {
            assert!(!text.contains(english), "{help}: {english}\n{text}");
        }
    }
    assert!(root.contains("Uso: puerto <COMANDO>"), "{root}");
    assert!(root.contains("-h, --help     Muestra la ayuda"), "{root}");
    assert!(
        scaffold.contains("[OPCIONES] <NOMBRE> [-- <CAMPOS>...]"),
        "{scaffold}"
    );
    assert!(typo.contains("subcomando desconocido 'scafold'"), "{typo}");
    assert!(
        typo.contains("hay un subcomando parecido: 'scaffold'"),
        "{typo}"
    );
    assert!(
        missing.contains("faltan estos argumentos obligatorios:"),
        "{missing}"
    );
    assert!(
        missing.contains("Para más información, prueba '--help'."),
        "{missing}"
    );
}

#[test]
fn cli_definition_is_well_formed() {
    // Catches conflicting flags, duplicate short options and broken `raw = true` args at test
    // time rather than at the first invocation.
    use clap::CommandFactory;
    crate::Cli::command().debug_assert();
}

// ── puerto listar ──────────────────────────────────────────────────────────

#[test]
fn list_fails_outside_puerto_project() {
    let dir = temp_dir("listar_no_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let result = run_list(&dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("no se encuentra puerto.toml"));

    cleanup(&dir);
}

#[test]
fn list_succeeds_inside_puerto_project() {
    let dir = temp_dir("listar_with_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let output = generate_project("list-app", &dir).unwrap();
    run_list(&output).unwrap();

    cleanup(&dir);
}

// ── require_puerto_project ────────────────────────────────────────────────

#[test]
fn generate_scaffold_fails_outside_puerto_project() {
    let dir = temp_dir("gen_no_toml");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let result = require_puerto_project(&dir);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no se encuentra puerto.toml")
    );

    cleanup(&dir);
}
