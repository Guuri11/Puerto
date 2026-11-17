// Phase 20: what canasta (a basketball league built with Puerto 0.20) found missing.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A project (`--db` or not) with `Linea { puntos: i64, codigo: String! }` scaffolded.
fn linea_project(name: &str, db: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("liga".into()), db, &dir).unwrap();
    let fields = vec![
        puerto_toml::parse_field_arg("puntos:i64").unwrap(),
        puerto_toml::parse_field_arg("codigo:String!").unwrap(),
    ];
    scaffold::run_scaffold("Linea", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

// ── 20.5: the tests take their values from the mother ───────────────────────

#[test]
fn generated_tests_take_their_valid_values_from_the_mother() {
    // canasta added `puntos >= 0 && puntos <= 100`-style rules and dozens of generated tests broke
    // on their own `42`s: the mother's defaults are the one place to change.
    let (dir, output) = linea_project("canasta_mother_values", false);

    for path in [
        "contextos/linea/dominio/src/modelo.rs",
        "contextos/linea/aplicacion/src/crear_linea.rs",
        "contextos/linea/aplicacion/src/obtener_linea.rs",
        "contextos/linea/aplicacion/src/listar_lineas.rs",
        "contextos/linea/aplicacion/src/actualizar_linea.rs",
        "contextos/linea/aplicacion/src/borrar_linea.rs",
        "contextos/linea/infraestructura/src/repositorio.rs",
    ] {
        let content = read(&output, path);
        let tests = &content[content.find("#[cfg(test)]").unwrap()..];
        assert!(
            !tests.contains("puntos: 42"),
            "{path} writes its own 42:\n{tests}"
        );
        assert!(
            tests.contains("MadreLinea"),
            "{path} does not use the mother:\n{tests}"
        );
    }
    let crear = read(&output, "contextos/linea/aplicacion/src/crear_linea.rs");
    assert!(
        crear.contains("let props = MadreLinea::new().construir_props();"),
        "{crear}"
    );
    assert!(crear.contains("puntos: props.puntos,"), "{crear}");
    let modelo = read(&output, "contextos/linea/dominio/src/modelo.rs");
    assert!(
        modelo.contains("Linea::new(MadreLinea::new().con_codigo_vacio().construir_props())"),
        "{modelo}"
    );
    // A unique field still gets a fresh value per row: two saved rows must not collide.
    let repositorio = read(
        &output,
        "contextos/linea/infraestructura/src/repositorio.rs",
    );
    assert!(
        repositorio.contains("..MadreLinea::new().construir_props()"),
        "{repositorio}"
    );
    assert!(
        repositorio.contains("codigo: Uuid::new_v4().to_string(),"),
        "{repositorio}"
    );
    cleanup(&dir);
}

#[test]
fn postgres_seeds_take_their_valid_values_from_the_mother() {
    let (dir, output) = linea_project("canasta_mother_seeds", true);

    let siembra = read(
        &output,
        "contextos/linea/infraestructura/src/pruebas/siembra.rs",
    );
    assert!(
        siembra.contains("..MadreLinea::new().construir_props()"),
        "{siembra}"
    );
    assert!(!siembra.contains("puntos: 42"), "{siembra}");
    cleanup(&dir);
}
