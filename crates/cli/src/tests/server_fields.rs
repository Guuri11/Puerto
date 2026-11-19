// Fase 27.17: fields the server sets — `por_defecto` (a value on create) and `generado` (the
// domain makes it up). Neither is in what `crear` asks for.
use super::*;
use crate::commands::new::{NewOptions, new_project};

fn project(name: &str, web: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            no_db: true,
            no_web: !web,
            no_portada: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

fn read(base: &Path, path: &str) -> String {
    fs::read_to_string(base.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

const FIELDS: [&str; 3] = [
    "titular:String",
    "estado:EstadoEntrada:enum:Pendiente/Confirmada/Anulada",
    "codigo:CodigoEntrada:String!",
];

fn shape(defaults: &[&str], generated: &[&str]) -> scaffold::EntityShape {
    let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    scaffold::EntityShape::default().with_server_set(&strings(defaults), &strings(generated))
}

fn scaffold_entrada(output: &Path, shape: &scaffold::EntityShape) -> Result<(), String> {
    let fields: Vec<puerto_toml::Field> = FIELDS
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    scaffold::run_scaffold_with_options(
        "Entrada",
        output,
        &fields,
        OverwritePolicy::Force,
        &scaffold::ScaffoldAccess::default(),
        shape,
    )
    .map_err(|e| e.to_string())
}

fn field(output: &Path, name: &str) -> puerto_toml::Field {
    puerto_toml::read(output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Entrada")
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap()
}

#[test]
fn scaffold_records_the_fields_the_server_sets_and_a_rescaffold_keeps_them() {
    let (dir, output) = project("server_fields_manifest", false);

    scaffold_entrada(&output, &shape(&["estado=Pendiente"], &["codigo"])).unwrap();

    assert_eq!(
        field(&output, "estado").default.as_deref(),
        Some("Pendiente")
    );
    assert!(field(&output, "codigo").generated);
    let manifest = read(&output, "puerto.toml");
    assert!(
        manifest.contains("por_defecto = \"Pendiente\""),
        "{manifest}"
    );
    assert!(manifest.contains("generado = true"), "{manifest}");

    scaffold_entrada(&output, &scaffold::EntityShape::default()).unwrap();
    assert_eq!(
        field(&output, "estado").default.as_deref(),
        Some("Pendiente")
    );
    assert!(field(&output, "codigo").generated);
    cleanup(&dir);
}

#[test]
fn create_leaves_out_what_the_server_sets_and_update_keeps_the_generated_value() {
    let (dir, output) = project("server_fields_layers", false);

    scaffold_entrada(&output, &shape(&["estado=Pendiente"], &["codigo"])).unwrap();

    let dto = read(&output, "contextos/entrada/rest/src/dto.rs");
    let create_request = &dto[dto.find("pub struct PeticionCrearEntrada").unwrap()..];
    let create_request = &create_request[..create_request.find('}').unwrap()];
    assert!(create_request.contains("titular"), "{create_request}");
    assert!(
        !create_request.contains("estado") && !create_request.contains("codigo"),
        "{create_request}"
    );
    let update_request = &dto[dto.find("pub struct PeticionActualizarEntrada").unwrap()..];
    let update_request = &update_request[..update_request.find('}').unwrap()];
    assert!(update_request.contains("estado"), "{update_request}");
    assert!(!update_request.contains("codigo"), "{update_request}");

    let create = read(&output, "contextos/entrada/aplicacion/src/crear_entrada.rs");
    let params = &create[create.find("pub struct ParamsCrearEntrada").unwrap()..];
    let params = &params[..params.find('}').unwrap()];
    assert!(
        !params.contains("estado") && !params.contains("codigo"),
        "{params}"
    );
    assert!(create.contains("EstadoEntrada::Pendiente"), "{create}");
    assert!(create.contains("Entrada::generar_codigo()"), "{create}");

    let update = read(
        &output,
        "contextos/entrada/aplicacion/src/actualizar_entrada.rs",
    );
    let params = &update[update.find("pub struct ParamsActualizarEntrada").unwrap()..];
    let params = &params[..params.find('}').unwrap()];
    assert!(
        params.contains("estado") && !params.contains("codigo"),
        "{params}"
    );
    assert!(update.contains("entidad.codigo.clone()"), "{update}");

    let generated = read(&output, "contextos/entrada/dominio/src/generados.rs");
    assert!(
        generated.contains("pub fn generar_codigo() -> String"),
        "{generated}"
    );
    assert!(generated.contains("#[test]"), "{generated}");
    assert!(read(&output, "contextos/entrada/dominio/src/lib.rs").contains("pub mod generados;"));
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_a_server_set_field_it_cannot_set_before_writing() {
    let (dir, output) = project("server_fields_refusals", false);
    let manifest = read(&output, "puerto.toml");

    for (defaults, generated, expected) in [
        (&["nada=1"][..], &[][..], "nada"),
        (&["estado=Borrado"][..], &[][..], "Borrado"),
        (&["titular="][..], &[][..], "titular"),
        (&[][..], &["titular"][..], "titular"),
        (&[][..], &["estado"][..], "estado"),
        (&["codigo=ABC"][..], &["codigo"][..], "codigo"),
    ] {
        let err = scaffold_entrada(&output, &shape(defaults, generated)).unwrap_err();
        assert!(err.contains(expected), "{defaults:?} {generated:?}: {err}");
        assert_eq!(read(&output, "puerto.toml"), manifest);
    }
    assert!(!output.join("contextos/entrada").exists());
    cleanup(&dir);
}

#[test]
fn validate_rejects_a_generated_field_that_is_not_a_unique_string() {
    let dir = temp_dir("server_fields_validate");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "my-app"

[[entidad]]
nombre = "Entrada"
casos_de_uso = []

[[entidad.campos]]
nombre = "plazas"
tipo = "i64"
generado = true

[[entidad.campos]]
nombre = "activa"
tipo = "bool"
por_defecto = "quizas"
"#,
    )
    .unwrap();

    let err = validate::run_validate(&dir).unwrap_err().to_string();

    assert!(err.contains("plazas"), "{err}");
    assert!(err.contains("activa"), "{err}");
    cleanup(&dir);
}

#[test]
fn the_web_form_hides_a_defaulted_field_on_create_and_never_shows_a_generated_one() {
    let (dir, output) = project("server_fields_web", true);

    scaffold_entrada(&output, &shape(&["estado=Pendiente"], &["codigo"])).unwrap();

    let view = read(
        &output,
        "web/src/contextos/entrada/vistas/formulario-entrada.tsx",
    );
    assert!(view.contains("creando"), "{view}");
    assert!(view.contains("hidden={creando}"), "{view}");
    assert!(
        view.contains("Esquemas[\"PeticionActualizarEntrada\"]"),
        "{view}"
    );
    let values = &view[view
        .find("export interface ValoresFormularioEntrada")
        .unwrap()..];
    let values = &values[..values.find('}').unwrap()];
    assert!(
        values.contains("estado") && !values.contains("codigo"),
        "{values}"
    );
    let container = read(
        &output,
        "web/src/contextos/entrada/contenedores/formulario-entrada.tsx",
    );
    assert!(
        container.contains("creando={existente === null}"),
        "{container}"
    );
    let mother = read(&output, "web/src/pruebas/entrada.ts");
    let request = &mother[mother.find("peticion(").unwrap()..mother.find("aleatorio(").unwrap()];
    assert!(
        !request.contains("estado") && !request.contains("codigo"),
        "{request}"
    );
    let random = &mother[mother.find("aleatorio(").unwrap()..];
    assert!(
        random.contains("estado") && random.contains("codigo"),
        "{random}"
    );
    cleanup(&dir);
}
