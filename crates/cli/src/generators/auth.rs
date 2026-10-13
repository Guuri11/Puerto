//! `puerto new --auth oidc` (ADR-0021): the `User` entity every caller becomes, the provisioning
//! `Authenticator` in front of the provider, `GET /me`, and Keycloak for development. Every file
//! here is written once — the project's from then on.

use std::fs;
use std::path::Path;

use crate::generators::naming::write_file;
use crate::generators::project::{add_compose_service, add_workspace_dependency};
use crate::generators::render::render;
use crate::patchers::api_rs::patch_api_rs;
use crate::patchers::lib_rs::insert_before_block_end;

/// The Keycloak a project runs in development; `make test/auth` tests Puerto against the same tag.
pub const KEYCLOAK_IMAGE: &str = "quay.io/keycloak/keycloak:26.8.0";
/// Where Keycloak listens on the host: off 8080, the API's own port.
pub const KEYCLOAK_PORT: u16 = 8180;
/// The development user's password, in the imported realm and in `.env.example`.
pub const DEV_PASSWORD: &str = "admin-dev-only";
/// The audience Keycloak's mapper adds to the `web` client's tokens, and `AUTH_AUDIENCE`.
pub const AUDIENCE: &str = "api";
/// The OpenID Connect client of the frontend, pinned like every `web/package.json` dependency.
pub const OIDC_CLIENT_TS_VERSION: &str = "3.5.0";

/// A mode `puerto new --auth` knows.
pub fn check_mode(mode: &str) -> Result<(), String> {
    crate::validation::validate_auth_mode(mode)
}

/// Whether `puerto new --auth <mode>` can write a project with this database choice — asked
/// before anything is written. Local accounts live in Postgres.
pub fn check_new_project(mode: &str, db: bool) -> Result<(), String> {
    check_mode(mode)?;
    if mode == "local" && !db {
        return Err("--auth local keeps accounts in the database: add --db".into());
    }
    Ok(())
}

/// Who may call each use case of an entity: what the templates of an auth project branch on.
/// Outside an auth project nothing is protected.
#[derive(Debug, Clone, Default)]
pub struct Access {
    auth: bool,
    public: Vec<String>,
    roles: std::collections::BTreeMap<String, Vec<String>>,
}

impl Access {
    /// The entity `pascal`'s, as `puerto.toml` at `base` records it.
    pub fn of(base: &Path, pascal: &str) -> Self {
        crate::puerto_toml::read(base)
            .map(|config| Self::in_config(&config, pascal))
            .unwrap_or_default()
    }

    /// The entity `pascal`'s, as `config` records it.
    pub fn in_config(config: &crate::puerto_toml::PuertoConfig, pascal: &str) -> Self {
        let entity = config.entity.iter().find(|e| e.name == pascal);
        Self {
            auth: config.project.has_auth(),
            public: entity.map(|e| e.public.clone()).unwrap_or_default(),
            roles: entity.map(|e| e.roles.clone()).unwrap_or_default(),
        }
    }

    pub fn is_auth(&self) -> bool {
        self.auth
    }

    /// Its route takes `auth: BearerAuth`, its params an `actor`.
    pub fn is_protected(&self, use_case: &str) -> bool {
        self.auth && !self.public.iter().any(|p| p == use_case)
    }

    /// The roles its route requires (`#[require_role]`), none when public.
    pub fn roles(&self, use_case: &str) -> Vec<String> {
        if !self.is_protected(use_case) {
            return vec![];
        }
        self.roles.get(use_case).cloned().unwrap_or_default()
    }

    /// Every role any of `use_cases` requires, once each — what the route tests' caller holds.
    pub fn all_roles(&self, use_cases: &[String]) -> Vec<String> {
        let mut all: Vec<String> = vec![];
        for role in use_cases.iter().flat_map(|uc| self.roles(uc)) {
            if !all.contains(&role) {
                all.push(role);
            }
        }
        all
    }
}

/// A host name has no `_`: the development user's address has to be one.
fn host(project: &str) -> String {
    project.replace('_', "-")
}

fn render_static(template: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(render(template, &tera::Context::new())?)
}

/// A template that differs between the modes: `local` adds what registering and signing in need.
fn render_for(template: &str, local: bool) -> Result<String, Box<dyn std::error::Error>> {
    let mut ctx = tera::Context::new();
    ctx.insert("local", &local);
    Ok(render(template, &ctx)?)
}

/// The `User` entity: model, errors and repository port. Declared in `business/src/lib.rs` — it is
/// not an `[[entity]]`, so no scaffold or `bootstrap.rs` entity block ever touches it.
fn write_user_domain(base: &Path, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join("business/src/domain/user");
    write_file(
        &dir.join("model.rs"),
        &render_for("auth/user_model.tera", local)?,
    )?;
    write_file(
        &dir.join("errors.rs"),
        &render_for("auth/user_errors.tera", local)?,
    )?;
    write_file(
        &dir.join("repository.rs"),
        &render_static("auth/user_repository.tera")?,
    )?;
    // The callers the scaffolded tests pass as `actor`.
    write_file(
        &base.join("business/src/tests/mothers/principal_mother.rs"),
        &render_static("auth/principal_mother.tera")?,
    )?;
    crate::patchers::lib_rs::patch_mothers_lib(base, "principal")?;
    let lib = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&lib)?;
    if !src.contains("pub mod user {") {
        let block = "\n    pub mod user {\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n    }\n";
        fs::write(&lib, insert_before_block_end(&src, "domain", block)?)?;
    }
    Ok(())
}

/// The user repository (in memory, or Postgres with its migration) and, for OIDC, the provisioning
/// authenticator.
fn write_user_infrastructure(
    base: &Path,
    db: bool,
    local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join("infrastructure/src/user");
    let repository = if db {
        "auth/user_repository_pg.tera"
    } else {
        "auth/user_repository_inmemory.tera"
    };
    write_file(&dir.join("repository.rs"), &render_static(repository)?)?;
    if !local {
        write_file(
            &dir.join("provisioning.rs"),
            &render_static("auth/provisioning.tera")?,
        )?;
    }
    if db {
        crate::generators::migration::run_migration(
            "create_users_table",
            base,
            Some(&render_static("auth/users.sql")?),
        )?;
    }
    let lib = base.join("infrastructure/src/lib.rs");
    let mut src = fs::read_to_string(&lib)?;
    if !src.contains("pub mod user {") {
        if !src.is_empty() && !src.ends_with('\n') {
            src.push('\n');
        }
        src.push_str(if local {
            "pub mod user {\n    pub mod repository;\n}\n"
        } else {
            "pub mod user {\n    pub mod provisioning;\n    pub mod repository;\n}\n"
        });
        fs::write(&lib, src)?;
    }
    Ok(())
}

/// `GET /me`: `presentation/src/api/me.rs` and its `dto.rs` and `routes.rs`.
fn write_me_api(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let api = base.join("presentation/src/api");
    write_file(&api.join("me.rs"), &render_static("auth/me.tera")?)?;
    write_file(&api.join("me/dto.rs"), &render_static("auth/me_dto.tera")?)?;
    write_file(
        &api.join("me/routes.rs"),
        &render_static("auth/me_routes.tera")?,
    )?;
    patch_api_rs(base, "me")
}

/// The local use cases, in the order a `lib.rs` declares them.
const LOCAL_USE_CASES: [&str; 3] = ["login", "logout", "register_user"];

/// Local accounts (`PLAN.md` § 16.6): `register_user`, `login`, `logout` and `POST /auth/*`, the
/// credentials and sessions migration after `users`'.
fn write_local_accounts(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let business = base.join("business/src");
    for uc in LOCAL_USE_CASES {
        write_file(
            &business.join(format!("domain/user/use_cases/{uc}.rs")),
            &render_static(&format!("auth/local/{uc}_trait.tera"))?,
        )?;
        write_file(
            &business.join(format!("application/user/{uc}.rs")),
            &render_static(&format!("auth/local/{uc}.tera"))?,
        )?;
    }
    let lib = business.join("lib.rs");
    let src = fs::read_to_string(&lib)?;
    if !src.contains("pub mod register_user;") {
        let decls: String = LOCAL_USE_CASES
            .iter()
            .map(|uc| format!("            pub mod {uc};\n"))
            .collect();
        let src = insert_before_block_end(
            &src,
            "user",
            &format!("        pub mod use_cases {{\n{decls}        }}\n"),
        )?;
        let decls: String = LOCAL_USE_CASES
            .iter()
            .map(|uc| format!("        pub mod {uc};\n"))
            .collect();
        let src = insert_before_block_end(
            &src,
            "application",
            &format!("\n    pub mod user {{\n{decls}    }}\n"),
        )?;
        fs::write(&lib, src)?;
    }

    let api = base.join("presentation/src/api");
    for (file, template) in [
        ("auth.rs", "auth_api"),
        ("auth/dto.rs", "auth_dto"),
        ("auth/error_mapper.rs", "auth_error_mapper"),
        ("auth/responses.rs", "auth_responses"),
        ("auth/routes.rs", "auth_routes"),
    ] {
        write_file(
            &api.join(file),
            &render_static(&format!("auth/local/{template}.tera"))?,
        )?;
    }
    patch_api_rs(base, "auth")?;

    crate::generators::migration::run_migration(
        "create_auth_tables",
        base,
        Some(&render_static("auth/local/auth.sql")?),
    )?;
    Ok(())
}

/// Inserts `line` after the first line of `manifest` that is exactly `after`.
fn insert_after_line(manifest: &str, after: &str, line: &str) -> Option<String> {
    let at = manifest.find(&format!("{after}\n"))? + after.len() + 1;
    Some(format!("{}{line}\n{}", &manifest[..at], &manifest[at..]))
}

/// `puerto-auth` for `bootstrap.rs`, and the mocks the generated tests use.
fn add_dependencies(base: &Path, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    add_workspace_dependency(base, "puerto-auth")?;
    let core = "puerto-core = { workspace = true }";
    let core_mocks = "puerto-core = { workspace = true, features = [\"test-utils\"] }";

    let path = base.join("presentation/Cargo.toml");
    let src = fs::read_to_string(&path)?;
    if !src.contains("puerto-auth") {
        let src = insert_after_line(
            &src,
            core,
            if local {
                "# The password hasher `bootstrap.rs` builds (ADR-0021).\npuerto-auth = { workspace = true, features = [\"local\"] }"
            } else {
                "# The OIDC adapter `bootstrap.rs` builds (ADR-0021).\npuerto-auth = { workspace = true, features = [\"oidc\"] }"
            },
        )
        .and_then(|src| {
            insert_after_line(
                &src,
                "business = { path = \"../business\", features = [\"test-utils\"] }",
                &format!("# `MockAuthenticator`, for the routes that take `BearerAuth`.\n{core_mocks}"),
            )
        })
        .ok_or_else(|| {
            format!(
                "{}: no `{core}` or business test-utils dev-dependency",
                path.display()
            )
        })?;
        fs::write(&path, src)?;
    }

    // `provisioning.rs` tests over `MockUserRepository` and `MockAuthenticator`.
    let path = base.join("infrastructure/Cargo.toml");
    let src = fs::read_to_string(&path)?;
    if !src.contains(core_mocks) {
        let src = insert_after_line(
            &src,
            "[dev-dependencies]",
            &format!(
                "business = {{ path = \"../business\", features = [\"test-utils\"] }}\n{core_mocks}"
            ),
        )
        .ok_or_else(|| format!("{}: no [dev-dependencies]", path.display()))?;
        fs::write(&path, src)?;
    }

    // The local use cases' tests drive the ports' mocks.
    if local {
        let path = base.join("business/Cargo.toml");
        let src = fs::read_to_string(&path)?;
        if !src.contains(core_mocks) {
            let src = insert_after_line(
                &src,
                "[dev-dependencies]",
                &format!("# The auth ports' mocks (`MockPasswordHasher`, …).\n{core_mocks}"),
            )
            .ok_or_else(|| format!("{}: no [dev-dependencies]", path.display()))?;
            fs::write(&path, src)?;
        }
    }
    Ok(())
}

fn keycloak_service(project: &str) -> String {
    format!(
        r#"  keycloak:
    image: {KEYCLOAK_IMAGE}
    container_name: {project}-keycloak
    # Imports infrastructure/keycloak/realm.json on the first start: the `web` client, the
    # `admin` role and a user holding it (`make auth/ui`).
    command: start-dev --import-realm
    ports:
      - "{KEYCLOAK_PORT}:8080"
    environment:
      KC_BOOTSTRAP_ADMIN_USERNAME: admin
      KC_BOOTSTRAP_ADMIN_PASSWORD: admin
      KC_HEALTH_ENABLED: "true"
    volumes:
      - ./infrastructure/keycloak:/opt/keycloak/data/import:ro
    restart: unless-stopped
    healthcheck:
      # The image has no curl: bash's /dev/tcp asks the management port.
      test: ["CMD-SHELL", "exec 3<>/dev/tcp/localhost/9000 && printf 'GET /health/ready HTTP/1.0\r\n\r\n' >&3 && grep -q UP <&3"]
      interval: 5s
      timeout: 5s
      retries: 30
"#
    )
}

fn write_keycloak(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = tera::Context::new();
    ctx.insert("realm", project);
    ctx.insert("project", project);
    ctx.insert("host", &host(project));
    ctx.insert("password", DEV_PASSWORD);
    write_file(
        &base.join("infrastructure/keycloak/realm.json"),
        &render("auth/realm.json.tera", &ctx)?,
    )?;
    add_compose_service(base, "keycloak", &keycloak_service(project))
}

fn add_env(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vars = format!(
        "\n# Auth — the OpenID Connect provider (ADR-0021). Keycloak (docker-compose.yml) in\n\
         # development: `make auth/ui`; sign in as admin@{host}.localhost / {DEV_PASSWORD}.\n\
         # Auth0: AUTH_ISSUER=https://<tenant>.auth0.com/, AUTH_AUDIENCE=<the API identifier>,\n\
         # AUTH_ROLES_CLAIM=<the claim an Action adds, e.g. https://example.com/roles>\n\
         AUTH_ISSUER=http://localhost:{KEYCLOAK_PORT}/realms/{project}\n\
         AUTH_AUDIENCE={AUDIENCE}\nAUTH_ROLES_CLAIM=realm_access.roles\n",
        host = host(project)
    );
    for file in [".env", ".env.example"] {
        let path = base.join(file);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.contains("AUTH_ISSUER") {
            continue;
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&vars);
        fs::write(&path, content)?;
    }
    Ok(())
}

/// `AUTH_SESSION_TTL`, read by `puerto_sqlx::auth::session_ttl_from_env`.
fn add_local_env(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let vars = "\n# Auth — accounts in this database (ADR-0021). How long a session lasts: 30d, 12h, 90m.\n\
                AUTH_SESSION_TTL=30d\n";
    for file in [".env", ".env.example"] {
        let path = base.join(file);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.contains("AUTH_SESSION_TTL") {
            continue;
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(vars);
        fs::write(&path, content)?;
    }
    Ok(())
}

/// The compose targets a project without a database lacks (a `--db` one has them already).
const COMPOSE_MAKEFILE_TARGETS: &str = r#"
.PHONY: docker-compose/up docker-compose/down

DOCKER_COMPOSE := docker compose

docker-compose/up: ## Start the containers
	@echo "${CYAN}Starting containers...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Stop the containers
	@echo "${CYAN}Stopping containers...${NC}"
	$(DOCKER_COMPOSE) down

# The template's `run` keeps its recipe; here it gains the provider `bootstrap.rs` discovers.
run: docker-compose/up
"#;

fn auth_makefile_targets(project: &str) -> String {
    format!(
        r#"
.PHONY: auth/ui

auth/ui: docker-compose/up ## The Keycloak console: users, roles, sessions
	@echo "${{GREEN}}Keycloak: http://localhost:{KEYCLOAK_PORT}/admin/master/console/#/{project} (admin / admin)${{NC}}"
	@echo "${{GREEN}}Sign in to the app as admin@{host}.localhost / {DEV_PASSWORD} (role admin)${{NC}}"
"#,
        host = host(project)
    )
}

/// `make auth/grant`: a role is a row's `roles` entry in a local project, applied from the next
/// request (no cache in between).
fn local_makefile_targets(project: &str) -> String {
    let db_name = project.replace('-', "_");
    format!(
        r#"
.PHONY: auth/grant

auth/grant: docker-compose/up ## Give an account a role: make auth/grant EMAIL=ada@example.com ROLE=admin
	@test -n "$(EMAIL)" -a -n "$(ROLE)" || {{ echo "${{RED}}usage: make auth/grant EMAIL=… ROLE=…${{NC}}"; exit 1; }}
	$(DOCKER_COMPOSE) exec -T postgres psql -U devuser -d {db_name} -v ON_ERROR_STOP=1 -c \
		"UPDATE users SET roles = array_append(roles, '$(ROLE)'), updated_at = now() WHERE issuer = 'local' AND subject = lower(trim('$(EMAIL)')) AND NOT '$(ROLE)' = ANY(roles)"
"#
    )
}

fn add_makefile_targets(
    base: &Path,
    project: &str,
    local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(());
    }
    let mut src = fs::read_to_string(&path)?;
    if src.contains("auth/ui:") || src.contains("auth/grant:") {
        return Ok(());
    }
    if !src.ends_with('\n') {
        src.push('\n');
    }
    if !src.contains("docker-compose/up:") {
        src.push_str(COMPOSE_MAKEFILE_TARGETS);
    }
    src.push_str(&if local {
        local_makefile_targets(project)
    } else {
        auth_makefile_targets(project)
    });
    fs::write(&path, src)?;
    Ok(())
}

/// The frontend's auth files of both modes, written once under `web/` (their templates mirror the
/// paths). The last three replace the template's own: the app runs inside `AuthProvider`, the
/// layout shows the user, and `renderScreen` signs a caller in.
const WEB_FILES: [&str; 11] = [
    "src/domain/auth/session.ts",
    "src/infrastructure/auth/bearer-middleware.ts",
    "src/infrastructure/auth/bearer-middleware.test.ts",
    "src/presentation/core/auth.tsx",
    "src/presentation/core/auth.test.tsx",
    "src/presentation/shared/ui/user-menu.tsx",
    "src/presentation/shared/ui/user-menu.test.tsx",
    "src/test-support/auth.ts",
    "src/main.tsx",
    "src/presentation/shared/ui/layout.tsx",
    "src/test-support/render.tsx",
];

/// An OIDC project's: the provider's client and the page it sends the user back to.
const OIDC_WEB_FILES: [&str; 4] = [
    "src/infrastructure/auth/oidc-auth-gateway.ts",
    "src/infrastructure/auth/oidc-auth-gateway.test.ts",
    "src/presentation/pages/auth-callback.tsx",
    "src/presentation/pages/auth-callback.test.tsx",
];

/// A local project's: the `Accounts` port, the gateway over `/auth/*`, the sign-in and sign-up pages.
const LOCAL_WEB_FILES: [&str; 8] = [
    "src/domain/auth/accounts.ts",
    "src/domain/auth/messages.ts",
    "src/infrastructure/auth/local-auth-gateway.ts",
    "src/infrastructure/auth/local-auth-gateway.test.ts",
    "src/presentation/pages/login.tsx",
    "src/presentation/pages/login.test.tsx",
    "src/presentation/pages/register.tsx",
    "src/presentation/pages/register.test.tsx",
];

/// The dependency-cruiser rule that keeps the provider's client behind `AuthGateway`.
const CRUISER_RULE: &str = r#"    {
      name: "only-the-auth-adapter-imports-oidc-client-ts",
      comment:
        "The provider's client stays behind AuthGateway: screens see a CurrentUser, never a token.",
      severity: "error",
      from: { pathNot: "^src/infrastructure/auth/" },
      to: { path: "oidc-client-ts" },
    },
"#;

/// `oidc-client-ts` in `web/package.json`, before `openapi-fetch` (dependencies stay sorted).
fn add_oidc_client(package: &str) -> Option<String> {
    if package.contains("\"oidc-client-ts\"") {
        return Some(package.to_string());
    }
    let at = package.find("    \"openapi-fetch\": ")?;
    Some(format!(
        "{}    \"oidc-client-ts\": \"{OIDC_CLIENT_TS_VERSION}\",\n{}",
        &package[..at],
        &package[at..]
    ))
}

/// The provider and client `oidc-client-ts` signs in with — read by `oidcSettingsFromEnv()`.
fn web_env(project: &str) -> String {
    format!(
        "# Auth — the OpenID Connect provider the browser signs in with (ADR-0021). Keycloak in\n\
         # development; Auth0: VITE_AUTH_ISSUER=https://<tenant>.auth0.com/, VITE_AUTH_CLIENT_ID=<the\n\
         # SPA's client id>, VITE_AUTH_AUDIENCE=<the API identifier, AUTH_AUDIENCE of the API>.\n\
         VITE_AUTH_ISSUER=http://localhost:{KEYCLOAK_PORT}/realms/{project}\n\
         VITE_AUTH_CLIENT_ID=web\n\
         VITE_AUTH_AUDIENCE=\n"
    )
}

/// The frontend half (`PLAN.md` § 16.5, § 16.7): sign in — with the provider, or with a password
/// — send the token, guard screens.
fn write_web(base: &Path, project: &str, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    let web = base.join("web");
    let mode_files: &[&str] = if local {
        &LOCAL_WEB_FILES
    } else {
        &OIDC_WEB_FILES
    };
    for file in WEB_FILES.iter().chain(mode_files) {
        write_file(
            &web.join(file),
            &render_for(&format!("auth/web/{file}.tera"), local)?,
        )?;
    }
    if local {
        return Ok(());
    }

    let path = web.join("package.json");
    let package = fs::read_to_string(&path)?;
    let package = add_oidc_client(&package)
        .ok_or_else(|| format!("{}: no `openapi-fetch` dependency", path.display()))?;
    fs::write(&path, package)?;

    let path = web.join(".dependency-cruiser.cjs");
    let cruiser = fs::read_to_string(&path)?;
    if !cruiser.contains("oidc-client-ts") {
        let cruiser = insert_after_line(&cruiser, "  forbidden: [", CRUISER_RULE.trim_end())
            .ok_or_else(|| format!("{}: no `forbidden: [`", path.display()))?;
        fs::write(&path, cruiser)?;
    }

    for file in [".env", ".env.example"] {
        let path = web.join(file);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.contains("VITE_AUTH_ISSUER") {
            continue;
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&web_env(project));
        fs::write(&path, content)?;
    }
    Ok(())
}

/// The root `AGENTS.md` section an auth project gets (ADR-0010: what it names is tested).
pub const AGENTS_AUTH_SECTION: &str = r#"## Auth (`[project] auth`)

The API is an OAuth2 resource server (ADR-0021 of Puerto): it verifies the bearer token of every
protected request against the provider at `AUTH_ISSUER` — Keycloak in development
(`make docker-compose/up`, `make auth/ui`), Auth0 or any OpenID Connect provider in production.

- A handler that takes `auth: BearerAuth` is protected: no or a bad token is answered
  `401 auth.unauthenticated` before it runs. `#[require_role("admin")]` on it adds the
  `403 auth.forbidden` check. Both come from `puerto_web`.
- `puerto generate scaffold <Name>` protects every CRUD route. `--public list_products,get_product`
  leaves routes open; `--role delete_product=admin` adds `#[require_role]` (two roles on one route:
  the caller needs both). Both land in the entity's `public` / `roles` in `puerto.toml`, and
  `puerto validate` checks `routes.rs` still says the same — change both together.
- A protected use case's params carry `actor: Principal` (the handler passes `auth.0`); tests
  build one with `PrincipalMother`. `generate use-case` adds it too.
- **Roles come from the token only** (`AUTH_ROLES_CLAIM`); the `users.roles` column is not read.
- Every caller is a `User` (`business/src/domain/user/`), created on their first request by
  `infrastructure/src/user/provisioning.rs`. Point at a person with a `UserId`, never with the
  provider's subject. These files are yours.
- `GET /me` answers the caller as the API resolved them. The frontend reads it, never the token.
- A rule that depends on data ("only the owner edits") belongs in the use case, not in an
  attribute.

"#;

/// The root `AGENTS.md` section of a project that keeps its own accounts (`--auth local`).
pub const AGENTS_AUTH_LOCAL_SECTION: &str = r#"## Auth (`[project] auth`)

The API keeps its own accounts (ADR-0021 of Puerto): `POST /auth/register` creates one,
`POST /auth/login` answers an opaque session token, `POST /auth/logout` revokes it. A client sends
the token as `Authorization: Bearer <token>`; the database keeps only its SHA-256.

- A handler that takes `auth: BearerAuth` is protected: no, an unknown or an expired session is
  answered `401 auth.unauthenticated` before it runs. `#[require_role("admin")]` on it adds the
  `403 auth.forbidden` check. Both come from `puerto_web`.
- `puerto generate scaffold <Name>` protects every CRUD route. `--public list_products,get_product`
  leaves routes open; `--role delete_product=admin` adds `#[require_role]` (two roles on one route:
  the caller needs both). Both land in the entity's `public` / `roles` in `puerto.toml`, and
  `puerto validate` checks `routes.rs` still says the same — change both together.
- A protected use case's params carry `actor: Principal` (the handler passes `auth.0`); tests
  build one with `PrincipalMother`. `generate use-case` adds it too.
- **Roles are the `users.roles` column**: `make auth/grant EMAIL=… ROLE=…`. They apply from the
  next request.
- Passwords are argon2id hashes in `credentials`; `User` carries no secret. A login with an unknown
  email and one with a wrong password get the same `401 auth.invalid_credentials`, in the same time.
  Sessions last `AUTH_SESSION_TTL` (`.env`); expired rows are refused, pruning them is yours.
- `register_user`, `login`, `logout` (`business/src/{domain,application}/user/`) and
  `presentation/src/api/auth/` are yours. Registration is open: close it, or add rules, there.
- `GET /me` answers the caller as the API resolved them.
- A rule that depends on data ("only the owner edits") belongs in the use case, not in an
  attribute.

"#;

/// What follows [`AGENTS_AUTH_SECTION`] in a project with `web/`.
pub const AGENTS_AUTH_WEB_SECTION: &str = r#"### Auth in `web/`

- `useAuth()` (`presentation/core/auth.tsx`) gives `{ user, can(role), login, logout }`; `user` is
  `GET /me`, never the token. Generated screens hide what `can` denies, but the API decides: its
  `403` reaches the alert as `auth.forbidden`.
- `core/generated/routes.tsx` wraps every non-public screen in `<RequireAuth>`, which signs in and
  comes back. Wrap your own routes in `routes.tsx` the same way.
- The token stays in memory and `oidc-client-ts` stays in `infrastructure/auth/`
  (`make web/lint` checks it): a reload signs in again through the provider's session.
  `web/.env` holds `VITE_AUTH_*`.
- Screen tests sign in a caller holding every role (`renderScreen`); pass `{ user }` for another one
  (`CurrentUserMother`), or `{ auth: fakeAuthGateway(…) }` to see the sign-ins it starts.

"#;

/// What follows [`AGENTS_AUTH_LOCAL_SECTION`] in a project with `web/`.
pub const AGENTS_AUTH_LOCAL_WEB_SECTION: &str = r#"### Auth in `web/`

- `useAuth()` (`presentation/core/auth.tsx`) gives `{ user, can(role), login, logout }`; `user` is
  `GET /me`. Generated screens hide what `can` denies, but the API decides: its `403` reaches the
  alert as `auth.forbidden`.
- `core/generated/routes.tsx` wraps every non-public screen in `<RequireAuth>`, which sends whoever
  is not signed in to `/login?next=…`, as a `401` from any call does. Wrap your own routes in
  `routes.tsx` the same way.
- `/login` and `/register` (`presentation/pages/`) sign in through `Accounts`
  (`domain/auth/accounts.ts`); `domain/auth/messages.ts` turns the API's codes into what the forms
  say, under the field they are about. These files are yours.
- The session token is in `localStorage` (`infrastructure/auth/local-auth-gateway.ts`): a reload
  stays signed in; Sign out revokes it on the API.
- Screen tests sign in a caller holding every role (`renderScreen`); pass `{ user }` for another one
  (`CurrentUserMother`), `{ auth: fakeAuthGateway(…) }` or `{ accounts: fakeAccounts() }` to see
  what a screen asked.

"#;

fn add_agents_section(
    base: &Path,
    web: bool,
    local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("AGENTS.md");
    let Ok(agents) = fs::read_to_string(&path) else {
        return Ok(());
    };
    if agents.contains("## Auth (`[project] auth`)") {
        return Ok(());
    }
    let section = if local && web {
        format!("{AGENTS_AUTH_LOCAL_SECTION}{AGENTS_AUTH_LOCAL_WEB_SECTION}")
    } else if local {
        AGENTS_AUTH_LOCAL_SECTION.to_string()
    } else if web {
        format!("{AGENTS_AUTH_SECTION}{AGENTS_AUTH_WEB_SECTION}")
    } else {
        AGENTS_AUTH_SECTION.to_string()
    };
    // Before the conventions: next to the rules about what is generated.
    let out = match agents.find("## Conventions") {
        Some(at) => format!("{}{section}{}", &agents[..at], &agents[at..]),
        None => format!("{agents}\n{section}"),
    };
    fs::write(&path, out)?;
    Ok(())
}

/// Turns a fresh project into an auth one (`puerto new --auth oidc|local`). The Greeting demo
/// stays open: it is listed `public`.
pub fn apply_auth_to_new_project(
    base: &Path,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    check_mode(mode)?;
    let mut config = crate::puerto_toml::read(base)?;
    config.project.auth = Some(mode.to_string());
    for entity in &mut config.entity {
        if entity.name == "Greeting" && entity.public.is_empty() {
            entity.public = entity.use_cases.clone();
        }
    }
    crate::puerto_toml::write(base, &config)?;
    let project = config.project.name.clone();

    let local = mode == "local";

    write_user_domain(base, local)?;
    write_user_infrastructure(base, config.project.db, local)?;
    write_me_api(base)?;
    add_dependencies(base, local)?;
    if local {
        write_local_accounts(base)?;
        add_local_env(base)?;
    } else {
        write_keycloak(base, &project)?;
        add_env(base, &project)?;
    }
    add_makefile_targets(base, &project, local)?;
    if config.project.web {
        write_web(base, &project, local)?;
    }
    add_agents_section(base, config.project.web, local)?;

    crate::generators::bootstrap::regenerate_bootstrap(base)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_write_local_accounts_only_with_a_database() {
        assert!(check_new_project("oidc", false).is_ok());
        assert!(check_new_project("local", true).is_ok());
        assert!(
            check_new_project("local", false)
                .unwrap_err()
                .contains("--db")
        );
        assert!(
            check_new_project("auth0", true)
                .unwrap_err()
                .contains("unknown auth mode")
        );
    }

    #[test]
    fn should_insert_after_the_exact_line_when_the_manifest_has_it() {
        let manifest = "[dependencies]\npuerto-core = { workspace = true }\npoem = \"3\"\n";
        assert_eq!(
            insert_after_line(manifest, "puerto-core = { workspace = true }", "x = 1"),
            Some(
                "[dependencies]\npuerto-core = { workspace = true }\nx = 1\npoem = \"3\"\n".into()
            )
        );
        assert_eq!(
            insert_after_line(manifest, "[dev-dependencies]", "x = 1"),
            None
        );
    }

    fn access(auth: bool, public: &[&str], roles: &[(&str, &[&str])]) -> Access {
        Access {
            auth,
            public: public.iter().map(|s| s.to_string()).collect(),
            roles: roles
                .iter()
                .map(|(uc, r)| (uc.to_string(), r.iter().map(|s| s.to_string()).collect()))
                .collect(),
        }
    }

    #[test]
    fn should_protect_every_use_case_but_the_public_ones_when_the_project_has_auth() {
        let access = access(true, &["list_products"], &[]);
        assert!(access.is_protected("create_product"));
        assert!(!access.is_protected("list_products"));
    }

    #[test]
    fn should_protect_nothing_when_the_project_has_no_auth() {
        let access = access(false, &[], &[("create_product", &["admin"])]);
        assert!(!access.is_protected("create_product"));
        assert_eq!(access.roles("create_product"), Vec::<String>::new());
    }

    #[test]
    fn should_collect_each_role_once_when_use_cases_share_it() {
        let access = access(
            true,
            &[],
            &[
                ("create_product", &["admin"]),
                ("delete_product", &["admin", "owner"]),
            ],
        );
        let use_cases = ["create_product".to_string(), "delete_product".to_string()];
        assert_eq!(access.roles("delete_product"), vec!["admin", "owner"]);
        assert_eq!(access.all_roles(&use_cases), vec!["admin", "owner"]);
    }

    #[test]
    fn should_add_the_oidc_client_before_openapi_fetch_once() {
        let package =
            "  \"dependencies\": {\n    \"openapi-fetch\": \"0.17.0\",\n    \"react\": \"19\"\n";
        let added = add_oidc_client(package).unwrap();
        assert_eq!(
            added,
            format!(
                "  \"dependencies\": {{\n    \"oidc-client-ts\": \"{OIDC_CLIENT_TS_VERSION}\",\n    \"openapi-fetch\": \"0.17.0\",\n    \"react\": \"19\"\n"
            )
        );
        assert_eq!(add_oidc_client(&added).unwrap(), added);
        assert_eq!(add_oidc_client("{}"), None);
    }

    #[test]
    fn should_make_a_host_name_when_the_project_name_has_underscores() {
        assert_eq!(host("my_shop"), "my-shop");
    }
}
