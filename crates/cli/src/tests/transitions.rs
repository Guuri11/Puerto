// Fase 27.12: `puerto generar transicion <Entidad> <verbo>` — a state change from end to end.
use super::*;
use crate::commands::new::{NewOptions, new_project};
use crate::generators::consistency::auth_drift;
use crate::validation::validate_transition_verb;

fn project(name: &str, options: NewOptions) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            no_portada: true,
            ..options
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

/// No database, no web: the plain in-memory backend.
fn plain() -> NewOptions {
    NewOptions {
        no_db: true,
        no_web: true,
        ..Default::default()
    }
}

fn read(base: &Path, path: &str) -> String {
    fs::read_to_string(base.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn scaffold_producto(output: &Path) {
    scaffold::run_scaffold(
        "Producto",
        output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Force,
    )
    .unwrap();
}

fn transicion(
    output: &Path,
    verb: &str,
    event: Option<&str>,
    roles: &[&str],
) -> Result<(), String> {
    let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
    scaffold::run_generate_transition("Producto", verb, event, &roles, output)
        .map_err(|e| e.to_string())
}

fn producto(output: &Path) -> puerto_toml::Entity {
    puerto_toml::read(output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Producto")
        .unwrap()
}

// ── The layers ─────────────────────────────────────────────────────────────

#[test]
fn transition_with_events_writes_the_method_the_use_case_and_its_own_route() {
    let (dir, output) = project(
        "transition_events",
        NewOptions {
            db: true,
            events: Some("db".into()),
            no_web: true,
            ..Default::default()
        },
    );
    scaffold_producto(&output);

    transicion(&output, "cancelar", Some("producto_cancelado"), &[]).unwrap();

    let domain = read(&output, "contextos/producto/dominio/src/cancelar.rs");
    assert!(domain.contains("impl Producto {"), "{domain}");
    assert!(
        domain.contains("pub fn cancelar(&mut self) -> Result<EventoProducto, ErrorProducto>"),
        "{domain}"
    );
    assert!(domain.contains("EventoProducto::Cancelado(SnapshotProducto::from_entidad(self))"));
    assert!(domain.contains("Validacion(\"no_se_puede_cancelar\".into())"));
    assert!(domain.contains("#[ignore = \"escribe la regla\"]"));
    assert!(domain.contains("\"producto.validacion.no_se_puede_cancelar\""));
    assert!(read(&output, "contextos/producto/dominio/src/lib.rs").contains("pub mod cancelar;"));
    assert!(read(&output, "contextos/producto/dominio/src/eventos.rs").contains("Cancelado("));

    let use_case = read(
        &output,
        "contextos/producto/aplicacion/src/cancelar_producto.rs",
    );
    assert!(
        use_case.contains("pub struct ParamsCancelarProducto"),
        "{use_case}"
    );
    assert!(use_case.contains("pub id: IdProducto,"));
    assert!(
        !use_case.contains("actor:"),
        "no auth, no caller: {use_case}"
    );
    assert!(use_case.contains("let evento = entidad.cancelar()?;"));
    assert!(use_case.contains("self.uow_factory.guardar(&entidad, evento).await?;"));
    assert!(
        use_case.contains("\"producto_cancelado\""),
        "the test checks the event"
    );
    assert!(use_case.contains("#[ignore = \"escribe la regla\"]"));
    assert!(
        read(&output, "contextos/producto/aplicacion/src/lib.rs")
            .contains("pub mod cancelar_producto;")
    );

    let route = read(&output, "contextos/producto/rest/src/cancelar_producto.rs");
    assert!(
        route.contains("pub struct ApiCancelarProducto {"),
        "{route}"
    );
    assert!(route.contains("pub cancelar_producto: Arc<dyn CancelarProducto>,"));
    assert!(route.contains("path = \"/productos/:id/cancelar\""));
    assert!(route.contains("method = \"post\""));
    assert!(route.contains("pub enum RespuestaCancelarProducto {"));
    assert!(route.contains("pub fn from_status("));
    for status in ["200", "404", "422"] {
        assert!(
            route.contains(&format!("deberia_responder_{status}_")),
            "{status}: {route}"
        );
    }
    assert!(!route.contains("BearerAuth"), "no auth: {route}");
    assert!(
        read(&output, "contextos/producto/rest/src/lib.rs").contains("pub mod cancelar_producto;")
    );
    assert!(
        !read(&output, "contextos/producto/rest/src/rutas.rs").contains("cancelar_producto"),
        "rutas.rs is the user's: ApiProducto gets no field"
    );

    let entity = producto(&output);
    assert_eq!(entity.transitions, ["cancelar"]);
    assert!(entity.use_cases.contains(&"cancelar_producto".to_string()));
    assert_eq!(entity.events, ["producto_cancelado"]);

    let wiring = wiring(&output);
    assert!(
        wiring.contains("CancelarProductoImpl"),
        "cableado builds it"
    );
    assert!(wiring.contains("use rest_producto::cancelar_producto::ApiCancelarProducto;"));
    assert!(wiring.contains("ApiCancelarProducto {"));
    assert!(
        wiring.contains("(api_producto, api_cancelar_producto)"),
        "{wiring}"
    );
    let api_literal = wiring
        .lines()
        .find(|l| l.contains("let api_producto = ApiProducto"))
        .unwrap();
    assert!(!api_literal.contains("cancelar_producto"), "{api_literal}");
    cleanup(&dir);
}

#[test]
fn transition_with_events_but_no_event_of_its_own_emits_actualizado() {
    let (dir, output) = project(
        "transition_events_updated",
        NewOptions {
            db: true,
            events: Some("db".into()),
            no_web: true,
            ..Default::default()
        },
    );
    scaffold_producto(&output);

    transicion(&output, "archivar", None, &[]).unwrap();

    let domain = read(&output, "contextos/producto/dominio/src/archivar.rs");
    assert!(domain.contains("EventoProducto::Actualizado(SnapshotProducto::from_entidad(self))"));
    let use_case = read(
        &output,
        "contextos/producto/aplicacion/src/archivar_producto.rs",
    );
    assert!(use_case.contains("\"producto_actualizado\""), "{use_case}");
    assert!(producto(&output).events.is_empty());
    cleanup(&dir);
}

#[test]
fn transition_without_events_returns_nothing_and_saves_with_the_repository() {
    let (dir, output) = project("transition_plain", plain());
    scaffold_producto(&output);

    transicion(&output, "archivar", None, &[]).unwrap();

    let domain = read(&output, "contextos/producto/dominio/src/archivar.rs");
    assert!(
        domain.contains("pub fn archivar(&mut self) -> Result<(), ErrorProducto>"),
        "{domain}"
    );
    assert!(!domain.contains("Evento"), "{domain}");
    let use_case = read(
        &output,
        "contextos/producto/aplicacion/src/archivar_producto.rs",
    );
    assert!(use_case.contains("entidad.archivar()?;"), "{use_case}");
    assert!(use_case.contains("self.repositorio.guardar(&entidad).await?;"));
    assert!(!use_case.contains("uow_factory"));
    cleanup(&dir);
}

#[test]
fn transition_in_an_auth_project_takes_the_caller_and_requires_its_roles() {
    let (dir, output) = project(
        "transition_auth",
        NewOptions {
            auth: Some("oidc".into()),
            ..plain()
        },
    );
    scaffold_producto(&output);

    transicion(&output, "cancelar", None, &["admin"]).unwrap();

    let use_case = read(
        &output,
        "contextos/producto/aplicacion/src/cancelar_producto.rs",
    );
    assert!(use_case.contains("pub actor: Principal,"), "{use_case}");
    let route = read(&output, "contextos/producto/rest/src/cancelar_producto.rs");
    assert!(route.contains("#[requiere_rol(\"admin\")]"), "{route}");
    assert!(route.contains("auth: BearerAuth"));
    assert!(route.contains("actor: auth.0"));
    assert!(route.contains("deberia_responder_401_"));
    assert!(route.contains("deberia_responder_403_"));
    assert_eq!(
        producto(&output).roles.get("cancelar_producto"),
        Some(&vec!["admin".to_string()])
    );
    let config = puerto_toml::read(&output).unwrap();
    assert!(
        auth_drift(&output, &config).is_empty(),
        "{:?}",
        auth_drift(&output, &config)
    );

    // The drift check reads the transition's own file.
    let path = output.join("contextos/producto/rest/src/cancelar_producto.rs");
    fs::write(&path, route.replace("    #[requiere_rol(\"admin\")]\n", "")).unwrap();
    let drift = auth_drift(&output, &config);
    assert!(
        drift
            .iter()
            .any(|l| l.contains("cancelar_producto") && l.contains("admin")),
        "{drift:?}"
    );
    cleanup(&dir);
}

// ── What it refuses ────────────────────────────────────────────────────────

#[test]
fn transition_refuses_what_it_cannot_write_before_writing_anything() {
    let (dir, output) = project("transition_refusals", plain());
    scaffold_producto(&output);
    scaffold::run_use_case("Producto", "publicar_producto", &output).unwrap();
    let manifest = read(&output, "puerto.toml");

    for (verb, event, roles, expected) in [
        ("crear", None, &[][..], "crear"),
        ("Cancelar", None, &[][..], "snake_case"),
        ("modelo", None, &[][..], "modelo"),
        ("match", None, &[][..], "palabra clave"),
        ("publicar", None, &[][..], "ya existe"),
        ("cancelar", Some("producto_cancelado"), &[][..], "eventos"),
        ("cancelar", None, &["admin"][..], "auth"),
    ] {
        let err = transicion(&output, verb, event, roles).unwrap_err();
        assert!(err.contains(expected), "{verb}: {err}");
        assert_eq!(
            read(&output, "puerto.toml"),
            manifest,
            "{verb}: puerto.toml untouched"
        );
    }
    let saludo = scaffold::run_generate_transition("Saludo", "archivar", None, &[], &output);
    assert!(saludo.unwrap_err().to_string().contains("CRUD"));
    let undeclared = scaffold::run_generate_transition("Nada", "archivar", None, &[], &output);
    assert!(undeclared.unwrap_err().to_string().contains("puerto.toml"));
    assert!(
        !output
            .join("contextos/producto/dominio/src/cancelar.rs")
            .exists()
    );

    transicion(&output, "archivar", None, &[]).unwrap();
    let again = transicion(&output, "archivar", None, &[]).unwrap_err();
    assert!(again.contains("ya existe"), "{again}");
    cleanup(&dir);
}

#[test]
fn transition_verbs_are_snake_case_and_never_a_crud_verb_a_method_or_a_domain_module() {
    for verb in ["cancelar", "agotar", "poner_a_la_venta"] {
        assert!(validate_transition_verb(verb).is_ok(), "{verb}");
    }
    for verb in [
        "crear",
        "obtener",
        "listar",
        "actualizar",
        "borrar",
        "validar",
        "new",
        "modelo",
        "errores",
        "repositorio",
        "valores",
        "eventos",
        "unit_of_work",
        "pruebas",
        "match",
        "Cancelar",
        "1cancelar",
    ] {
        assert!(validate_transition_verb(verb).is_err(), "{verb}");
    }
}

// ── The manifest ───────────────────────────────────────────────────────────

#[test]
fn rescaffold_with_force_keeps_the_transition() {
    let (dir, output) = project("transition_rescaffold", plain());
    scaffold_producto(&output);
    transicion(&output, "archivar", None, &[]).unwrap();
    let route = read(&output, "contextos/producto/rest/src/archivar_producto.rs");

    scaffold_producto(&output);

    assert!(read(&output, "contextos/producto/dominio/src/lib.rs").contains("pub mod archivar;"));
    assert!(
        read(&output, "contextos/producto/rest/src/lib.rs").contains("pub mod archivar_producto;")
    );
    assert_eq!(
        read(&output, "contextos/producto/rest/src/archivar_producto.rs"),
        route
    );
    assert!(!read(&output, "contextos/producto/rest/src/rutas.rs").contains("archivar_producto"));
    assert!(wiring(&output).contains("ApiArchivarProducto {"));
    assert_eq!(producto(&output).transitions, ["archivar"]);
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn validate_reports_a_transition_without_its_use_case_or_its_files() {
    let (dir, output) = project("transition_validate", plain());
    scaffold_producto(&output);
    transicion(&output, "archivar", None, &[]).unwrap();
    assert!(validate::run_validate(&output).is_ok());

    fs::remove_file(output.join("contextos/producto/dominio/src/archivar.rs")).unwrap();
    let err = validate::run_validate(&output).unwrap_err().to_string();
    assert!(
        err.contains("contextos/producto/dominio/src/archivar.rs"),
        "{err}"
    );

    let manifest = read(&output, "puerto.toml");
    fs::write(
        output.join("puerto.toml"),
        manifest
            .replace("    \"archivar_producto\",\n", "")
            .replace(
                "transiciones = [\"archivar\"]",
                "transiciones = [\"archivar\", \"crear\"]",
            ),
    )
    .unwrap();
    let err = validate::run_validate(&output).unwrap_err().to_string();
    assert!(err.contains("archivar_producto"), "its use case: {err}");
    assert!(err.contains("'crear'"), "a CRUD verb: {err}");
    cleanup(&dir);
}

// ── The web ────────────────────────────────────────────────────────────────

const BUTTON: &str = "web/src/contextos/producto/contenedores/archivar-producto.tsx";
const BUTTON_TEST: &str = "web/src/contextos/producto/contenedores/archivar-producto.test.tsx";
const DETAIL: &str = "web/src/contextos/producto/contenedores/detalle-producto.tsx";
const MESSAGES: &str = "web/src/contextos/producto/mensajes.ts";

#[test]
fn transition_in_a_web_project_writes_its_button_and_puts_it_in_the_detail() {
    let (dir, output) = project(
        "transition_web",
        NewOptions {
            no_db: true,
            ..Default::default()
        },
    );
    scaffold_producto(&output);

    transicion(&output, "archivar", None, &[]).unwrap();

    let button = read(&output, BUTTON);
    assert!(
        button.contains("export function BotonArchivarProducto({ id, alTerminar }"),
        "{button}"
    );
    assert!(button.contains("api.POST(\"/productos/{id}/archivar\""));
    assert!(button.contains("mensaje=\"¿Archivar producto?\""));
    assert!(button.contains("textoConfirmar=\"Archivar producto\""));
    assert!(button.contains("textoCancelar=\"Volver\""));
    assert!(button.contains("aProblema(codigoError(fallo)).texto"));
    assert!(!button.contains("useAuth"), "no auth: {button}");

    let test = read(&output, BUTTON_TEST);
    assert!(test.contains("coleccion(\"/productos\", \"producto\", [sembrado], { archivar: {} })"));
    assert!(test.contains("const codigo = \"producto.validacion.no_se_puede_archivar\";"));
    assert!(test.contains("\"POST /productos/{id}/archivar\": codigo"));
    assert!(test.contains("aProblema(codigo).texto"));

    let detail = read(&output, DETAIL);
    assert!(
        detail.contains("import { BotonArchivarProducto } from \"./archivar-producto\";"),
        "{detail}"
    );
    assert!(
        detail.contains(
            "<div className=\"card-actions justify-end\">\n              <BotonArchivarProducto id={id} alTerminar={producto.recargar} />\n"
        ),
        "{detail}"
    );
    assert!(
        read(&output, MESSAGES)
            .contains("\"producto.validacion.no_se_puede_archivar\": \"No se puede archivar\",")
    );
    let dialog = read(&output, "web/src/compartido/ui/dialogo-confirmar.tsx");
    assert!(dialog.contains("textoCancelar = \"Cancelar\""), "{dialog}");
    assert!(dialog.contains("children"));

    // What the command patched is what `generar web --forzar` renders from the manifest.
    let files = [BUTTON, BUTTON_TEST, DETAIL, MESSAGES].map(|f| read(&output, f));
    crate::generators::web::run_generate_web("Producto", &output, OverwritePolicy::Force).unwrap();
    for (path, before) in [BUTTON, BUTTON_TEST, DETAIL, MESSAGES].iter().zip(files) {
        assert_eq!(read(&output, path), before, "{path}");
    }
    cleanup(&dir);
}

#[test]
fn transition_button_of_an_auth_project_hides_from_who_cannot_use_it() {
    let (dir, output) = project(
        "transition_web_auth",
        NewOptions {
            no_db: true,
            auth: Some("oidc".into()),
            ..Default::default()
        },
    );
    scaffold_producto(&output);

    transicion(&output, "archivar", None, &["admin"]).unwrap();
    scaffold::run_generate_transition("Producto", "destacar", None, &[], &output).unwrap();

    let with_role = read(&output, BUTTON);
    assert!(
        with_role.contains("if (!puede(\"admin\")) return null;"),
        "{with_role}"
    );
    let test = read(&output, BUTTON_TEST);
    assert!(test.contains("no debería ofrecer"), "{test}");
    let without_role = read(
        &output,
        "web/src/contextos/producto/contenedores/destacar-producto.tsx",
    );
    assert!(
        without_role.contains("if (usuario === null) return null;"),
        "{without_role}"
    );
    assert!(read(&output, "web/src/generado/roles.ts").contains("\"admin\""));
    cleanup(&dir);
}

#[test]
fn transition_on_an_entity_without_screens_writes_no_web() {
    let (dir, output) = project("transition_no_web", plain());
    scaffold_producto(&output);

    transicion(&output, "archivar", None, &[]).unwrap();

    assert!(!output.join(BUTTON).exists());
    cleanup(&dir);
}
