use super::*;
use crate::commands::new::{NewOptions, new_project};
use crate::generators::auth::AGENTS_AUTH_SECTION;

// ── Phase 16.4: `puerto nuevo --auth oidc` ─────────────────────────────────────

fn new_auth_project(name: &str, options: NewOptions) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            auth: Some("oidc".into()),
            ..options
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

fn new_auth_project_without_auth(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

fn read(output: &Path, file: &str) -> String {
    fs::read_to_string(output.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn new_with_auth_records_the_mode_and_keeps_the_demo_public() {
    let (dir, output) = new_auth_project(
        "auth_new_manifest",
        NewOptions {
            no_db: true,
            ..Default::default()
        },
    );

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(config.project.auth.as_deref(), Some("oidc"));
    let saludo = config.entity.iter().find(|e| e.name == "Saludo").unwrap();
    assert_eq!(saludo.public, vec!["obtener_saludo"]);
    // What it wrote passes its own checks: the demo's handler takes no BearerAuth, and `user`
    // is not an orphan entity.
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn new_with_auth_writes_the_user_the_provisioning_and_me() {
    let (dir, output) = new_auth_project(
        "auth_new_files",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    for file in [
        "contextos/usuario/dominio/src/modelo.rs",
        "contextos/usuario/dominio/src/errores.rs",
        "contextos/usuario/dominio/src/repositorio.rs",
        "contextos/usuario/infraestructura/src/repositorio.rs",
        "contextos/usuario/infraestructura/src/aprovisionamiento.rs",
        "keycloak/realm.json",
        "contextos/usuario/rest/src/me.rs",
        "contextos/usuario/rest/src/me/dto.rs",
        "contextos/usuario/rest/src/me/rutas.rs",
        "docker-compose.yml",
    ] {
        assert!(output.join(file).exists(), "{file}");
    }
    assert_eq!(
        read(&output, "contextos/usuario/dominio/src/lib.rs"),
        "pub mod errores;\npub mod modelo;\npub mod repositorio;\n"
    );
    assert!(
        read(
            &output,
            "contextos/usuario/infraestructura/src/repositorio.rs"
        )
        .contains("RepositorioUsuarioEnMemoria")
    );
    assert!(read(&output, "contextos/usuario/rest/src/lib.rs").contains("pub mod me;"));
    let bootstrap = wiring(&output);
    assert!(
        bootstrap.contains("AutenticadorOidc::descubrir"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("RepositorioUsuarioEnMemoria::new()"),
        "{bootstrap}"
    );
    assert!(bootstrap.contains(".data(autenticador)"), "{bootstrap}");
    assert!(bootstrap.contains("api_me"), "{bootstrap}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_and_db_keeps_users_in_postgres() {
    let (dir, output) = new_auth_project(
        "auth_new_db",
        NewOptions {
            db: true,
            no_web: true,
            no_demo: true,
            ..Default::default()
        },
    );

    assert!(
        read(
            &output,
            "contextos/usuario/infraestructura/src/repositorio.rs"
        )
        .contains("RepositorioUsuarioPostgres")
    );
    let migrations: Vec<String> = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let users = migrations
        .iter()
        .find(|m| m.ends_with("_crear_tabla_usuarios.sql"))
        .unwrap_or_else(|| panic!("{migrations:?}"));
    let sql = read(&output, &format!("migraciones/{users}"));
    assert!(sql.contains("UNIQUE (emisor, sujeto)"), "{sql}");
    // No entity uses the database yet: the bootstrap still opens the pool, for the users.
    let bootstrap = wiring(&output);
    assert!(bootstrap.contains("crear_pool_postgres"), "{bootstrap}");
    assert!(
        bootstrap.contains("RepositorioUsuarioPostgres::new(pool.clone())"),
        "{bootstrap}"
    );
    // Postgres keeps its service; Keycloak joins it before the volumes.
    let compose = read(&output, "docker-compose.yml");
    let keycloak = compose.find("  keycloak:").unwrap();
    assert!(compose.find("  postgres:").unwrap() < keycloak);
    assert!(keycloak < compose.find("\nvolumes:").unwrap(), "{compose}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_without_the_demo_still_declares_the_user_and_me() {
    // Regression: the demo strip rewrote the three `lib.rs`/`api.rs` files after auth had
    // declared its modules in them, and the project did not compile (E0432, E0433).
    let (dir, output) = new_auth_project(
        "auth_new_no_demo",
        NewOptions {
            no_db: true,
            no_web: true,
            no_demo: true,
            ..Default::default()
        },
    );

    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains("\"contextos/*/*\""),
        "the usuario context is a member:\n{root}"
    );
    assert!(root.contains("dominio_usuario = { path"), "{root}");
    assert!(!root.contains("saludo"), "{root}");
    let infrastructure = read(&output, "contextos/usuario/infraestructura/src/lib.rs");
    assert!(
        infrastructure.contains("pub mod aprovisionamiento;"),
        "{infrastructure}"
    );
    let api = read(&output, "presentacion/api/Cargo.toml");
    assert!(api.contains("rest_usuario = { workspace = true }"), "{api}");
    assert!(!api.contains("saludo"), "{api}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_points_env_and_make_at_the_development_provider() {
    let (dir, output) = new_auth_project(
        "auth_new_env",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    for env in [".env", ".env.example"] {
        let src = read(&output, env);
        assert!(
            src.contains("SERVICE_PORT=8080"),
            "{env}: the base vars stay"
        );
        assert!(
            src.contains("AUTH_ISSUER=http://localhost:8180/realms/auth-new-env\n"),
            "{env}: {src}"
        );
        assert!(src.contains("AUTH_AUDIENCE=api\n"), "{env}");
        assert!(
            src.contains("AUTH_ROLES_CLAIM=realm_access.roles\n"),
            "{env}"
        );
    }
    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("auth/ui: docker-compose/up"));
    assert!(makefile.contains("docker-compose/up: ##"));
    assert!(makefile.contains("\nrun: docker-compose/up\n"));
    let realm = read(&output, "keycloak/realm.json");
    let realm: serde_json::Value = serde_json::from_str(&realm).expect("realm.json is JSON");
    assert_eq!(realm["realm"], "auth-new-env");
    assert_eq!(realm["users"][0]["email"], "admin@auth-new-env.localhost");
    assert_eq!(realm["clients"][0]["clientId"], "web");
    cleanup(&dir);
}

#[test]
fn new_with_auth_gives_development_a_password_grant_client_and_a_user_without_roles() {
    let (dir, output) = new_auth_project(
        "auth_new_token",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    let realm = read(&output, "keycloak/realm.json");
    let realm: serde_json::Value = serde_json::from_str(&realm).expect("realm.json is JSON");
    let clients = realm["clients"].as_array().unwrap();
    let web = clients.iter().find(|c| c["clientId"] == "web").unwrap();
    assert_eq!(
        web["directAccessGrantsEnabled"], false,
        "web keeps PKCE only"
    );
    let dev = clients.iter().find(|c| c["clientId"] == "dev-cli").unwrap();
    assert_eq!(dev["publicClient"], true);
    assert_eq!(dev["directAccessGrantsEnabled"], true);
    assert_eq!(dev["standardFlowEnabled"], false);
    assert_eq!(dev["redirectUris"], serde_json::json!([]));
    assert_eq!(
        dev["protocolMappers"], web["protocolMappers"],
        "the same `api` audience as web's tokens"
    );
    let users = realm["users"].as_array().unwrap();
    let user = users
        .iter()
        .find(|u| u["email"] == "user@auth-new-token.localhost")
        .expect("a second user");
    assert_eq!(
        user["realmRoles"],
        serde_json::json!(["default-roles-auth-new-token"]),
        "no `admin`"
    );
    assert_eq!(
        user["credentials"], users[0]["credentials"],
        "same dev password"
    );

    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("\nauth/token: ##"), "{makefile}");
    assert!(makefile.contains("client_id=dev-cli"), "{makefile}");
    assert!(
        makefile.contains("admin@auth-new-token.localhost"),
        "admin@ by default: {makefile}"
    );
    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains("make -s auth/token"), "{agents}");
    assert!(agents.contains("`dev-cli`"), "{agents}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_adds_the_auth_section_to_agents_md() {
    let (dir, output) = new_auth_project(
        "auth_new_agents",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains(AGENTS_AUTH_SECTION));
    assert!(agents.find("## Auth").unwrap() < agents.find("## Convenciones").unwrap());
    // What the section names exists in the project it describes (ADR-0010).
    for path in [
        "contextos/usuario/dominio/",
        "contextos/usuario/infraestructura/src/aprovisionamiento.rs",
    ] {
        assert!(AGENTS_AUTH_SECTION.contains(path), "{path}");
        assert!(output.join(path).exists(), "{path}");
    }
    assert!(read(&output, "Makefile").contains("auth/ui:"));
    cleanup(&dir);
}

#[test]
fn new_without_auth_writes_none_of_it() {
    let (dir, output) = new_auth_project_without_auth("auth_none");

    assert!(!output.join("contextos/usuario").exists());
    assert!(!output.join("contextos/usuario/rest").exists());
    assert!(!read(&output, "AGENTS.md").contains("## Auth"));
    assert!(!read(&output, "Cargo.toml").contains("puerto-auth"));
    cleanup(&dir);
}

// ── Phase 16.4: `scaffold` in an auth project ────────────────────────────────

fn access(public: &[&str], roles: &[&str]) -> scaffold::ScaffoldAccess {
    let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    scaffold::ScaffoldAccess::from_args(&strings(public), &strings(roles)).unwrap()
}

fn scaffold_producto(output: &Path, access: &scaffold::ScaffoldAccess) -> Result<(), String> {
    scaffold::run_scaffold_with_access(
        "Producto",
        output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Force,
        access,
    )
    .map_err(|e| e.to_string())
}

fn auth_project(name: &str) -> (PathBuf, PathBuf) {
    new_auth_project(
        name,
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    )
}

#[test]
fn scaffold_in_an_auth_project_protects_every_route_but_the_public_ones() {
    let (dir, output) = auth_project("auth_scaffold_routes");

    scaffold_producto(
        &output,
        &access(&["listar_productos"], &["borrar_producto=admin"]),
    )
    .unwrap();

    let routes = read(&output, "contextos/producto/rest/src/rutas.rs");
    assert_eq!(routes.matches("auth: BearerAuth").count(), 4, "{routes}");
    assert_eq!(routes.matches("actor: auth.0").count(), 4, "{routes}");
    assert_eq!(routes.matches("#[requiere_rol(\"admin\")]").count(), 1);
    let config = puerto_toml::read(&output).unwrap();
    let producto = config.entity.iter().find(|e| e.name == "Producto").unwrap();
    assert_eq!(producto.public, vec!["listar_productos"]);
    assert_eq!(
        producto.roles.get("borrar_producto"),
        Some(&vec!["admin".to_string()])
    );
    // The use cases behind protected routes carry the caller; the public one does not.
    let use_case = |uc: &str| {
        read(
            &output,
            &format!("contextos/producto/aplicacion/src/{uc}.rs"),
        )
    };
    assert!(use_case("crear_producto").contains("pub actor: Principal,"));
    assert!(!use_case("listar_productos").contains("actor"));
    assert!(
        read(&output, "contextos/producto/dominio/src/errores.rs").contains("producto.prohibido")
    );
    // What scaffold wrote is what validate checks.
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn rescaffold_without_flags_keeps_what_the_manifest_says() {
    let (dir, output) = auth_project("auth_scaffold_keep");
    scaffold_producto(
        &output,
        &access(&["listar_productos"], &["crear_producto=admin"]),
    )
    .unwrap();

    scaffold_producto(&output, &scaffold::ScaffoldAccess::default()).unwrap();

    let producto = puerto_toml::read(&output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Producto")
        .unwrap();
    assert_eq!(producto.public, vec!["listar_productos"]);
    assert!(
        read(&output, "contextos/producto/rest/src/rutas.rs")
            .contains("#[requiere_rol(\"admin\")]")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_access_naming_another_use_case_before_writing() {
    let (dir, output) = auth_project("auth_scaffold_refuse");

    let result = scaffold_producto(&output, &access(&["listar_producto"], &[]));

    assert!(
        result
            .unwrap_err()
            .contains("'listar_producto' no es uno de")
    );
    assert!(!output.join("contextos/producto").exists());
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_a_role_on_a_public_route() {
    let (dir, output) = auth_project("auth_scaffold_both");

    let result = scaffold_producto(
        &output,
        &access(&["listar_productos"], &["listar_productos=admin"]),
    );

    assert!(
        result
            .unwrap_err()
            .contains("es público y a la vez exige un rol")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_the_user_entity_in_an_auth_project() {
    let (dir, output) = auth_project("auth_scaffold_user");

    let result = scaffold::run_scaffold_with_access(
        "Usuario",
        &output,
        &[],
        OverwritePolicy::Force,
        &scaffold::ScaffoldAccess::default(),
    );

    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("un proyecto con auth ya usa")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_access_flags_outside_an_auth_project() {
    let (dir, output) = new_auth_project_without_auth("auth_scaffold_noauth");

    let result = scaffold_producto(&output, &access(&["listar_productos"], &[]));

    assert!(
        result
            .unwrap_err()
            .contains("necesitan un proyecto con auth")
    );
    cleanup(&dir);
}

#[test]
fn role_args_need_a_use_case_and_a_role() {
    let result = scaffold::ScaffoldAccess::from_args(&[], &["admin".into()]);
    assert!(
        result
            .unwrap_err()
            .contains("El formato es <caso_de_uso>=<rol>")
    );
}

#[test]
fn a_custom_use_case_of_an_auth_project_takes_the_caller() {
    let (dir, output) = auth_project("auth_use_case");
    scaffold_producto(&output, &scaffold::ScaffoldAccess::default()).unwrap();

    scaffold::run_use_case("Producto", "publish_producto", &output).unwrap();

    let params = read(
        &output,
        "contextos/producto/aplicacion/src/publish_producto.rs",
    );
    assert!(params.contains("pub actor: Principal,"), "{params}");
    cleanup(&dir);
}

// ── Phase 16.5: the frontend, OIDC ───────────────────────────────────────────

fn auth_web_project(name: &str) -> (PathBuf, PathBuf) {
    new_auth_project(
        name,
        NewOptions {
            no_db: true,
            ..Default::default()
        },
    )
}

#[test]
fn new_with_auth_and_web_writes_the_gateway_the_provider_and_the_callback() {
    let (dir, output) = auth_web_project("auth_web_files");

    for file in [
        "web/src/auth/sesion.ts",
        "web/src/auth/pasarela-oidc.ts",
        "web/src/auth/pasarela-oidc.test.ts",
        "web/src/auth/middleware-bearer.ts",
        "web/src/auth/middleware-bearer.test.ts",
        "web/src/auth/auth.tsx",
        "web/src/auth/auth.test.tsx",
        "web/src/paginas/auth-callback.tsx",
        "web/src/paginas/auth-callback.test.tsx",
        "web/src/compartido/ui/menu-usuario.tsx",
        "web/src/compartido/ui/menu-usuario.test.tsx",
        "web/src/pruebas/auth.ts",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // The app runs inside the provider, the layout shows who is signed in, tests sign a caller in.
    assert!(read(&output, "web/src/main.tsx").contains("<ProveedorAuth>"));
    assert!(read(&output, "web/src/compartido/ui/marco.tsx").contains("<MenuUsuario />"));
    let render = read(&output, "web/src/pruebas/renderizar.tsx");
    assert!(render.contains("MadreUsuarioActual.llamante()"), "{render}");
    assert!(!render.contains("{% raw %}"), "{render}");
    // The composition root builds the gateway and signs every call; the callback has its route.
    let dependencies = read(&output, "web/src/generado/dependencias.ts");
    assert!(
        dependencies.contains("auth: PasarelaAuth;"),
        "{dependencies}"
    );
    assert!(
        dependencies.contains("pasarelaOidc(ajustesOidcDeEntorno())"),
        "{dependencies}"
    );
    let routes = read(&output, "web/src/generado/rutas.tsx");
    assert!(
        routes.contains("<Route path=\"/auth/callback\">"),
        "{routes}"
    );
    assert!(output.join("web/src/generado/roles.ts").exists());
    cleanup(&dir);
}

#[test]
fn new_with_auth_and_web_pins_the_client_and_keeps_it_in_the_adapter() {
    let (dir, output) = auth_web_project("auth_web_package");

    let package = read(&output, "web/package.json");
    assert!(
        package.contains(&format!(
            "\"oidc-client-ts\": \"{}\",",
            crate::generators::auth::OIDC_CLIENT_TS_VERSION
        )),
        "{package}"
    );
    // Dependencies stay sorted: `oidc-client-ts` before `openapi-fetch`.
    assert!(package.find("oidc-client-ts").unwrap() < package.find("openapi-fetch").unwrap());
    let cruiser = read(&output, "web/.dependency-cruiser.cjs");
    assert!(
        cruiser.contains("solo-la-pasarela-importa-oidc-client-ts"),
        "{cruiser}"
    );
    for file in ["web/.env", "web/.env.example"] {
        let env = read(&output, file);
        assert!(
            env.contains("VITE_AUTH_ISSUER=http://localhost:8180/realms/auth-web-package"),
            "{file}: {env}"
        );
        assert!(env.contains("VITE_AUTH_CLIENT_ID=web\n"), "{file}: {env}");
        assert!(env.contains("VITE_AUTH_AUDIENCE=\n"), "{file}: {env}");
    }
    assert!(read(&output, "AGENTS.md").contains(crate::generators::auth::AGENTS_AUTH_WEB_SECTION));
    cleanup(&dir);
}

#[test]
fn new_with_auth_and_no_web_writes_nothing_for_the_frontend() {
    let (dir, output) = auth_project("auth_no_web");

    assert!(!output.join("web").exists());
    assert!(!read(&output, "AGENTS.md").contains(crate::generators::auth::AGENTS_AUTH_WEB_SECTION));
    cleanup(&dir);
}

fn scaffold_web_producto(output: &Path, access: &scaffold::ScaffoldAccess) {
    scaffold::run_scaffold_with_access(
        "Producto",
        output,
        &[scaffold_test_field("title", "String")],
        OverwritePolicy::Force,
        access,
    )
    .unwrap();
}

#[test]
fn scaffold_in_an_auth_web_project_guards_every_screen_but_the_public_ones() {
    let (dir, output) = auth_web_project("auth_web_routes");

    scaffold_web_producto(
        &output,
        &access(
            &["listar_productos", "obtener_producto"],
            &["crear_producto=admin"],
        ),
    );

    let routes = read(&output, "web/src/generado/rutas.tsx");
    // List and detail are public; new and edit (create, update) are not.
    assert!(
        routes.contains("<Route path=\"/productos\">\n        <ContenedorListaProducto />"),
        "{routes}"
    );
    assert!(
        routes.contains("<Route path=\"/productos/nuevo\">\n        <RequiereAuth>\n          <ContenedorFormularioProducto />"),
        "{routes}"
    );
    assert!(
        routes.contains(
            "<Route path=\"/productos/:id/editar\">\n        {(params) => (\n          <RequiereAuth>"
        ),
        "{routes}"
    );
    assert!(
        routes.contains(
            "<Route path=\"/productos/:id\">\n        {(params) => <ContenedorDetalleProducto"
        ),
        "{routes}"
    );
    let roles = read(&output, "web/src/generado/roles.ts");
    assert!(roles.contains("[\"admin\"]"), "{roles}");
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_auth_web_project_hides_the_actions_a_role_guards() {
    let (dir, output) = auth_web_project("auth_web_screens");

    scaffold_web_producto(
        &output,
        &access(
            &[],
            &[
                "crear_producto=admin",
                "borrar_producto=admin",
                "borrar_producto=owner",
            ],
        ),
    );

    let containers = "web/src/contextos/producto/contenedores";
    let list = read(&output, &format!("{containers}/lista-producto.tsx"));
    assert!(list.contains("{puede(\"admin\") && ("), "{list}");
    let detail = read(&output, &format!("{containers}/detalle-producto.tsx"));
    assert!(
        detail.contains("{puede(\"admin\") && puede(\"owner\") && ("),
        "{detail}"
    );
    // Updating needs no role: Edit is always offered.
    assert!(
        !detail.contains("puede(\"admin\") && (\n                <Link"),
        "{detail}"
    );
    let detail_test = read(&output, &format!("{containers}/detalle-producto.test.tsx"));
    assert!(
        detail_test.contains("no debería ofrecer Borrar a quien no tiene su rol"),
        "{detail_test}"
    );
    assert!(
        detail_test.contains("debería mostrar auth.prohibido cuando la API rechaza el borrado"),
        "{detail_test}"
    );
    let list_test = read(&output, &format!("{containers}/lista-producto.test.tsx"));
    assert!(
        list_test.contains("no debería ofrecer Crear a quien no tiene su rol"),
        "{list_test}"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_auth_web_project_without_roles_checks_none() {
    let (dir, output) = auth_web_project("auth_web_no_roles");

    scaffold_web_producto(&output, &access(&[], &[]));

    let containers = "web/src/contextos/producto/contenedores";
    for file in ["lista-producto.tsx", "detalle-producto.tsx"] {
        let src = read(&output, &format!("{containers}/{file}"));
        assert!(!src.contains("useAuth"), "{file}: {src}");
    }
    // A protected deletion can still be refused by the use case itself.
    let detail_test = read(&output, &format!("{containers}/detalle-producto.test.tsx"));
    assert!(detail_test.contains("auth.prohibido"), "{detail_test}");
    cleanup(&dir);
}

// ── Phase 16.6: `puerto nuevo --auth local` ────────────────────────────────────

fn new_local_project(name: &str, options: NewOptions) -> Result<(PathBuf, PathBuf), String> {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            auth: Some("local".into()),
            ..options
        },
        Some(dir.clone()),
    )
    .map(|output| (dir.clone(), output))
    .map_err(|e| {
        assert!(
            !dir.join(name.replace('_', "-")).exists(),
            "nothing is written when refused"
        );
        cleanup(&dir);
        e.to_string()
    })
}

fn local_project(name: &str) -> (PathBuf, PathBuf) {
    new_local_project(
        name,
        NewOptions {
            db: true,
            no_web: true,
            ..Default::default()
        },
    )
    .unwrap()
}

/// The file name and content of the migration whose name ends with `suffix`.
fn migration_ending(output: &Path, suffix: &str) -> (String, String) {
    let dir = output.join("migraciones");
    let name = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .find(|m| m.ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    let content = read(output, &format!("migraciones/{name}"));
    (name, content)
}

#[test]
fn new_with_local_auth_writes_accounts_sessions_and_the_auth_api() {
    let (dir, output) = local_project("auth_local_files");

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(config.project.auth.as_deref(), Some("local"));
    for file in [
        "contextos/usuario/aplicacion/src/registrar_usuario.rs",
        "contextos/usuario/aplicacion/src/login.rs",
        "contextos/usuario/aplicacion/src/logout.rs",
        "contextos/usuario/rest/src/auth.rs",
        "contextos/usuario/rest/src/auth/dto.rs",
        "contextos/usuario/rest/src/auth/rutas.rs",
        "contextos/usuario/rest/src/auth/respuestas.rs",
        "contextos/usuario/rest/src/auth/errores.rs",
        "contextos/usuario/rest/src/me/rutas.rs",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // No provider to provision from, and none to run.
    assert!(
        !output
            .join("contextos/usuario/infraestructura/src/aprovisionamiento.rs")
            .exists()
    );
    assert!(!output.join("keycloak").exists());
    assert!(!read(&output, "docker-compose.yml").contains("keycloak"));
    // The tables `puerto-sqlx` is tested against, users first: credentials and sessions
    // reference them.
    let (users, users_sql) = migration_ending(&output, "_crear_tabla_usuarios.sql");
    let (auth, auth_sql) = migration_ending(&output, "_crear_tablas_auth.sql");
    assert!(users_sql.ends_with(include_str!("../../../sqlx/migrations/users.sql")));
    assert!(auth_sql.ends_with(include_str!("../../../sqlx/migrations/auth.sql")));
    assert!(users < auth, "{users} runs before {auth}");
    let bootstrap = wiring(&output);
    for wired in [
        "SesionesPostgres::new",
        "CredencialesPostgres::new",
        "HasherArgon2",
        "api_auth",
        ".data(autenticador)",
    ] {
        assert!(bootstrap.contains(wired), "{wired}: {bootstrap}");
    }
    assert!(!bootstrap.contains("Oidc"), "{bootstrap}");
    assert!(read(&output, "presentacion/api/Cargo.toml").contains("features = [\"local\"]"));
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn new_with_local_auth_grants_roles_from_make_and_reads_the_session_lifetime() {
    let (dir, output) = local_project("auth_local_env");

    let env = read(&output, ".env.example");
    assert!(env.contains("AUTH_SESSION_TTL=30d"), "{env}");
    assert!(!env.contains("AUTH_ISSUER"), "{env}");
    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("auth/conceder:"), "{makefile}");
    assert!(makefile.contains("array_append(roles"), "{makefile}");
    assert!(!makefile.contains("auth/ui:"), "{makefile}");
    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains("## Auth (`[proyecto] auth`)"), "{agents}");
    assert!(agents.contains("make auth/conceder"), "{agents}");
    assert!(!agents.contains("Keycloak"), "{agents}");
    cleanup(&dir);
}

#[test]
fn new_refuses_local_auth_without_a_database() {
    let err = new_local_project(
        "auth_local_no_db",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    )
    .unwrap_err();

    assert!(err.contains("--db"), "{err}");
}

#[test]
fn new_with_local_auth_and_web_signs_in_with_a_password_not_a_provider() {
    let (dir, output) = new_local_project(
        "auth_local_web",
        NewOptions {
            db: true,
            ..Default::default()
        },
    )
    .unwrap();

    for file in [
        "web/src/auth/cuentas.ts",
        "web/src/auth/mensajes.ts",
        "web/src/auth/pasarela-local.ts",
        "web/src/auth/pasarela-local.test.ts",
        "web/src/paginas/login.tsx",
        "web/src/paginas/registro.tsx",
        "web/src/auth/auth.tsx",
        "web/src/auth/middleware-bearer.ts",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // No provider: no OIDC client, callback, settings or dependency.
    assert!(!output.join("web/src/auth/pasarela-oidc.ts").exists());
    assert!(!output.join("web/src/paginas/auth-callback.tsx").exists());
    assert!(!output.join("web/.env").exists());
    assert!(!read(&output, "web/package.json").contains("oidc-client-ts"));
    let dependencies = read(&output, "web/src/generado/dependencias.ts");
    assert!(dependencies.contains("cuentas: Cuentas;"), "{dependencies}");
    assert!(dependencies.contains("pasarelaLocal()"), "{dependencies}");
    let routes = read(&output, "web/src/generado/rutas.tsx");
    assert!(routes.contains("<Route path=\"/login\">"), "{routes}");
    assert!(routes.contains("<Route path=\"/registro\">"), "{routes}");
    assert!(!routes.contains("/auth/callback"), "{routes}");
    assert!(read(&output, "web/src/pruebas/auth.ts").contains("export function cuentasFalsas"));
    assert!(
        read(&output, "AGENTS.md").contains(crate::generators::auth::AGENTS_AUTH_LOCAL_WEB_SECTION)
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_a_local_auth_project_protects_routes_as_in_an_oidc_one() {
    let (dir, output) = local_project("auth_local_scaffold");

    scaffold_producto(
        &output,
        &access(&["listar_productos"], &["borrar_producto=admin"]),
    )
    .unwrap();

    let routes = read(&output, "contextos/producto/rest/src/rutas.rs");
    assert_eq!(routes.matches("auth: BearerAuth").count(), 4, "{routes}");
    assert_eq!(routes.matches("#[requiere_rol(\"admin\")]").count(), 1);
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}
