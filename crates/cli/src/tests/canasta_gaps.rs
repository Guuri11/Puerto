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

// ── 20.7: a unique over several fields ───────────────────────────────────────

fn linea_fields() -> Vec<puerto_toml::Field> {
    vec![
        puerto_toml::parse_field_arg("jugador:String").unwrap(),
        puerto_toml::parse_field_arg("partido:String").unwrap(),
        puerto_toml::parse_field_arg("faltas:i64").unwrap(),
        puerto_toml::parse_field_arg("nota:opt:String").unwrap(),
    ]
}

fn scaffold_linea(name: &str, db: bool, shape: &scaffold::EntityShape) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("liga".into()), db, &dir).unwrap();
    scaffold::run_scaffold_with_options(
        "Linea",
        &output,
        &linea_fields(),
        OverwritePolicy::Ask,
        &scaffold::ScaffoldAccess::default(),
        shape,
    )
    .unwrap();
    (dir, output)
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

fn unique_shape(groups: &[&str]) -> scaffold::EntityShape {
    scaffold::EntityShape::from_args(&strings(groups), &[])
}

fn order_shape(keys: &[&str]) -> scaffold::EntityShape {
    scaffold::EntityShape::from_args(&[], &strings(keys))
}

#[test]
fn scaffold_records_a_unique_group_in_puerto_toml() {
    let (dir, output) = scaffold_linea(
        "canasta_unique_toml",
        false,
        &unique_shape(&["jugador,partido"]),
    );

    let config = puerto_toml::read(&output).unwrap();
    let linea = config.entity.iter().find(|e| e.name == "Linea").unwrap();
    assert_eq!(
        linea.uniques,
        vec![vec!["jugador".to_string(), "partido".to_string()]]
    );
    let toml = read(&output, "puerto.toml");
    assert!(toml.contains("unicos = [["), "{toml}");

    // A re-scaffold without --unico keeps what the block says.
    scaffold::run_scaffold("Linea", &output, &[], OverwritePolicy::Force).unwrap();
    let config = puerto_toml::read(&output).unwrap();
    let linea = config.entity.iter().find(|e| e.name == "Linea").unwrap();
    assert_eq!(linea.uniques.len(), 1);
    cleanup(&dir);
}

#[test]
fn validation_rejects_unique_groups_that_cannot_be_an_index() {
    let entity = |groups: &[&[&str]]| puerto_toml::Entity {
        name: "Linea".into(),
        fields: linea_fields(),
        uniques: groups
            .iter()
            .map(|g| g.iter().map(|f| f.to_string()).collect())
            .collect(),
        ..Default::default()
    };
    let errors_of = |groups: &[&[&str]]| crate::validation::validate_entity_shape(&entity(groups));

    assert!(errors_of(&[&["jugador", "partido"]]).is_empty());
    for (groups, expected) in [
        (&[&["jugador"][..]][..], "at least two"),
        (&[&["jugador", "equipo"][..]][..], "'equipo' is not a field"),
        (&[&["jugador", "jugador"][..]][..], "'jugador' twice"),
        (&[&["jugador", "nota"][..]][..], "'nota'"),
        (
            &[&["jugador", "partido"][..], &["partido", "jugador"][..]][..],
            "twice",
        ),
    ] {
        let errors = errors_of(groups);
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "{groups:?}: {errors:?}"
        );
    }
}

#[test]
fn scaffold_refuses_a_unique_group_naming_an_unknown_field_and_writes_nothing() {
    let dir = temp_dir("canasta_unique_refused");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("liga".into()), false, &dir).unwrap();

    let err = scaffold::run_scaffold_with_options(
        "Linea",
        &output,
        &linea_fields(),
        OverwritePolicy::Ask,
        &scaffold::ScaffoldAccess::default(),
        &unique_shape(&["jugador,equipo"]),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("'equipo' is not a field"), "{err}");
    assert!(!output.join("contextos/linea").exists());
    assert!(!read(&output, "puerto.toml").contains("Linea"));
    cleanup(&dir);
}

#[test]
fn a_unique_group_is_a_partial_index_and_the_in_memory_repository_enforces_it() {
    let (dir, output) = scaffold_linea(
        "canasta_unique_index",
        true,
        &unique_shape(&["jugador,partido"]),
    );

    let migration = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .find(|sql| sql.contains("CREATE TABLE lineas"))
        .unwrap();
    assert!(
        migration.contains(
            "CREATE UNIQUE INDEX lineas_jugador_partido_key ON lineas (jugador, partido) WHERE borrado = false;"
        ),
        "{migration}"
    );
    let pg = read(
        &output,
        "contextos/linea/infraestructura/src/repositorio.rs",
    );
    assert!(
        pg.contains("async fn deberia_rechazar_un_duplicado_de_jugador_y_partido(pool: PgPool)"),
        "{pg}"
    );
    cleanup(&dir);

    let (dir, output) = scaffold_linea(
        "canasta_unique_memory",
        false,
        &unique_shape(&["jugador,partido"]),
    );
    let memory = read(
        &output,
        "contextos/linea/infraestructura/src/repositorio.rs",
    );
    assert!(
        memory.contains("(fila.jugador == entidad.jugador && fila.partido == entidad.partido)"),
        "{memory}"
    );
    assert!(
        memory.contains("async fn deberia_rechazar_un_duplicado_de_jugador_y_partido()"),
        "{memory}"
    );
    // Two seeded rows must not collide on the group: its fields get a fresh value each.
    assert!(
        memory.contains("jugador: Uuid::new_v4().to_string(),"),
        "{memory}"
    );
    cleanup(&dir);
}

// ── 20.8: the order of a list ─────────────────────────────────────────────────

#[test]
fn scaffold_records_the_order_and_keeps_the_uniques_it_was_not_given() {
    let (dir, output) = scaffold_linea(
        "canasta_order_toml",
        false,
        &scaffold::EntityShape::from_args(&strings(&["jugador,partido"]), &strings(&["-faltas"])),
    );
    let linea = |output: &Path| {
        puerto_toml::read(output)
            .unwrap()
            .entity
            .into_iter()
            .find(|e| e.name == "Linea")
            .unwrap()
    };
    assert_eq!(linea(&output).order, strings(&["-faltas"]));

    // Only --orden: the uniques stay; nothing: both stay.
    scaffold::run_scaffold_with_options(
        "Linea",
        &output,
        &[],
        OverwritePolicy::Force,
        &scaffold::ScaffoldAccess::default(),
        &order_shape(&["jugador", "-faltas"]),
    )
    .unwrap();
    assert_eq!(linea(&output).order, strings(&["jugador", "-faltas"]));
    assert_eq!(linea(&output).uniques.len(), 1);
    scaffold::run_scaffold("Linea", &output, &[], OverwritePolicy::Force).unwrap();
    assert_eq!(linea(&output).order.len(), 2);
    cleanup(&dir);
}

#[test]
fn validation_rejects_an_order_that_names_no_sortable_field() {
    let entity = |keys: &[&str]| puerto_toml::Entity {
        name: "Linea".into(),
        fields: linea_fields(),
        order: strings(keys),
        ..Default::default()
    };
    let errors_of = |keys: &[&str]| crate::validation::validate_entity_shape(&entity(keys));

    assert!(errors_of(&["-faltas", "jugador"]).is_empty());
    for (keys, expected) in [
        (&["equipo"][..], "'equipo' is not a field"),
        (&["faltas", "-faltas"][..], "'faltas' twice"),
        (&["nota"][..], "'nota'"),
        (&["-"][..], "'-'"),
    ] {
        let errors = errors_of(keys);
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "{keys:?}: {errors:?}"
        );
    }
}

#[test]
fn the_order_reaches_postgres_and_the_in_memory_repository() {
    let shape = order_shape(&["-faltas", "jugador"]);
    let (dir, output) = scaffold_linea("canasta_order_pg", true, &shape);
    let pg = read(
        &output,
        "contextos/linea/infraestructura/src/repositorio.rs",
    );
    assert!(
        pg.contains("ORDER BY faltas DESC, jugador, creado_en DESC, id LIMIT $1 OFFSET $2"),
        "{pg}"
    );
    cleanup(&dir);

    let (dir, output) = scaffold_linea("canasta_order_memory", false, &shape);
    let memory = read(
        &output,
        "contextos/linea/infraestructura/src/repositorio.rs",
    );
    let compact: String = memory.split_whitespace().collect();
    assert!(
        compact.contains(
            "a.faltas.cmp(&b.faltas).reverse().then_with(||a.jugador.cmp(&b.jugador)).then_with(||b.creado_en.cmp(&a.creado_en))"
        ),
        "{memory}"
    );
    cleanup(&dir);
}
