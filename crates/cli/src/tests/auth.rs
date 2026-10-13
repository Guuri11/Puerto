use super::*;
use crate::commands::new::{NewOptions, new_project};
use crate::generators::auth::AGENTS_AUTH_SECTION;

// ── Phase 16.4: `puerto new --auth oidc` ─────────────────────────────────────

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
    let greeting = config.entity.iter().find(|e| e.name == "Greeting").unwrap();
    assert_eq!(greeting.public, vec!["get_greeting"]);
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
        "business/src/domain/user/model.rs",
        "business/src/domain/user/errors.rs",
        "business/src/domain/user/repository.rs",
        "infrastructure/src/user/repository.rs",
        "infrastructure/src/user/provisioning.rs",
        "infrastructure/keycloak/realm.json",
        "presentation/src/api/me.rs",
        "presentation/src/api/me/dto.rs",
        "presentation/src/api/me/routes.rs",
        "docker-compose.yml",
    ] {
        assert!(output.join(file).exists(), "{file}");
    }
    assert!(read(&output, "business/src/lib.rs").contains("pub mod user {"));
    assert!(
        read(&output, "infrastructure/src/user/repository.rs").contains("InMemoryUserRepository")
    );
    assert!(read(&output, "presentation/src/api.rs").contains("pub mod me;"));
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(
        bootstrap.contains("OidcAuthenticator::discover"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("InMemoryUserRepository::new()"),
        "{bootstrap}"
    );
    assert!(bootstrap.contains(".data(authenticator)"), "{bootstrap}");
    assert!(bootstrap.contains("me_api"), "{bootstrap}");
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

    assert!(read(&output, "infrastructure/src/user/repository.rs").contains("PgUserRepository"));
    let migrations: Vec<String> = fs::read_dir(output.join("infrastructure/migrations"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let users = migrations
        .iter()
        .find(|m| m.ends_with("_create_users_table.sql"))
        .unwrap_or_else(|| panic!("{migrations:?}"));
    let sql = read(&output, &format!("infrastructure/migrations/{users}"));
    assert!(sql.contains("UNIQUE (issuer, subject)"), "{sql}");
    // No entity uses the database yet: the bootstrap still opens the pool, for the users.
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(bootstrap.contains("create_postgres_pool"), "{bootstrap}");
    assert!(
        bootstrap.contains("PgUserRepository::new(pool.clone())"),
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

    let business = read(&output, "business/src/lib.rs");
    assert!(business.contains("pub mod user {"), "{business}");
    assert!(business.contains("pub mod principal_mother;"), "{business}");
    assert!(!business.contains("greeting"), "{business}");
    let infrastructure = read(&output, "infrastructure/src/lib.rs");
    assert!(
        infrastructure.contains("pub mod provisioning;"),
        "{infrastructure}"
    );
    let api = read(&output, "presentation/src/api.rs");
    assert!(api.contains("pub mod me;"), "{api}");
    assert!(!api.contains("greeting"), "{api}");
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
    let realm = read(&output, "infrastructure/keycloak/realm.json");
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

    let realm = read(&output, "infrastructure/keycloak/realm.json");
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
    assert!(agents.find("## Auth").unwrap() < agents.find("## Conventions").unwrap());
    // What the section names exists in the project it describes (ADR-0010).
    for path in [
        "business/src/domain/user",
        "infrastructure/src/user/provisioning.rs",
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

    assert!(!output.join("business/src/domain/user").exists());
    assert!(!output.join("presentation/src/api/me.rs").exists());
    assert!(!read(&output, "AGENTS.md").contains("## Auth"));
    assert!(!read(&output, "Cargo.toml").contains("puerto-auth"));
    cleanup(&dir);
}

// ── Phase 16.4: `scaffold` in an auth project ────────────────────────────────

fn access(public: &[&str], roles: &[&str]) -> scaffold::ScaffoldAccess {
    let strings = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    scaffold::ScaffoldAccess::from_args(&strings(public), &strings(roles)).unwrap()
}

fn scaffold_product(output: &Path, access: &scaffold::ScaffoldAccess) -> Result<(), String> {
    scaffold::run_scaffold_with_access(
        "Product",
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

    scaffold_product(
        &output,
        &access(&["list_products"], &["delete_product=admin"]),
    )
    .unwrap();

    let routes = read(&output, "presentation/src/api/product/routes.rs");
    assert_eq!(routes.matches("auth: BearerAuth").count(), 4, "{routes}");
    assert_eq!(routes.matches("actor: auth.0").count(), 4, "{routes}");
    assert_eq!(routes.matches("#[require_role(\"admin\")]").count(), 1);
    let config = puerto_toml::read(&output).unwrap();
    let product = config.entity.iter().find(|e| e.name == "Product").unwrap();
    assert_eq!(product.public, vec!["list_products"]);
    assert_eq!(
        product.roles.get("delete_product"),
        Some(&vec!["admin".to_string()])
    );
    // The use cases behind protected routes carry the caller; the public one does not.
    let use_case = |uc: &str| {
        read(
            &output,
            &format!("business/src/domain/product/use_cases/{uc}.rs"),
        )
    };
    assert!(use_case("create_product").contains("pub actor: Principal,"));
    assert!(!use_case("list_products").contains("actor"));
    assert!(read(&output, "business/src/domain/product/errors.rs").contains("product.forbidden"));
    // What scaffold wrote is what validate checks.
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn rescaffold_without_flags_keeps_what_the_manifest_says() {
    let (dir, output) = auth_project("auth_scaffold_keep");
    scaffold_product(
        &output,
        &access(&["list_products"], &["create_product=admin"]),
    )
    .unwrap();

    scaffold_product(&output, &scaffold::ScaffoldAccess::default()).unwrap();

    let product = puerto_toml::read(&output)
        .unwrap()
        .entity
        .into_iter()
        .find(|e| e.name == "Product")
        .unwrap();
    assert_eq!(product.public, vec!["list_products"]);
    assert!(
        read(&output, "presentation/src/api/product/routes.rs")
            .contains("#[require_role(\"admin\")]")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_access_naming_another_use_case_before_writing() {
    let (dir, output) = auth_project("auth_scaffold_refuse");

    let result = scaffold_product(&output, &access(&["list_product"], &[]));

    assert!(result.unwrap_err().contains("'list_product' is not one of"));
    assert!(!output.join("business/src/domain/product").exists());
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_a_role_on_a_public_route() {
    let (dir, output) = auth_project("auth_scaffold_both");

    let result = scaffold_product(
        &output,
        &access(&["list_products"], &["list_products=admin"]),
    );

    assert!(
        result
            .unwrap_err()
            .contains("both public and requires a role")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_the_user_entity_in_an_auth_project() {
    let (dir, output) = auth_project("auth_scaffold_user");

    let result = scaffold::run_scaffold_with_access(
        "User",
        &output,
        &[],
        OverwritePolicy::Force,
        &scaffold::ScaffoldAccess::default(),
    );

    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("an auth project already uses")
    );
    cleanup(&dir);
}

#[test]
fn scaffold_refuses_access_flags_outside_an_auth_project() {
    let (dir, output) = new_auth_project_without_auth("auth_scaffold_noauth");

    let result = scaffold_product(&output, &access(&["list_products"], &[]));

    assert!(result.unwrap_err().contains("need an auth project"));
    cleanup(&dir);
}

#[test]
fn role_args_need_a_use_case_and_a_role() {
    let result = scaffold::ScaffoldAccess::from_args(&[], &["admin".into()]);
    assert!(result.unwrap_err().contains("Expected <use_case>=<role>"));
}

#[test]
fn a_custom_use_case_of_an_auth_project_takes_the_caller() {
    let (dir, output) = auth_project("auth_use_case");
    scaffold_product(&output, &scaffold::ScaffoldAccess::default()).unwrap();

    scaffold::run_use_case("Product", "publish_product", &output).unwrap();

    let params = read(
        &output,
        "business/src/domain/product/use_cases/publish_product.rs",
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
        "web/src/domain/auth/session.ts",
        "web/src/infrastructure/auth/oidc-auth-gateway.ts",
        "web/src/infrastructure/auth/oidc-auth-gateway.test.ts",
        "web/src/infrastructure/auth/bearer-middleware.ts",
        "web/src/infrastructure/auth/bearer-middleware.test.ts",
        "web/src/presentation/core/auth.tsx",
        "web/src/presentation/core/auth.test.tsx",
        "web/src/presentation/pages/auth-callback.tsx",
        "web/src/presentation/pages/auth-callback.test.tsx",
        "web/src/presentation/shared/ui/user-menu.tsx",
        "web/src/presentation/shared/ui/user-menu.test.tsx",
        "web/src/test-support/auth.ts",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // The app runs inside the provider, the layout shows who is signed in, tests sign a caller in.
    assert!(read(&output, "web/src/main.tsx").contains("<AuthProvider>"));
    assert!(read(&output, "web/src/presentation/shared/ui/layout.tsx").contains("<UserMenu />"));
    let render = read(&output, "web/src/test-support/render.tsx");
    assert!(render.contains("CurrentUserMother.caller()"), "{render}");
    assert!(!render.contains("{% raw %}"), "{render}");
    // The composition root builds the gateway and signs every call; the callback has its route.
    let dependencies = read(
        &output,
        "web/src/presentation/core/generated/dependencies.ts",
    );
    assert!(
        dependencies.contains("auth: AuthGateway;"),
        "{dependencies}"
    );
    assert!(
        dependencies.contains("oidcAuthGateway(oidcSettingsFromEnv())"),
        "{dependencies}"
    );
    let routes = read(&output, "web/src/presentation/core/generated/routes.tsx");
    assert!(
        routes.contains("<Route path=\"/auth/callback\">"),
        "{routes}"
    );
    assert!(
        output
            .join("web/src/presentation/core/generated/roles.ts")
            .exists()
    );
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
        cruiser.contains("only-the-auth-adapter-imports-oidc-client-ts"),
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

fn scaffold_web_product(output: &Path, access: &scaffold::ScaffoldAccess) {
    scaffold::run_scaffold_with_access(
        "Product",
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

    scaffold_web_product(
        &output,
        &access(&["list_products", "get_product"], &["create_product=admin"]),
    );

    let routes = read(&output, "web/src/presentation/core/generated/routes.tsx");
    // List and detail are public; new and edit (create, update) are not.
    assert!(
        routes.contains("<Route path=\"/products\">\n        <ProductListContainer />"),
        "{routes}"
    );
    assert!(
        routes.contains("<Route path=\"/products/new\">\n        <RequireAuth>\n          <ProductFormContainer />"),
        "{routes}"
    );
    assert!(
        routes.contains(
            "<Route path=\"/products/:id/edit\">\n        {(params) => (\n          <RequireAuth>"
        ),
        "{routes}"
    );
    assert!(
        routes.contains("<Route path=\"/products/:id\">{(params) => <ProductDetailContainer"),
        "{routes}"
    );
    let roles = read(&output, "web/src/presentation/core/generated/roles.ts");
    assert!(roles.contains("[\"admin\"]"), "{roles}");
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_auth_web_project_hides_the_actions_a_role_guards() {
    let (dir, output) = auth_web_project("auth_web_screens");

    scaffold_web_product(
        &output,
        &access(
            &[],
            &[
                "create_product=admin",
                "delete_product=admin",
                "delete_product=owner",
            ],
        ),
    );

    let containers = "web/src/presentation/features/product/containers";
    let list = read(&output, &format!("{containers}/product-list-container.tsx"));
    assert!(list.contains("{can(\"admin\") && ("), "{list}");
    let detail = read(
        &output,
        &format!("{containers}/product-detail-container.tsx"),
    );
    assert!(
        detail.contains("{can(\"admin\") && can(\"owner\") && ("),
        "{detail}"
    );
    // Updating needs no role: Edit is always offered.
    assert!(
        !detail.contains("can(\"admin\") && (\n                <Link"),
        "{detail}"
    );
    let detail_test = read(
        &output,
        &format!("{containers}/product-detail-container.test.tsx"),
    );
    assert!(
        detail_test.contains("should not offer Delete to a user without its role"),
        "{detail_test}"
    );
    assert!(
        detail_test.contains("should show auth.forbidden when the API refuses the deletion"),
        "{detail_test}"
    );
    let list_test = read(
        &output,
        &format!("{containers}/product-list-container.test.tsx"),
    );
    assert!(
        list_test.contains("should not offer New to a user without its role"),
        "{list_test}"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_an_auth_web_project_without_roles_checks_none() {
    let (dir, output) = auth_web_project("auth_web_no_roles");

    scaffold_web_product(&output, &access(&[], &[]));

    let containers = "web/src/presentation/features/product/containers";
    for file in ["product-list-container.tsx", "product-detail-container.tsx"] {
        let src = read(&output, &format!("{containers}/{file}"));
        assert!(!src.contains("useAuth"), "{file}: {src}");
    }
    // A protected deletion can still be refused by the use case itself.
    let detail_test = read(
        &output,
        &format!("{containers}/product-detail-container.test.tsx"),
    );
    assert!(detail_test.contains("auth.forbidden"), "{detail_test}");
    cleanup(&dir);
}

// ── Phase 16.6: `puerto new --auth local` ────────────────────────────────────

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
    let dir = output.join("infrastructure/migrations");
    let name = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .find(|m| m.ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    let content = read(output, &format!("infrastructure/migrations/{name}"));
    (name, content)
}

#[test]
fn new_with_local_auth_writes_accounts_sessions_and_the_auth_api() {
    let (dir, output) = local_project("auth_local_files");

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(config.project.auth.as_deref(), Some("local"));
    for file in [
        "business/src/domain/user/use_cases/register_user.rs",
        "business/src/domain/user/use_cases/login.rs",
        "business/src/domain/user/use_cases/logout.rs",
        "business/src/application/user/register_user.rs",
        "business/src/application/user/login.rs",
        "business/src/application/user/logout.rs",
        "presentation/src/api/auth.rs",
        "presentation/src/api/auth/dto.rs",
        "presentation/src/api/auth/routes.rs",
        "presentation/src/api/auth/responses.rs",
        "presentation/src/api/auth/error_mapper.rs",
        "presentation/src/api/me/routes.rs",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // No provider to provision from, and none to run.
    assert!(
        !output
            .join("infrastructure/src/user/provisioning.rs")
            .exists()
    );
    assert!(!output.join("infrastructure/keycloak").exists());
    assert!(!read(&output, "docker-compose.yml").contains("keycloak"));
    // The tables `puerto-sqlx` is tested against, users first: credentials and sessions
    // reference them.
    let (users, users_sql) = migration_ending(&output, "_create_users_table.sql");
    let (auth, auth_sql) = migration_ending(&output, "_create_auth_tables.sql");
    assert!(users_sql.ends_with(include_str!("../../../sqlx/migrations/users.sql")));
    assert!(auth_sql.ends_with(include_str!("../../../sqlx/migrations/auth.sql")));
    assert!(users < auth, "{users} runs before {auth}");
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    for wired in [
        "PgSessions::new",
        "PgCredentials::new",
        "Argon2Hasher",
        "auth_api",
        ".data(authenticator)",
    ] {
        assert!(bootstrap.contains(wired), "{wired}: {bootstrap}");
    }
    assert!(!bootstrap.contains("Oidc"), "{bootstrap}");
    assert!(read(&output, "presentation/Cargo.toml").contains("features = [\"local\"]"));
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
    assert!(makefile.contains("auth/grant:"), "{makefile}");
    assert!(makefile.contains("array_append(roles"), "{makefile}");
    assert!(!makefile.contains("auth/ui:"), "{makefile}");
    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains("## Auth (`[project] auth`)"), "{agents}");
    assert!(agents.contains("make auth/grant"), "{agents}");
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
        "web/src/domain/auth/accounts.ts",
        "web/src/domain/auth/messages.ts",
        "web/src/infrastructure/auth/local-auth-gateway.ts",
        "web/src/infrastructure/auth/local-auth-gateway.test.ts",
        "web/src/presentation/pages/login.tsx",
        "web/src/presentation/pages/register.tsx",
        "web/src/presentation/core/auth.tsx",
        "web/src/infrastructure/auth/bearer-middleware.ts",
    ] {
        assert!(output.join(file).exists(), "{file} missing");
    }
    // No provider: no OIDC client, callback, settings or dependency.
    assert!(
        !output
            .join("web/src/infrastructure/auth/oidc-auth-gateway.ts")
            .exists()
    );
    assert!(
        !output
            .join("web/src/presentation/pages/auth-callback.tsx")
            .exists()
    );
    assert!(!output.join("web/.env").exists());
    assert!(!read(&output, "web/package.json").contains("oidc-client-ts"));
    let dependencies = read(
        &output,
        "web/src/presentation/core/generated/dependencies.ts",
    );
    assert!(
        dependencies.contains("accounts: Accounts;"),
        "{dependencies}"
    );
    assert!(
        dependencies.contains("localAuthGateway()"),
        "{dependencies}"
    );
    let routes = read(&output, "web/src/presentation/core/generated/routes.tsx");
    assert!(routes.contains("<Route path=\"/login\">"), "{routes}");
    assert!(routes.contains("<Route path=\"/register\">"), "{routes}");
    assert!(!routes.contains("/auth/callback"), "{routes}");
    assert!(read(&output, "web/src/test-support/auth.ts").contains("export function fakeAccounts"));
    assert!(
        read(&output, "AGENTS.md").contains(crate::generators::auth::AGENTS_AUTH_LOCAL_WEB_SECTION)
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_a_local_auth_project_protects_routes_as_in_an_oidc_one() {
    let (dir, output) = local_project("auth_local_scaffold");

    scaffold_product(
        &output,
        &access(&["list_products"], &["delete_product=admin"]),
    )
    .unwrap();

    let routes = read(&output, "presentation/src/api/product/routes.rs");
    assert_eq!(routes.matches("auth: BearerAuth").count(), 4, "{routes}");
    assert_eq!(routes.matches("#[require_role(\"admin\")]").count(), 1);
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}
