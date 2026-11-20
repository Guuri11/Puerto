//! `puerto nuevo --auth oidc` (ADR-0021): the `Usuario` entity every caller becomes, the provisioning
//! `Autenticador` in front of the provider, `GET /me`, and Keycloak for development. Every file
//! here is written once — the project's from then on.

use crate::generators::layout::{self, Layer};
use std::fs;
use std::path::Path;

use crate::generators::naming::write_file;
use crate::generators::project::{add_compose_service, add_workspace_dependency};
use crate::generators::render::render;

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

/// A mode `puerto nuevo --auth` knows.
pub fn check_mode(mode: &str) -> Result<(), String> {
    crate::validation::validate_auth_mode(mode)
}

/// Whether `puerto nuevo --auth <mode>` can write a project with this database choice — asked
/// before anything is written. Local accounts live in Postgres.
pub fn check_new_project(mode: &str, db: bool) -> Result<(), String> {
    check_mode(mode)?;
    if mode == "local" && !db {
        return Err("--auth local guarda las cuentas en la base de datos: añade --db".into());
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
    /// 27.18: the use cases whose roles the use case checks, not the route — `obtener`,
    /// `actualizar` and `borrar` of an owned entity, which its owner reaches without them.
    owner_checked: Vec<String>,
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
        let owner_checked = entity
            .filter(|e| e.owner.is_some())
            .map(|e| {
                let snake = crate::generators::naming::pascal_to_snake(&e.name);
                ["get", "update", "delete"]
                    .iter()
                    .map(|action| crate::generators::naming::crud_use_case(action, &snake))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            auth: config.project.has_auth(),
            public: entity.map(|e| e.public.clone()).unwrap_or_default(),
            roles: entity.map(|e| e.roles.clone()).unwrap_or_default(),
            owner_checked,
        }
    }

    pub fn is_auth(&self) -> bool {
        self.auth
    }

    /// Its route takes `auth: BearerAuth`, its params an `actor`.
    pub fn is_protected(&self, use_case: &str) -> bool {
        self.auth && !self.public.iter().any(|p| p == use_case)
    }

    /// The roles its route requires (`#[requiere_rol]`), none when public.
    pub fn roles(&self, use_case: &str) -> Vec<String> {
        if !self.is_protected(use_case) {
            return vec![];
        }
        self.roles.get(use_case).cloned().unwrap_or_default()
    }

    /// The entity has an owner (27.18): its owner reads, edits and deletes its rows.
    pub fn is_owned(&self) -> bool {
        !self.owner_checked.is_empty()
    }

    /// The roles its route requires (`#[requiere_rol]`): [`Self::roles`], but none for a use case
    /// that checks them itself, next to the owner (27.18).
    pub fn route_roles(&self, use_case: &str) -> Vec<String> {
        if self.owner_checked.iter().any(|uc| uc == use_case) {
            return vec![];
        }
        self.roles(use_case)
    }

    /// Every role any of `use_cases`' routes requires, once each — what the route tests' caller
    /// holds.
    pub fn all_roles(&self, use_cases: &[String]) -> Vec<String> {
        let mut all: Vec<String> = vec![];
        for role in use_cases.iter().flat_map(|uc| self.route_roles(uc)) {
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

/// One crate of the `usuario` context: its files, written once, then its `Cargo.toml`, `lib.rs`
/// and workspace entry (ADR-0023: it is not an `[[entidad]]`, so no scaffold or `cableado` entity
/// block touches it).
fn write_user_crate(
    base: &Path,
    layer: Layer,
    files: &[(&str, String)],
    db: bool,
    local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let c = layout::USER_CONTEXT;
    let dir = layout::crate_dir(c, layer);
    for (file, content) in files {
        write_file(&base.join(&dir).join("src").join(file), content)?;
    }
    let template = match layer {
        Layer::Domain => "contexto/usuario_dominio_cargo.tera",
        Layer::Application => "contexto/usuario_aplicacion_cargo.tera",
        Layer::Infrastructure => "contexto/usuario_infra_cargo.tera",
        Layer::Rest => "contexto/usuario_rest_cargo.tera",
    };
    let mut ctx = tera::Context::new();
    ctx.insert("db", &db);
    ctx.insert("local", &local);
    write_file(
        &base.join(layout::crate_manifest(c, layer)),
        &render(template, &ctx)?,
    )?;
    crate::generators::crates::write_lib(base, &dir, "")?;
    crate::generators::crates::add_internal_dependency(base, &layout::crate_name(c, layer), &dir)?;
    crate::generators::crates::sync_workspace_dependencies(base)?;
    Ok(())
}

/// The `Usuario` entity: model, errors and repository port.
fn write_user_domain(base: &Path, db: bool, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    write_user_crate(
        base,
        Layer::Domain,
        &[
            ("modelo.rs", render_for("auth/user_model.tera", local)?),
            ("errores.rs", render_for("auth/user_errors.tera", local)?),
            (
                "repositorio.rs",
                render_static("auth/user_repository.tera")?,
            ),
        ],
        db,
        local,
    )
}

/// The user repository (in memory, or Postgres with its migration) and, for OIDC, the provisioning
/// authenticator.
fn write_user_infrastructure(
    base: &Path,
    db: bool,
    local: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = if db {
        "auth/user_repository_pg.tera"
    } else {
        "auth/user_repository_inmemory.tera"
    };
    let mut files = vec![("repositorio.rs", render_static(repository)?)];
    if !local {
        files.push((
            "aprovisionamiento.rs",
            render_static("auth/provisioning.tera")?,
        ));
    }
    write_user_crate(base, Layer::Infrastructure, &files, db, local)?;
    if db {
        crate::generators::migration::run_migration(
            "crear_tabla_usuarios",
            base,
            Some(&render_static("auth/users.sql")?),
        )?;
    }
    Ok(())
}

/// The local use cases: one file each, contract and implementation (ADR-0023 § 4).
const LOCAL_USE_CASES: [&str; 3] = ["login", "logout", "registrar_usuario"];

/// The `usuario` REST crate: `GET /me` and, with local accounts, `POST /auth/*`.
fn write_user_rest(base: &Path, db: bool, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = vec![
        ("me.rs", render_static("auth/me.tera")?),
        ("me/dto.rs", render_static("auth/me_dto.tera")?),
        ("me/rutas.rs", render_static("auth/me_routes.tera")?),
    ];
    if local {
        for (file, template) in [
            ("auth.rs", "auth_api"),
            ("auth/dto.rs", "auth_dto"),
            ("auth/errores.rs", "auth_error_mapper"),
            ("auth/respuestas.rs", "auth_responses"),
            ("auth/rutas.rs", "auth_routes"),
        ] {
            files.push((file, render_static(&format!("auth/local/{template}.tera"))?));
        }
    }
    write_user_crate(base, Layer::Rest, &files, db, local)
}

/// Local accounts (`PLAN.md` § 16.6): `registrar_usuario`, `login`, `logout`, the credentials and
/// sessions migration after `users`'.
fn write_local_accounts(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = vec![];
    for uc in LOCAL_USE_CASES {
        files.push((
            uc,
            crate::generators::merge::merge_use_case(
                &render_static(&format!("auth/local/{uc}_trait.tera"))?,
                &render_static(&format!("auth/local/{uc}.tera"))?,
            ),
        ));
    }
    let files: Vec<(String, String)> = files
        .into_iter()
        .map(|(uc, content)| (format!("{uc}.rs"), content))
        .collect();
    let files: Vec<(&str, String)> = files.iter().map(|(f, c)| (f.as_str(), c.clone())).collect();
    write_user_crate(base, Layer::Application, &files, true, true)?;

    crate::generators::migration::run_migration(
        "crear_tablas_auth",
        base,
        Some(&render_static("auth/local/auth.sql")?),
    )?;
    Ok(())
}

/// `puerto-auth` and the `usuario` crates for the `api` binary: `bootstrap.rs` builds the
/// authenticator, the user repository and, with local accounts, the use cases and the hasher.
fn add_dependencies(base: &Path, local: bool) -> Result<(), Box<dyn std::error::Error>> {
    use crate::generators::bootstrap::add_crate_dependency;
    add_workspace_dependency(base, "puerto-auth")?;
    let feature = if local { "local" } else { "oidc" };
    add_crate_dependency(
        base,
        layout::API_MANIFEST,
        "dependencies",
        "puerto-auth",
        &format!("{{ workspace = true, features = [\"{feature}\"] }}"),
    )?;
    add_crate_dependency(
        base,
        layout::API_MANIFEST,
        "dependencies",
        "puerto-nucleo",
        "{ workspace = true }",
    )?;
    let c = layout::USER_CONTEXT;
    let mut layers = vec![Layer::Domain, Layer::Infrastructure, Layer::Rest];
    if local {
        layers.push(Layer::Application);
    }
    for layer in layers {
        add_crate_dependency(
            base,
            layout::API_MANIFEST,
            "dependencies",
            &layout::crate_name(c, layer),
            "{ workspace = true }",
        )?;
    }
    if local {
        // The spec export test builds `AuthApi` from the use cases' doubles.
        add_crate_dependency(
            base,
            layout::API_MANIFEST,
            "dev-dependencies",
            &layout::crate_name(c, Layer::Application),
            "{ workspace = true, features = [\"pruebas\"] }",
        )?;
    }
    Ok(())
}

/// Inserts `line` after the first line of `manifest` that is exactly `after`.
fn insert_after_line(manifest: &str, after: &str, line: &str) -> Option<String> {
    let at = manifest.find(&format!("{after}\n"))? + after.len() + 1;
    Some(format!("{}{line}\n{}", &manifest[..at], &manifest[at..]))
}

fn keycloak_service(project: &str) -> String {
    format!(
        r#"  keycloak:
    image: {KEYCLOAK_IMAGE}
    container_name: {project}-keycloak
    # Importa keycloak/realm.json en el primer arranque: el cliente `web`, el
    # rol `admin` y un usuario que lo tiene (`make auth/ui`).
    command: start-dev --import-realm
    ports:
      - "{KEYCLOAK_PORT}:8080"
    environment:
      KC_BOOTSTRAP_ADMIN_USERNAME: admin
      KC_BOOTSTRAP_ADMIN_PASSWORD: admin
      KC_HEALTH_ENABLED: "true"
    volumes:
      - ./keycloak:/opt/keycloak/data/import:ro
    restart: unless-stopped
    healthcheck:
      # La imagen no trae curl: el /dev/tcp de bash pregunta al puerto de gestión.
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
        &base.join(layout::KEYCLOAK_REALM),
        &render("auth/realm.json.tera", &ctx)?,
    )?;
    add_compose_service(base, "keycloak", &keycloak_service(project))
}

fn add_env(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vars = format!(
        "\n# Auth — el proveedor OpenID Connect (ADR-0021). Keycloak (docker-compose.yml) en\n\
         # desarrollo: `make auth/ui`; inicia sesión como admin@{host}.localhost / {DEV_PASSWORD}.\n\
         # Auth0: AUTH_ISSUER=https://<tenant>.auth0.com/, AUTH_AUDIENCE=<el identificador de la API>,\n\
         # AUTH_ROLES_CLAIM=<el claim que añade una Action, p. ej. https://example.com/roles>\n\
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

/// `AUTH_SESSION_TTL`, read by `puerto_sqlx::auth::ttl_sesion_from_env`.
fn add_local_env(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let vars = "\n# Auth — las cuentas, en esta base de datos (ADR-0021). Lo que dura una sesión: 30d, 12h, 90m.\n\
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

docker-compose/up: ## Arranca los contenedores
	@echo "${CYAN}Arrancando los contenedores...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Para los contenedores
	@echo "${CYAN}Parando los contenedores...${NC}"
	$(DOCKER_COMPOSE) down

# El `run` de la plantilla conserva su receta; aquí gana el proveedor que descubre `bootstrap.rs`.
run: docker-compose/up
"#;

fn auth_makefile_targets(project: &str) -> String {
    format!(
        r#"
.PHONY: auth/ui auth/token

auth/ui: docker-compose/up ## La consola de Keycloak: usuarios, roles, sesiones
	@echo "${{GREEN}}Keycloak: http://localhost:{KEYCLOAK_PORT}/admin/master/console/#/{project} (admin / admin)${{NC}}"
	@echo "${{GREEN}}Inicia sesión en la app como admin@{host}.localhost / {DEV_PASSWORD} (rol admin)${{NC}}"

# El cliente `dev-cli` del realm de desarrollo (password grant): curl -H "Authorization: Bearer
# $(make -s auth/token)" … Un EMAIL del entorno de la shell no se toma por uno.
AUTH_ISSUER ?= $(shell sed -n 's/^AUTH_ISSUER=//p' .env 2>/dev/null)
TOKEN_EMAIL = $(if $(filter command line,$(origin EMAIL)),$(EMAIL),admin@{host}.localhost)

auth/token: ## Un token de acceso de desarrollo: make -s auth/token [EMAIL=user@{host}.localhost]
	@curl -sf "$(AUTH_ISSUER)/protocol/openid-connect/token" -d grant_type=password -d client_id=dev-cli \
		--data-urlencode "username=$(TOKEN_EMAIL)" --data-urlencode "password={DEV_PASSWORD}" \
		| sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p' | grep . \
		|| {{ echo "${{RED}}sin token para $(TOKEN_EMAIL) de $(AUTH_ISSUER): ¿está arrancado Keycloak? make docker-compose/up${{NC}}" >&2; exit 1; }}
"#,
        host = host(project)
    )
}

/// `make auth/conceder`: a role is a row's `roles` entry in a local project, applied from the next
/// request (no cache in between).
fn local_makefile_targets(project: &str) -> String {
    let db_name = project.replace('-', "_");
    format!(
        r#"
.PHONY: auth/conceder

auth/conceder: docker-compose/up ## Concede un rol a una cuenta: make auth/conceder EMAIL=ada@example.com ROL=admin
	@test -n "$(EMAIL)" -a -n "$(ROL)" || {{ echo "${{RED}}uso: make auth/conceder EMAIL=… ROL=…${{NC}}"; exit 1; }}
	$(DOCKER_COMPOSE) exec -T postgres psql -U devuser -d {db_name} -v ON_ERROR_STOP=1 -c \
		"UPDATE usuarios SET roles = array_append(roles, '$(ROL)'), actualizado_en = now() WHERE emisor = 'local' AND sujeto = lower(trim('$(EMAIL)')) AND NOT '$(ROL)' = ANY(roles)"
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
    if src.contains("auth/ui:") || src.contains("auth/conceder:") {
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
/// paths). The last two replace the template's own: the frame shows the user, and
/// `renderizarPantalla` signs a caller in. `main.tsx` (the app inside `ProveedorAuth`, the access
/// pages outside the app's frame) is `project::write_main`'s, which also knows the portada.
const WEB_FILES: [&str; 15] = [
    "src/auth/sesion.ts",
    "src/auth/middleware-bearer.ts",
    "src/auth/middleware-bearer.test.ts",
    "src/auth/auth.tsx",
    "src/auth/auth.test.tsx",
    "src/compartido/ui/menu-usuario.tsx",
    "src/compartido/ui/menu-usuario.test.tsx",
    "src/compartido/ui/avatar.tsx",
    "src/compartido/ui/avatar.test.tsx",
    "src/compartido/ui/marco-acceso.tsx",
    "src/paginas/perfil.tsx",
    "src/paginas/perfil.test.tsx",
    "src/pruebas/auth.ts",
    "src/compartido/ui/marco.tsx",
    "src/pruebas/renderizar.tsx",
];

/// An OIDC project's: the provider's client and the page it sends the user back to.
const OIDC_WEB_FILES: [&str; 4] = [
    "src/auth/pasarela-oidc.ts",
    "src/auth/pasarela-oidc.test.ts",
    "src/paginas/auth-callback.tsx",
    "src/paginas/auth-callback.test.tsx",
];

/// A local project's: the `Accounts` port, the gateway over `/auth/*`, the sign-in and sign-up pages.
const LOCAL_WEB_FILES: [&str; 8] = [
    "src/auth/cuentas.ts",
    "src/auth/mensajes.ts",
    "src/auth/pasarela-local.ts",
    "src/auth/pasarela-local.test.ts",
    "src/paginas/login.tsx",
    "src/paginas/login.test.tsx",
    "src/paginas/registro.tsx",
    "src/paginas/registro.test.tsx",
];

/// The dependency-cruiser rule that keeps the provider's client behind `PasarelaAuth`.
const CRUISER_RULE: &str = r#"    {
      name: "solo-la-pasarela-importa-oidc-client-ts",
      comment:
        "El cliente del proveedor se queda detrás de PasarelaAuth: las pantallas ven un UsuarioActual, nunca un token.",
      severity: "error",
      from: { pathNot: "^src/auth/pasarela-oidc" },
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
        "# Auth — el proveedor OpenID Connect con el que inicia sesión el navegador (ADR-0021).\n\
         # Keycloak en desarrollo; Auth0: VITE_AUTH_ISSUER=https://<tenant>.auth0.com/,\n\
         # VITE_AUTH_CLIENT_ID=<el client id de la SPA>, VITE_AUTH_AUDIENCE=<el identificador de la\n\
         # API, el AUTH_AUDIENCE de la API>.\n\
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
    let package = add_oidc_client(&package).ok_or_else(|| {
        format!(
            "{}: no tiene la dependencia `openapi-fetch`",
            path.display()
        )
    })?;
    fs::write(&path, package)?;

    let path = web.join(".dependency-cruiser.cjs");
    let cruiser = fs::read_to_string(&path)?;
    if !cruiser.contains("oidc-client-ts") {
        let cruiser = insert_after_line(&cruiser, "  forbidden: [", CRUISER_RULE.trim_end())
            .ok_or_else(|| format!("{}: no tiene `forbidden: [`", path.display()))?;
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
pub const AGENTS_AUTH_SECTION: &str = r#"## Auth (`[proyecto] auth`)

La API es un servidor de recursos OAuth2 (ADR-0021 de Puerto): verifica el token bearer de cada
petición protegida contra el proveedor de `AUTH_ISSUER`: Keycloak en desarrollo
(`make docker-compose/up`, `make auth/ui`), Auth0 o cualquier proveedor OpenID Connect en
producción.

- Un handler que recibe `auth: BearerAuth` está protegido: sin token o con uno malo se responde
  `401 auth.no_autenticado` antes de ejecutarlo. Un `#[requiere_rol("admin")]` encima añade la
  comprobación `403 auth.prohibido`. Las dos cosas vienen de `puerto_web`.
- `puerto generar scaffold <Nombre>` protege todas las rutas CRUD. `--publico
  listar_productos,obtener_producto` deja rutas abiertas; `--rol borrar_producto=admin` añade
  `#[requiere_rol]` (dos roles en una ruta: quien llama necesita los dos). Las dos opciones quedan
  en `publicos` / `roles` de la entidad en `puerto.toml`, y `puerto validar` comprueba que
  `rutas.rs` sigue diciendo lo mismo: cambia los dos a la vez.
- Los params de un caso de uso protegido llevan `actor: Principal` (el handler pasa `auth.0`); los
  tests construyen uno con `puerto_nucleo::auth::dobles::MadrePrincipal`. `generar caso-de-uso`
  también lo añade.
- **Los roles salen solo del token** (`AUTH_ROLES_CLAIM`); la columna `usuarios.roles` no se lee.
- Para llamar a la API a mano: `curl -H "Authorization: Bearer $(make -s auth/token)" …`;
  `admin@…` tiene `admin`, y `make -s auth/token EMAIL=user@…` ningún rol. Lo pide al cliente
  `dev-cli` del realm de desarrollo (password grant): no crees nunca un cliente así en un proveedor
  real.
- Cada quien llama es un `Usuario` (`contextos/usuario/dominio/`), que crea en su primera petición
  `contextos/usuario/infraestructura/src/aprovisionamiento.rs`. Señala a una persona con un
  `IdUsuario`, nunca con el sujeto del proveedor. Estos ficheros son tuyos.
- `GET /me` responde quién llama, tal como lo resolvió la API. El frontend lo lee a él, nunca el
  token.
- Una regla que depende de los datos («solo edita el propietario») va en el caso de uso, no en un
  atributo.

"#;

/// The root `AGENTS.md` section of a project that keeps its own accounts (`--auth local`).
pub const AGENTS_AUTH_LOCAL_SECTION: &str = r#"## Auth (`[proyecto] auth`)

La API guarda sus propias cuentas (ADR-0021 de Puerto): `POST /auth/registro` crea una,
`POST /auth/login` responde un token de sesión opaco y `POST /auth/logout` lo revoca. Un cliente
envía el token como `Authorization: Bearer <token>`; la base de datos solo guarda su SHA-256.

- Un handler que recibe `auth: BearerAuth` está protegido: sin sesión, o con una desconocida o
  caducada, se responde `401 auth.no_autenticado` antes de ejecutarlo. Un `#[requiere_rol("admin")]`
  encima añade la comprobación `403 auth.prohibido`. Las dos cosas vienen de `puerto_web`.
- `puerto generar scaffold <Nombre>` protege todas las rutas CRUD. `--publico
  listar_productos,obtener_producto` deja rutas abiertas; `--rol borrar_producto=admin` añade
  `#[requiere_rol]` (dos roles en una ruta: quien llama necesita los dos). Las dos opciones quedan
  en `publicos` / `roles` de la entidad en `puerto.toml`, y `puerto validar` comprueba que
  `rutas.rs` sigue diciendo lo mismo: cambia los dos a la vez.
- Los params de un caso de uso protegido llevan `actor: Principal` (el handler pasa `auth.0`); los
  tests construyen uno con `puerto_nucleo::auth::dobles::MadrePrincipal`. `generar caso-de-uso`
  también lo añade.
- **Los roles son la columna `usuarios.roles`**: `make auth/conceder EMAIL=… ROL=…`. Se aplican
  desde la petición siguiente.
- Las contraseñas son hashes argon2id en `credenciales`; `Usuario` no lleva ningún secreto. Un login
  con un email desconocido y otro con una contraseña equivocada reciben el mismo
  `401 auth.credenciales_invalidas`, en el mismo tiempo. Las sesiones duran `AUTH_SESSION_TTL`
  (`.env`); las filas caducadas se rechazan, y podarlas te toca a ti.
- `registrar_usuario`, `login`, `logout` (`contextos/usuario/aplicacion/`) y
  `contextos/usuario/rest/src/auth/` son tuyos. El registro está abierto: ciérralo, o añade reglas,
  ahí.
- `GET /me` responde quién llama, tal como lo resolvió la API.
- Una regla que depende de los datos («solo edita el propietario») va en el caso de uso, no en un
  atributo.

"#;

/// What follows [`AGENTS_AUTH_SECTION`] in a project with `web/`.
pub const AGENTS_AUTH_WEB_SECTION: &str = r#"### Auth en `web/`

- `useAuth()` (`auth/auth.tsx`) da `{ usuario, puede(rol), login, logout }`; `usuario` es
  `GET /me`, nunca el token. Las pantallas generadas esconden lo que `puede` niega, pero decide la
  API: su `403` llega a la alerta como `auth.prohibido`.
- `generado/rutas.tsx` envuelve cada pantalla no pública en `<RequiereAuth>`, que inicia sesión y
  vuelve. Envuelve igual tus propias rutas en `rutas.tsx`.
- `/auth/callback` va en `main.tsx`, dentro de `MarcoAcceso` (sin el menú de la app); `/perfil`
  (`paginas/perfil.tsx`) muestra lo que responde `GET /me` y se abre desde el menú de usuario.
- El token se queda en memoria y `oidc-client-ts` se queda en `auth/pasarela-oidc.ts`
  (`make web/lint` lo comprueba): al recargar se vuelve a iniciar sesión con la sesión del
  proveedor. `web/.env` guarda `VITE_AUTH_*`.
- Los tests de pantalla inician sesión con alguien que tiene todos los roles
  (`renderizarPantalla`); pasa `{ usuario }` para otro (`MadreUsuarioActual`), o
  `{ auth: pasarelaAuthFalsa(…) }` para ver los inicios de sesión que lanza.

"#;

/// What follows [`AGENTS_AUTH_LOCAL_SECTION`] in a project with `web/`.
pub const AGENTS_AUTH_LOCAL_WEB_SECTION: &str = r#"### Auth en `web/`

- `useAuth()` (`auth/auth.tsx`) da `{ usuario, puede(rol), login, logout }`; `usuario` es
  `GET /me`. Las pantallas generadas esconden lo que `puede` niega, pero decide la API: su `403`
  llega a la alerta como `auth.prohibido`.
- `generado/rutas.tsx` envuelve cada pantalla no pública en `<RequiereAuth>`, que manda a quien no
  tiene sesión iniciada a `/login?siguiente=…`, igual que un `401` de cualquier llamada. Envuelve
  igual tus propias rutas en `rutas.tsx`.
- `/login` y `/registro` (`paginas/`) inician sesión a través de `Cuentas` (`auth/cuentas.ts`);
  `auth/mensajes.ts` convierte los códigos de la API en lo que dicen los formularios, bajo el campo
  al que se refieren. `main.tsx` las monta dentro de `MarcoAcceso` (la marca a un lado, el
  formulario en una tarjeta), sin el menú de la app. Estos ficheros son tuyos.
- `/perfil` (`paginas/perfil.tsx`) muestra lo que responde `GET /me` y se abre desde el menú de
  usuario.
- El token de sesión está en `localStorage` (`auth/pasarela-local.ts`): al recargar se sigue con la
  sesión iniciada; «Cerrar sesión» la revoca en la API.
- Los tests de pantalla inician sesión con alguien que tiene todos los roles
  (`renderizarPantalla`); pasa `{ usuario }` para otro (`MadreUsuarioActual`),
  `{ auth: pasarelaAuthFalsa(…) }` o `{ cuentas: cuentasFalsas() }` para ver lo que pidió una
  pantalla.

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
    if agents.contains("## Auth (`[proyecto] auth`)") {
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
    let out = match agents.find("## Convenciones") {
        Some(at) => format!("{}{section}{}", &agents[..at], &agents[at..]),
        None => format!("{agents}\n{section}"),
    };
    fs::write(&path, out)?;
    Ok(())
}

/// Turns a fresh project into an auth one (`puerto nuevo --auth oidc|local`). The Saludo demo
/// stays open: it is listed `public`.
pub fn apply_auth_to_new_project(
    base: &Path,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    check_mode(mode)?;
    let mut config = crate::puerto_toml::read(base)?;
    config.project.auth = Some(mode.to_string());
    for entity in &mut config.entity {
        if entity.name == "Saludo" && entity.public.is_empty() {
            entity.public = entity.use_cases.clone();
        }
    }
    crate::puerto_toml::write(base, &config)?;
    let project = config.project.name.clone();

    let local = mode == "local";

    write_user_domain(base, config.project.db, local)?;
    write_user_infrastructure(base, config.project.db, local)?;
    write_user_rest(base, config.project.db, local)?;
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
        crate::generators::project::write_main(base)?;
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
                .contains("modo de auth desconocido")
        );
    }

    #[test]
    fn should_insert_after_the_exact_line_when_the_manifest_has_it() {
        let manifest = "[dependencies]\npuerto-nucleo = { workspace = true }\npoem = \"3\"\n";
        assert_eq!(
            insert_after_line(manifest, "puerto-nucleo = { workspace = true }", "x = 1"),
            Some(
                "[dependencies]\npuerto-nucleo = { workspace = true }\nx = 1\npoem = \"3\"\n"
                    .into()
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
            owner_checked: vec![],
        }
    }

    #[test]
    fn should_protect_every_use_case_but_the_public_ones_when_the_project_has_auth() {
        let access = access(true, &["listar_products"], &[]);
        assert!(access.is_protected("crear_product"));
        assert!(!access.is_protected("listar_products"));
    }

    #[test]
    fn should_protect_nothing_when_the_project_has_no_auth() {
        let access = access(false, &[], &[("crear_product", &["admin"])]);
        assert!(!access.is_protected("crear_product"));
        assert_eq!(access.roles("crear_product"), Vec::<String>::new());
    }

    #[test]
    fn should_collect_each_role_once_when_use_cases_share_it() {
        let access = access(
            true,
            &[],
            &[
                ("crear_product", &["admin"]),
                ("borrar_product", &["admin", "owner"]),
            ],
        );
        let use_cases = ["crear_product".to_string(), "borrar_product".to_string()];
        assert_eq!(access.roles("borrar_product"), vec!["admin", "owner"]);
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
