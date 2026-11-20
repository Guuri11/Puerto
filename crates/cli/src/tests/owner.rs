// Fase 27.18 (a): the owner of a row — `crear` sets it from the caller, `obtener`, `actualizar`
// and `borrar` let through only its owner or who holds every role of the route.
use super::*;
use crate::commands::new::{NewOptions, new_project};

fn auth_project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            no_db: true,
            no_web: true,
            auth: Some("oidc".into()),
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

fn scaffold_entrada(
    output: &Path,
    owner: &str,
    roles: &[&str],
    public: &[&str],
) -> Result<(), String> {
    let fields: Vec<puerto_toml::Field> = ["titular:String", "comprador_id:Uuid"]
        .iter()
        .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
        .collect();
    let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    let access = scaffold::ScaffoldAccess::from_args(&strings(public), &strings(roles))
        .map_err(|e| e.to_string())?;
    let shape =
        scaffold::EntityShape::default().with_owner((!owner.is_empty()).then(|| owner.to_string()));
    scaffold::run_scaffold_with_options(
        "Entrada",
        output,
        &fields,
        OverwritePolicy::Force,
        &access,
        &shape,
    )
    .map_err(|e| e.to_string())
}

fn block<'a>(src: &'a str, start: &str) -> &'a str {
    let from = &src[src.find(start).unwrap_or_else(|| panic!("{start}: {src}"))..];
    &from[..from.find("\n}").unwrap()]
}

#[test]
fn create_sets_the_owner_from_the_caller_and_update_keeps_it() {
    let (dir, output) = auth_project("owner_create");

    scaffold_entrada(&output, "comprador_id", &[], &[]).unwrap();

    let entity = puerto_toml::read(&output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Entrada")
        .unwrap();
    assert_eq!(entity.owner.as_deref(), Some("comprador_id"));
    let dto = read(&output, "contextos/entrada/rest/src/dto.rs");
    assert!(!block(&dto, "pub struct PeticionCrearEntrada").contains("comprador_id"));
    assert!(!block(&dto, "pub struct PeticionActualizarEntrada").contains("comprador_id"));
    assert!(
        block(&dto, "pub struct DtoEntrada").contains("comprador_id"),
        "the web tells the owner apart"
    );
    let create = read(&output, "contextos/entrada/aplicacion/src/crear_entrada.rs");
    assert!(!block(&create, "pub struct ParamsCrearEntrada").contains("comprador_id"));
    assert!(
        create.contains("comprador_id: params.actor.id_usuario"),
        "{create}"
    );
    let update = read(
        &output,
        "contextos/entrada/aplicacion/src/actualizar_entrada.rs",
    );
    assert!(!block(&update, "pub struct ParamsActualizarEntrada").contains("comprador_id"));
    assert!(
        update.contains("let comprador_id = entidad.comprador_id;"),
        "{update}"
    );
    cleanup(&dir);
}

#[test]
fn get_update_and_delete_let_through_only_the_owner_or_who_holds_the_route_roles() {
    let (dir, output) = auth_project("owner_check");

    scaffold_entrada(&output, "comprador_id", &["obtener_entrada=admin"], &[]).unwrap();

    for uc in ["obtener_entrada", "actualizar_entrada", "borrar_entrada"] {
        let src = read(
            &output,
            &format!("contextos/entrada/aplicacion/src/{uc}.rs"),
        );
        assert!(
            src.contains("if entidad.comprador_id != params.actor.id_usuario"),
            "{uc}: {src}"
        );
        assert!(src.contains("Err(ErrorEntrada::Prohibido)"), "{uc}");
        assert!(
            src.contains("_cuando_quien_llama_no_es_su_propietario()"),
            "{uc}: {src}"
        );
    }
    let get = read(
        &output,
        "contextos/entrada/aplicacion/src/obtener_entrada.rs",
    );
    assert!(get.contains("params.actor.tiene_rol(\"admin\")"), "{get}");
    assert!(get.contains("_cuando_quien_llama_tiene_el_rol()"), "{get}");
    // The route lets the owner reach the use case: its role is the use case's to check.
    let routes = read(&output, "contextos/entrada/rest/src/rutas.rs");
    assert!(!routes.contains("#[requiere_rol(\"admin\")]"), "{routes}");
    assert!(
        validate::run_validate(&output).is_ok(),
        "{:?}",
        validate::run_validate(&output).err()
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_an_owner_it_cannot_check_before_writing() {
    let (dir, output) = auth_project("owner_refusals");
    let manifest = read(&output, "puerto.toml");

    for (owner, public, expected) in [
        ("nada", &[][..], "nada"),
        ("titular", &[][..], "Uuid"),
        ("comprador_id", &["obtener_entrada"][..], "obtener_entrada"),
    ] {
        let err = scaffold_entrada(&output, owner, &[], public).unwrap_err();
        assert!(err.contains(expected), "{owner}: {err}");
        assert_eq!(read(&output, "puerto.toml"), manifest);
    }
    cleanup(&dir);

    let (dir, output) = (
        temp_dir("owner_no_auth"),
        temp_dir("owner_no_auth").join("owner-no-auth"),
    );
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    new_project(
        Some("owner-no-auth".into()),
        NewOptions {
            no_db: true,
            no_web: true,
            no_portada: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();
    let err = scaffold_entrada(&output, "comprador_id", &[], &[]).unwrap_err();
    assert!(err.contains("auth"), "{err}");
    cleanup(&dir);
}

// ── 27.18 (b): `GET /mis_<plural>` ──────────────────────────────────────────

#[test]
fn an_owned_entity_lists_its_callers_rows() {
    let (dir, output) = auth_project("owner_mine");

    scaffold_entrada(&output, "comprador_id", &[], &[]).unwrap();

    let entity = puerto_toml::read(&output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Entrada")
        .unwrap();
    assert!(
        entity
            .use_cases
            .contains(&"listar_mis_entradas".to_string())
    );
    let mine = read(
        &output,
        "contextos/entrada/aplicacion/src/listar_mis_entradas.rs",
    );
    assert!(
        mine.contains("comprador_id: Some(params.actor.id_usuario),"),
        "{mine}"
    );
    let port = read(&output, "contextos/entrada/dominio/src/repositorio.rs");
    assert!(
        block(&port, "pub struct FiltroEntrada").contains("pub comprador_id: Option<uuid::Uuid>,"),
        "{port}"
    );
    let list = read(
        &output,
        "contextos/entrada/aplicacion/src/listar_entradas.rs",
    );
    assert!(
        list.contains("comprador_id: None,"),
        "the whole list: {list}"
    );
    let memory = read(
        &output,
        "contextos/entrada/infraestructura/src/repositorio.rs",
    );
    assert!(
        memory.contains("filtro.comprador_id.is_none_or("),
        "{memory}"
    );
    let routes = read(&output, "contextos/entrada/rest/src/rutas.rs");
    assert!(routes.contains("path = \"/mis_entradas\""), "{routes}");
    assert!(routes.contains("pub listar_mis_entradas: Arc<dyn ListarMisEntradas>,"));
    assert!(
        routes.contains("deberia_responder_401_cuando_se_piden_las_entradas_propias_sin_token")
    );
    assert!(wiring(&output).contains("ListarMisEntradasImpl"));
    assert!(
        validate::run_validate(&output).is_ok(),
        "{:?}",
        validate::run_validate(&output).err()
    );
    cleanup(&dir);
}

// ── 27.18 (c): the web ─────────────────────────────────────────────────────

#[test]
fn the_web_lists_mine_and_lets_the_owner_edit_and_delete() {
    let dir = temp_dir("owner_web");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some("owner-web".into()),
        NewOptions {
            no_db: true,
            auth: Some("oidc".into()),
            no_portada: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();

    scaffold_entrada(&output, "comprador_id", &["borrar_entrada=admin"], &[]).unwrap();

    let list = read(
        &output,
        "web/src/contextos/entrada/contenedores/lista-entrada.tsx",
    );
    assert!(
        list.contains("{ propias = false }: { propias?: boolean }"),
        "{list}"
    );
    assert!(list.contains("api.GET(\"/mis_entradas\""), "{list}");
    let routes = read(&output, "web/src/generado/rutas.tsx");
    assert!(routes.contains("etiqueta: \"Mis entradas\""), "{routes}");
    assert!(
        routes.contains("<ContenedorListaEntrada propias />"),
        "{routes}"
    );
    let detail = read(
        &output,
        "web/src/contextos/entrada/contenedores/detalle-entrada.tsx",
    );
    assert!(
        detail.contains("usuario?.id === entrada.datos.compradorId"),
        "{detail}"
    );
    assert!(detail.contains("{propio && ("), "edit, no role: {detail}");
    assert!(
        detail.contains("{(propio || puede(\"admin\")) && ("),
        "delete: {detail}"
    );
    let test = read(
        &output,
        "web/src/contextos/entrada/contenedores/detalle-entrada.test.tsx",
    );
    assert!(
        test.contains("a su propietario aunque no tenga roles"),
        "{test}"
    );
    cleanup(&dir);
}
