use super::*;
use crate::commands::new::{NewOptions, new_project};
use crate::generators::blocks;
use std::collections::BTreeMap;

// ── Fase 26: `puerto generar web-bloque` (ADR-0028) ────────────────────────────────────────────

fn new_web_project(name: &str, options: NewOptions) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(Some(name.replace('_', "-")), options, Some(dir.clone())).unwrap();
    (dir, output)
}

fn plain(name: &str) -> (PathBuf, PathBuf) {
    new_web_project(
        name,
        NewOptions {
            no_db: true,
            ..Default::default()
        },
    )
}

fn with_auth(name: &str) -> (PathBuf, PathBuf) {
    new_web_project(
        name,
        NewOptions {
            no_db: true,
            auth: Some("oidc".into()),
            ..Default::default()
        },
    )
}

/// Every file under `web/src`, with its content: what a block may change.
fn web_sources(output: &Path) -> BTreeMap<PathBuf, String> {
    fn walk(dir: &Path, root: &Path, files: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, files);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                files.insert(rel, fs::read_to_string(&path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(&output.join("web/src"), output, &mut files);
    files
}

fn changed(before: &BTreeMap<PathBuf, String>, after: &BTreeMap<PathBuf, String>) -> Vec<String> {
    after
        .iter()
        .filter(|(path, content)| before.get(*path) != Some(content))
        .map(|(path, _)| path.display().to_string())
        .collect()
}

#[test]
fn the_block_catalog_lists_every_block_and_what_it_needs() {
    let catalog = blocks::catalog_text();

    for id in [
        "acceso/dividido",
        "acceso/centrado",
        "marco/lateral",
        "marco/superior",
        "panel/tarjetas",
        "panel/resumen",
        "perfil/tarjeta",
        "perfil/banda",
        "tema/puerto",
        "tema/bosque",
        "tema/ciruela",
        "tema/coral",
        "tema/pizarra",
        "tema/miel",
    ] {
        assert!(
            catalog.contains(id),
            "{id} no está en el catálogo:\n{catalog}"
        );
    }
    let line = |id: &str| catalog.lines().find(|l| l.contains(id)).unwrap();
    assert!(line("acceso/centrado").contains("(auth)"));
    assert!(line("perfil/banda").contains("(auth)"));
    assert!(!line("marco/superior").contains("(auth)"));
}

#[test]
fn a_default_block_on_a_new_project_changes_nothing() {
    let (dir, output) = plain("blocks_default_plain");
    let before = web_sources(&output);

    for id in ["marco/lateral", "panel/tarjetas", "tema/puerto"] {
        let written = blocks::run_install(id, &output, OverwritePolicy::Ask)
            .unwrap()
            .written;
        assert!(written.is_empty(), "{id} escribió {written:?}");
    }

    assert_eq!(
        changed(&before, &web_sources(&output)),
        Vec::<String>::new()
    );
    cleanup(&dir);
}

#[test]
fn a_default_auth_block_on_a_new_auth_project_changes_nothing() {
    let (dir, output) = with_auth("blocks_default_auth");
    let before = web_sources(&output);

    for id in ["acceso/dividido", "marco/lateral", "perfil/tarjeta"] {
        let written = blocks::run_install(id, &output, OverwritePolicy::Ask)
            .unwrap()
            .written;
        assert!(written.is_empty(), "{id} escribió {written:?}");
    }

    assert_eq!(
        changed(&before, &web_sources(&output)),
        Vec::<String>::new()
    );
    cleanup(&dir);
}

#[test]
fn another_variant_replaces_only_its_slot_and_only_with_forzar() {
    let (dir, output) = plain("blocks_forzar");
    let before = web_sources(&output);

    let refused = blocks::run_install("panel/resumen", &output, OverwritePolicy::Ask);
    assert!(
        refused.is_err(),
        "sin terminal y sin --forzar no debería escribir"
    );
    assert_eq!(
        changed(&before, &web_sources(&output)),
        Vec::<String>::new()
    );

    let written = blocks::run_install("panel/resumen", &output, OverwritePolicy::Force)
        .unwrap()
        .written;
    assert_eq!(written, vec![PathBuf::from("web/src/paginas/panel.tsx")]);
    let after = web_sources(&output);
    assert_eq!(changed(&before, &after), vec!["web/src/paginas/panel.tsx"]);
    assert!(after[Path::new("web/src/paginas/panel.tsx")].contains("export function PaginaPanel"));

    // Installed already: nothing to protect, nothing to write.
    let again = blocks::run_install("panel/resumen", &output, OverwritePolicy::Ask).unwrap();
    assert!(again.written.is_empty());
    assert_eq!(
        again.files,
        vec![PathBuf::from("web/src/paginas/panel.tsx")]
    );
    cleanup(&dir);
}

#[test]
fn an_auth_block_is_refused_in_a_project_without_auth() {
    let (dir, output) = plain("blocks_auth_refused");

    let err = blocks::run_install("acceso/centrado", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();

    assert!(err.contains("auth"), "{err}");
    assert!(
        !output
            .join("web/src/compartido/ui/marco-acceso.tsx")
            .exists()
    );
    cleanup(&dir);
}

#[test]
fn blocks_need_a_project_with_web() {
    let (dir, output) = new_web_project(
        "blocks_no_web",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    let err = blocks::run_install("panel/resumen", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();

    assert!(err.contains("web"), "{err}");
    cleanup(&dir);
}

#[test]
fn an_unknown_block_names_the_variants_of_its_slot_or_the_slots() {
    let (dir, output) = plain("blocks_unknown");

    let variant = blocks::run_install("panel/graficas", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();
    assert!(
        variant.contains("panel/tarjetas") && variant.contains("panel/resumen"),
        "{variant}"
    );

    let slot = blocks::run_install("grafica/barras", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();
    assert!(slot.contains("panel") && slot.contains("tema"), "{slot}");
    cleanup(&dir);
}

#[test]
fn the_top_frame_shows_the_user_only_with_auth() {
    let (plain_dir, plain_output) = plain("blocks_top_plain");
    let (auth_dir, auth_output) = with_auth("blocks_top_auth");

    for output in [&plain_output, &auth_output] {
        blocks::run_install("marco/superior", output, OverwritePolicy::Force).unwrap();
    }

    let frame = |output: &Path| fs::read_to_string(output.join("web/src/compartido/ui/marco.tsx"));
    let plain_frame = frame(&plain_output).unwrap();
    let auth_frame = frame(&auth_output).unwrap();
    assert!(plain_frame.contains("export interface EntradaMenu"));
    assert!(plain_frame.contains("menu-horizontal"));
    assert!(!plain_frame.contains("MenuUsuario"));
    assert!(auth_frame.contains("<MenuUsuario />"));
    cleanup(&plain_dir);
    cleanup(&auth_dir);
}

#[test]
fn every_theme_brings_daisyui_a_light_default_and_a_dark_for_the_system() {
    let (dir, output) = plain("blocks_themes");
    let themes: Vec<&str> = blocks::CATALOG
        .iter()
        .map(|b| b.id)
        .filter(|id| id.starts_with("tema/"))
        .collect();
    assert_eq!(themes.len(), 6, "{themes:?}");

    for id in themes {
        blocks::run_install(id, &output, OverwritePolicy::Force).unwrap();
        let theme = fs::read_to_string(output.join("web/src/marca/tema.css")).unwrap();
        assert!(theme.contains("@plugin \"daisyui\";"), "{id}");
        assert!(theme.contains("name: \"light\";\n  default: true;"), "{id}");
        assert!(
            theme.contains("name: \"dark\";\n  prefersdark: true;"),
            "{id}"
        );
    }
    cleanup(&dir);
}

// ── 26.3: la portada, elegida en `puerto nuevo` ────────────────────────────────────────────────

fn with_portada(name: &str, options: NewOptions) -> (PathBuf, PathBuf) {
    new_web_project(
        name,
        NewOptions {
            no_db: true,
            portada: true,
            ..options
        },
    )
}

fn read(output: &Path, file: &str) -> String {
    fs::read_to_string(output.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn new_with_portada_puts_it_at_the_root_and_the_app_under_panel() {
    let (dir, output) = with_portada("portada_default", NewOptions::default());

    let manifest = read(&output, "puerto.toml");
    assert!(manifest.contains("portada = true"), "{manifest}");
    assert!(manifest.contains("ruta_app = \"/panel\""), "{manifest}");
    assert!(
        read(&output, "web/src/generado/rutas.tsx").contains("export const rutaApp = \"/panel\";")
    );
    let main = read(&output, "web/src/main.tsx");
    for expected in [
        "<Route path=\"/\">",
        "<PaginaPortada />",
        "<Route path={rutaApp} nest>",
        "<Marco menu={entradasMenu}>",
        "<PaginaNoEncontrada />",
    ] {
        assert!(main.contains(expected), "main.tsx sin {expected}:\n{main}");
    }
    let portada = read(&output, "web/src/paginas/portada.tsx");
    assert!(portada.contains("export function PaginaPortada"));
    assert!(portada.contains("rutaApp"));
    assert!(output.join("web/src/paginas/portada.test.tsx").exists());
    cleanup(&dir);
}

#[test]
fn new_with_portada_takes_the_route_of_the_app() {
    let (dir, output) = with_portada(
        "portada_route",
        NewOptions {
            app_path: Some("/app".into()),
            ..Default::default()
        },
    );

    assert!(read(&output, "puerto.toml").contains("ruta_app = \"/app\""));
    assert!(
        read(&output, "web/src/generado/rutas.tsx").contains("export const rutaApp = \"/app\";")
    );
    cleanup(&dir);
}

#[test]
fn new_without_portada_keeps_the_app_at_the_root() {
    let (dir, output) = plain("portada_none");

    let manifest = read(&output, "puerto.toml");
    assert!(
        !manifest.contains("portada =") && !manifest.contains("ruta_app"),
        "{manifest}"
    );
    assert!(read(&output, "web/src/generado/rutas.tsx").contains("export const rutaApp = \"/\";"));
    let template_main = crate::TEMPLATE_DIR
        .get_file("web/src/main.tsx")
        .and_then(|f| f.contents_utf8())
        .unwrap();
    assert_eq!(read(&output, "web/src/main.tsx"), template_main);
    assert!(!output.join("web/src/paginas/portada.tsx").exists());
    cleanup(&dir);
}

#[test]
fn new_with_portada_and_auth_keeps_the_access_pages_outside_the_app() {
    let (dir, output) = with_portada(
        "portada_auth",
        NewOptions {
            auth: Some("oidc".into()),
            ..Default::default()
        },
    );

    let main = read(&output, "web/src/main.tsx");
    for expected in [
        "<ProveedorAuth>",
        "<PaginaPortada />",
        "<Route path=\"/auth/callback\">",
        "<MarcoAcceso>",
        "<Route path={rutaApp} nest>",
    ] {
        assert!(main.contains(expected), "main.tsx sin {expected}:\n{main}");
    }
    cleanup(&dir);
}

#[test]
fn signing_in_without_a_next_page_goes_to_the_app() {
    let (dir, output) = new_web_project(
        "portada_login",
        NewOptions {
            db: true,
            auth: Some("local".into()),
            portada: true,
            ..Default::default()
        },
    );

    let login = read(&output, "web/src/paginas/login.tsx");
    assert!(login.contains(": rutaApp;"), "{login}");
    assert!(read(&output, "web/src/paginas/login.test.tsx").contains("toBe(rutaApp)"));
    cleanup(&dir);
}

#[test]
fn new_refuses_a_portada_without_web_or_a_route_that_is_not_one() {
    for (name, options) in [
        (
            "portada_no_web",
            NewOptions {
                no_db: true,
                no_web: true,
                portada: true,
                ..Default::default()
            },
        ),
        (
            "portada_bad_root",
            NewOptions {
                no_db: true,
                portada: true,
                app_path: Some("/".into()),
                ..Default::default()
            },
        ),
        (
            "portada_bad_case",
            NewOptions {
                no_db: true,
                portada: true,
                app_path: Some("/Panel".into()),
                ..Default::default()
            },
        ),
        (
            "portada_bad_slash",
            NewOptions {
                no_db: true,
                portada: true,
                app_path: Some("panel".into()),
                ..Default::default()
            },
        ),
    ] {
        let dir = temp_dir(name);
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let result = new_project(Some(name.replace('_', "-")), options, Some(dir.clone()));
        assert!(result.is_err(), "{name} debería rechazarse");
        cleanup(&dir);
    }
}

#[test]
fn the_welcome_portada_is_a_block_of_projects_with_portada_only() {
    let (dir, output) = with_portada("portada_block", NewOptions::default());
    let written = blocks::run_install("portada/bienvenida", &output, OverwritePolicy::Ask)
        .unwrap()
        .written;
    assert!(written.is_empty(), "{written:?}");
    cleanup(&dir);

    let (dir, output) = plain("portada_block_refused");
    let err = blocks::run_install("portada/bienvenida", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();
    assert!(err.contains("portada"), "{err}");
    cleanup(&dir);
}

#[test]
fn validar_checks_the_portada_and_the_route_of_the_app() {
    let (dir, output) = with_portada("portada_validate", NewOptions::default());
    assert!(validate::run_validate(&output).is_ok());

    fs::remove_file(output.join("web/src/paginas/portada.tsx")).unwrap();
    let missing = validate::run_validate(&output).unwrap_err().to_string();
    assert!(missing.contains("portada.tsx"), "{missing}");
    cleanup(&dir);

    let (dir, output) = plain("portada_validate_route");
    let manifest =
        read(&output, "puerto.toml").replace("web = true", "web = true\nruta_app = \"/panel\"");
    fs::write(output.join("puerto.toml"), manifest).unwrap();
    let orphan = validate::run_validate(&output).unwrap_err().to_string();
    assert!(orphan.contains("ruta_app"), "{orphan}");
    cleanup(&dir);
}

// ── 26.4: las secciones de marketing ───────────────────────────────────────────────────────────

const SECTIONS: [&str; 6] = [
    "cabecera",
    "caracteristicas",
    "precios",
    "preguntas",
    "llamada",
    "pie",
];

#[test]
fn the_product_portada_brings_every_section_in_its_own_file() {
    let (dir, output) = with_portada("portada_producto", NewOptions::default());

    let refused = blocks::run_install("portada/producto", &output, OverwritePolicy::Ask);
    assert!(
        refused.is_err(),
        "pisaría la portada de bienvenida sin --forzar"
    );

    let written = blocks::run_install("portada/producto", &output, OverwritePolicy::Force)
        .unwrap()
        .written;
    assert_eq!(written.len(), 1 + SECTIONS.len(), "{written:?}");
    let page = read(&output, "web/src/paginas/portada.tsx");
    for section in SECTIONS {
        let file = format!("web/src/portada/{section}.tsx");
        let component = format!(
            "Seccion{}",
            crate::generators::naming::to_pascal_case(section)
        );
        assert!(read(&output, &file).contains(&format!("export function {component}")));
        assert!(
            page.contains(&format!("<{component} />")),
            "la portada no monta {component}"
        );
    }
    cleanup(&dir);
}

#[test]
fn a_section_replaces_its_file_of_the_product_portada_only() {
    let (dir, output) = with_portada("portada_seccion", NewOptions::default());

    let err = blocks::run_install("cabecera/dividida", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();
    assert!(err.contains("portada/producto"), "{err}");

    blocks::run_install("portada/producto", &output, OverwritePolicy::Force).unwrap();
    let default = blocks::run_install("cabecera/centrada", &output, OverwritePolicy::Ask)
        .unwrap()
        .written;
    assert!(default.is_empty(), "{default:?}");
    let before = web_sources(&output);
    blocks::run_install("cabecera/dividida", &output, OverwritePolicy::Force).unwrap();
    assert_eq!(
        changed(&before, &web_sources(&output)),
        vec!["web/src/portada/cabecera.tsx"]
    );
    cleanup(&dir);
}

#[test]
fn the_sections_need_a_project_with_portada() {
    let (dir, output) = plain("portada_secciones_sin");

    let err = blocks::run_install("precios/planes", &output, OverwritePolicy::Force)
        .unwrap_err()
        .to_string();

    assert!(err.contains("portada"), "{err}");
    cleanup(&dir);
}

// ── 27.20: the escaparate, a public list on the portada ────────────────────

fn escaparate_project(name: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = with_portada(
        name,
        NewOptions {
            auth: Some("oidc".into()),
            ..Default::default()
        },
    );
    blocks::run_install("portada/producto", &output, OverwritePolicy::Force).unwrap();
    let access = |public: &[&str]| {
        let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        scaffold::ScaffoldAccess::from_args(&strings(public), &[]).unwrap()
    };
    let fields = |args: &[&str]| -> Vec<puerto_toml::Field> {
        args.iter()
            .map(|a| puerto_toml::parse_field_arg(a).unwrap())
            .collect()
    };
    let scaffold = |name: &str, fields: &[puerto_toml::Field], public: &[&str]| {
        scaffold::run_scaffold_with_options(
            name,
            &output,
            fields,
            OverwritePolicy::Force,
            &access(public),
            &scaffold::EntityShape::default(),
        )
        .unwrap();
    };
    scaffold(
        "Sala",
        &fields(&["nombre:String"]),
        &["listar_salas", "obtener_sala"],
    );
    scaffold(
        "Concierto",
        &fields(&[
            "titulo:String",
            "sala_id:ref:Sala",
            "estado:EstadoConcierto:enum:AlaVenta/Agotado",
            "precio:Precio:dinero",
            "contacto:String",
            "lote:Uuid",
            "sala_reserva_id:ref:opt:Sala",
            "sala_ensayo_id:ref:Sala",
        ]),
        &["listar_conciertos", "obtener_concierto"],
    );
    scaffold("Entrada", &fields(&["titular:String"]), &[]);
    (dir, output)
}

#[test]
fn the_escaparate_shows_the_first_of_a_public_list_and_joins_the_portada() {
    let (dir, output) = escaparate_project("escaparate_install");

    let installed = blocks::run_install_for(
        "escaparate/tarjetas",
        Some("Concierto"),
        &output,
        OverwritePolicy::Ask,
    )
    .unwrap();

    assert!(installed.missing.is_empty(), "{:?}", installed.missing);
    let section = read(&output, "web/src/portada/escaparate.tsx");
    assert!(
        section.contains("export function SeccionEscaparate()"),
        "{section}"
    );
    assert!(
        section.contains("api.GET(\"/conciertos\", PRIMEROS)"),
        "{section}"
    );
    assert!(
        section.contains("{concierto.titulo}"),
        "the title: {section}"
    );
    assert!(
        section.contains("etiquetasSala.get(concierto.salaId) ?? concierto.salaId"),
        "{section}"
    );
    assert!(
        section.contains("TEXTOS_ESTADO[concierto.estado]"),
        "{section}"
    );
    assert!(section.contains("euros(concierto.precio)"), "{section}");
    assert_eq!(
        section.matches("api.GET(\"/salas\"").count(),
        1,
        "one load per target: {section}"
    );
    assert!(
        !section.contains("concierto.lote"),
        "a bare Uuid: {section}"
    );
    assert!(
        !section.contains("salaReservaId"),
        "an optional reference: {section}"
    );
    assert!(read(&output, "web/src/portada/escaparate.test.tsx").contains("SeccionEscaparate"));
    let page = read(&output, "web/src/paginas/portada.tsx");
    assert!(
        page.contains("<SeccionCabecera />\n        <SeccionEscaparate />\n"),
        "{page}"
    );
    assert!(
        page.contains("import { SeccionEscaparate } from \"../portada/escaparate\";"),
        "{page}"
    );
    let page_test = read(&output, "web/src/paginas/portada.test.tsx");
    assert!(
        page_test.contains("renderizarPantalla(<PaginaPortada />, { api: apiFalsa([]) })"),
        "{page_test}"
    );

    // Again: nothing changes, and the page keeps one section.
    blocks::run_install_for(
        "escaparate/tarjetas",
        Some("Concierto"),
        &output,
        OverwritePolicy::Ask,
    )
    .unwrap();
    assert_eq!(
        read(&output, "web/src/paginas/portada.tsx")
            .matches("<SeccionEscaparate />")
            .count(),
        1
    );
    assert!(blocks::catalog_text().contains("escaparate/tarjetas"));
    cleanup(&dir);
}

#[test]
fn the_escaparate_needs_an_entity_whose_list_is_public() {
    let (dir, output) = escaparate_project("escaparate_refusals");

    let err = |entity: Option<&str>| {
        blocks::run_install_for("escaparate/tarjetas", entity, &output, OverwritePolicy::Ask)
            .unwrap_err()
            .to_string()
    };

    assert!(err(None).contains("<Entidad>"), "{}", err(None));
    assert!(
        err(Some("Entrada")).contains("no es pública"),
        "{}",
        err(Some("Entrada"))
    );
    assert!(
        err(Some("Nada")).contains("puerto.toml"),
        "{}",
        err(Some("Nada"))
    );
    assert!(!output.join("web/src/portada/escaparate.tsx").exists());
    cleanup(&dir);
}
