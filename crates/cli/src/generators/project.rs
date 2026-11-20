use crate::generators::layout::{self, Layer};
use std::{fs, path::Path};

use crate::generators::naming::write_file;
use crate::generators::render::render;

const DB_MAKEFILE_TARGETS: &str = r#"
.PHONY: db/arrancar db/parar docker-compose/up docker-compose/down db/reiniciar \
        sqlx/online sqlx/offline sqlx/migrate sqlx/prepare sqlx/check \
        generar/migracion

DOCKER         := docker
DOCKER_COMPOSE := docker compose

db/arrancar: docker-compose/up ## Arranca los contenedores de la base de datos

db/parar: docker-compose/down ## Para los contenedores de la base de datos

docker-compose/up: ## Arranca todos los contenedores
	@echo "${CYAN}Arrancando los contenedores...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Para todos los contenedores
	@echo "${CYAN}Parando los contenedores...${NC}"
	$(DOCKER_COMPOSE) down

db/reiniciar: ## Vacía la base de datos y vuelve a aplicar las migraciones (DESTRUCTIVO; sin preguntar: CONFIRM=DESTRUIR)
	@if [ "$${CONFIRM:-}" != "DESTRUIR" ]; then \
		printf "${RED}Esto destruye TODOS los datos de la base de datos. Escribe DESTRUIR para confirmar: ${NC}"; \
		read CONFIRM; \
	fi; \
	if [ "$$CONFIRM" != "DESTRUIR" ]; then \
		echo "${GREEN}Cancelado${NC}"; \
		exit 1; \
	fi; \
	echo "${CYAN}Parando los contenedores y borrando los volúmenes...${NC}"; \
	$(DOCKER_COMPOSE) down -v; \
	echo "${CYAN}Volviendo a arrancar los contenedores...${NC}"; \
	$(DOCKER_COMPOSE) up -d --wait; \
	echo "${GREEN}Aplicando las migraciones...${NC}"; \
	sqlx migrate run --source migraciones

# El `run` y el `test/infraestructura` de la plantilla conservan su receta; aquí ganan la base
# de datos.
run test/infraestructura: docker-compose/up

sqlx/online: ## Pasa SQLx a modo ONLINE (comprueba contra la base de datos en marcha)
	@printf '[env]\nSQLX_OFFLINE = "false"\n' > .cargo/config.toml
	@echo "${GREEN}SQLx en modo ONLINE${NC}"

sqlx/offline: ## Pasa SQLx a modo OFFLINE (usa la caché guardada)
	@printf '[env]\nSQLX_OFFLINE = "true"\n' > .cargo/config.toml
	@echo "${GREEN}SQLx en modo OFFLINE${NC}"

sqlx/migrate: docker-compose/up ## Aplica las migraciones pendientes
	@echo "${GREEN}Aplicando las migraciones...${NC}"
	sqlx migrate run --source migraciones

sqlx/prepare: docker-compose/up ## Regenera la caché offline de SQLx (necesita la base de datos en marcha)
	@echo "${GREEN}Preparando la caché de SQLx...${NC}"
	SQLX_OFFLINE=false $(CARGO) sqlx prepare --workspace

sqlx/check: docker-compose/up ## Comprueba que la caché de SQLx coincide con las consultas actuales
	@echo "${GREEN}Comprobando la caché de SQLx...${NC}"
	SQLX_OFFLINE=false $(CARGO) sqlx prepare --workspace --check

generar/migracion: ## Crea una migración SQLx nueva — make generar/migracion NOMBRE=anadir_usuarios
	@if [ -z "$$NOMBRE" ]; then \
		echo "${RED}Error: falta el nombre — make generar/migracion NOMBRE=anadir_tabla_usuarios${NC}"; \
		exit 1; \
	fi
	@echo "${GREEN}Creando la migración '$$NOMBRE'...${NC}"
	puerto generar migracion $$NOMBRE
"#;

fn docker_compose_content(project_name: &str, db_name: &str) -> String {
    format!(
        "services:\n  postgres:\n    image: postgres:16\n    container_name: {project_name}-postgres\n    ports:\n      - \"${{DB_HOST_PORT:-5432}}:5432\"\n    environment:\n      POSTGRES_USER: devuser\n      POSTGRES_PASSWORD: password\n      POSTGRES_DB: {db_name}\n    volumes:\n      - postgres-data:/var/lib/postgresql/data\n    restart: unless-stopped\n    healthcheck:\n      test: [\"CMD-SHELL\", \"pg_isready -U devuser -d {db_name}\"]\n      interval: 5s\n      timeout: 5s\n      retries: 5\n\nvolumes:\n  postgres-data:\n"
    )
}

fn add_env_to_gitignore(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(".gitignore");
    if path.exists() {
        let mut src = fs::read_to_string(&path)?;
        if !src.contains(".env") {
            if !src.ends_with('\n') {
                src.push('\n');
            }
            src.push_str(".env\n");
            fs::write(&path, src)?;
        }
    } else {
        // `.sqlx/` is deliberately *not* ignored: `--db` projects compile with SQLX_OFFLINE=true,
        // so the query cache written by `make sqlx/prepare` must be committed for a clone (or CI)
        // to build without a live database.
        write_file(&path, "/target\n.env\n")?;
    }
    Ok(())
}

fn add_db_makefile_targets(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(());
    }
    let src = fs::read_to_string(&path)?;
    if src.contains("docker-compose/up") {
        return Ok(()); // idempotent
    }
    let mut patched = src;
    if !patched.ends_with('\n') {
        patched.push('\n');
    }
    patched.push_str(DB_MAKEFILE_TARGETS);
    fs::write(&path, patched)?;
    Ok(())
}

fn patch_makefile_setup_for_db(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(()); // no Makefile — nothing to patch
    }
    let src = fs::read_to_string(&path)?;
    let sqlx_line =
        "\t$(CARGO) install sqlx-cli --no-default-features --features postgres --locked\n";
    if src.contains("sqlx-cli") {
        return Ok(()); // idempotent
    }
    let patched = src
        .replace(
            "\t$(CARGO) install cargo-nextest --locked\n",
            &format!("\t$(CARGO) install cargo-nextest --locked\n{sqlx_line}"),
        )
        // The template's banner speaks of the in-memory repositories.
        .replace(
            "Arrancando el servidor de la API (en memoria: los datos se pierden al reiniciar)...",
            "Arrancando el servidor de la API...",
        );
    fs::write(&path, patched)?;
    Ok(())
}

/// `migrador/`: the one `MIGRADOR` of the whole database (ADR-0023 § 6).
const MIGRADOR_MANIFEST: &str = r#"[package]
name = "migrador"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
"#;

const MIGRADOR_LIB: &str = r#"//! Las migraciones de toda la base de datos (`migraciones/`), incrustadas al compilar: `api` las
//! aplica al arrancar y todos los tests de Postgres corren sobre ellas —
//! `#[sqlx::test(migrator = "migrador::MIGRADOR")]` (ADR-0023 § 6 de Puerto).

pub static MIGRADOR: sqlx::migrate::Migrator = sqlx::migrate!("../migraciones");
"#;

/// Without it, `migrate!` does not see a new migration until some `.rs` file changes.
const MIGRADOR_BUILD: &str = r#"fn main() {
    println!("cargo:rerun-if-changed=../migraciones");
}
"#;

/// The workspace side of `--db`: `migrador` as a member, `sqlx` and `puerto-sqlx` as dependencies,
/// and the `api` binary on both (the pool, the migrations at startup, the readiness ping).
fn add_migrador(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(layout::MIGRADOR_DIR).join("Cargo.toml"),
        MIGRADOR_MANIFEST,
    )?;
    write_file(
        &base.join(layout::MIGRADOR_DIR).join("src/lib.rs"),
        MIGRADOR_LIB,
    )?;
    write_file(
        &base.join(layout::MIGRADOR_DIR).join("build.rs"),
        MIGRADOR_BUILD,
    )?;
    let manifest = base.join(layout::WORKSPACE_MANIFEST);
    let src = fs::read_to_string(&manifest)?;
    if !src.contains("\"migrador\"") {
        fs::write(
            &manifest,
            src.replacen("\"nucleo\", ", "\"nucleo\", \"migrador\", ", 1),
        )?;
    }
    add_workspace_dependency(base, "puerto-sqlx")?;
    crate::generators::crates::add_internal_dependency(base, "migrador", layout::MIGRADOR_DIR)?;
    for name in ["migrador", "puerto-sqlx"] {
        crate::generators::bootstrap::add_crate_dependency(
            base,
            layout::API_MANIFEST,
            "dependencies",
            name,
            "{ workspace = true }",
        )?;
    }
    crate::generators::crates::sync_workspace_dependencies(base)?;
    Ok(())
}

/// Adds the service `name` (its YAML, indented as a service) to `docker-compose.yml`: after the
/// last service, before the top-level volumes. Creates the file when the project has none.
pub(crate) fn add_compose_service(
    base: &Path,
    name: &str,
    service: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("docker-compose.yml");
    if !path.exists() {
        write_file(&path, &format!("services:\n{service}"))?;
        return Ok(());
    }
    let src = fs::read_to_string(&path)?;
    if src.contains(&format!("\n  {name}:\n")) {
        return Ok(());
    }
    let out = match src.find("\nvolumes:\n") {
        Some(at) => format!("{}{service}{}", &src[..=at], &src[at..]),
        None => {
            let sep = if src.ends_with('\n') { "" } else { "\n" };
            format!("{src}{sep}{service}")
        }
    };
    fs::write(&path, out)?;
    Ok(())
}

/// Adds runtime crate `name` to the root `[workspace.dependencies]`, after the runtime crates the
/// template declares and on the same minor as this CLI.
pub(crate) fn add_workspace_dependency(
    base: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = format!(
        "\"{}\"",
        crate::runtime::requirement(env!("CARGO_PKG_VERSION"))
    );
    let path = base.join(layout::WORKSPACE_MANIFEST);
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // No runtime crate left (`--sin-demo` without web): under the block's comment again.
    let comment = "# en estos crates, en la misma versión minor que la CLI `puerto` que generó este proyecto.\n";
    if !src.lines().any(|l| l.starts_with("puerto-")) && src.contains(comment) {
        fs::write(
            &path,
            src.replacen(comment, &format!("{comment}{name} = {version}\n"), 1),
        )?;
        return Ok(());
    }
    insert_workspace_dependency(base, name, &version, Some("puerto-"))
}

/// Adds `name = value` to the root `[workspace.dependencies]` unless it is there: after the last
/// entry starting with `after` when there is one, else at the end of the table.
pub(crate) fn insert_workspace_dependency(
    base: &Path,
    name: &str,
    value: &str,
    after: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(layout::WORKSPACE_MANIFEST);
    let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if src.lines().any(|l| l.starts_with(&format!("{name} = "))) {
        return Ok(());
    }
    let header = "[workspace.dependencies]\n";
    let Some(at) = src.find(header) else {
        return Err(format!(
            "{}: no tiene tabla [workspace.dependencies]",
            path.display()
        )
        .into());
    };
    let line = format!("{name} = {value}");
    let body = at + header.len();
    let table_end = src[body..].find("\n[").map_or(src.len(), |i| body + i + 1);
    let table = &src[body..table_end];
    // The end of the last `after…` line, else of the table's last entry.
    let anchor = after
        .and_then(|prefix| {
            let mut offset = 0;
            let mut found = None;
            for l in table.split_inclusive('\n') {
                if l.starts_with(prefix) {
                    found = Some(offset + l.trim_end().len());
                }
                offset += l.len();
            }
            found
        })
        .unwrap_or_else(|| table.trim_end().len());
    let end = body + anchor;
    let out = if anchor > 0 {
        format!("{}\n{line}{}", &src[..end], &src[end..])
    } else {
        format!("{}{line}\n{}", &src[..body], &src[body..])
    };
    fs::write(&path, out)?;
    Ok(())
}

/// Write the base `.env` and `.env.example` for every new project (db or not).
/// `scripts/sembrar.sh` (27.21): the example data `make sembrar` sends to the running API, with the
/// helpers of the project's auth. Written once, executable; the user's from then on.
pub fn write_seed_script(
    base: &Path,
    auth: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("scripts/sembrar.sh");
    if path.exists() {
        return Ok(());
    }
    let mut ctx = tera::Context::new();
    ctx.insert("auth", &auth.unwrap_or_default());
    let name = crate::puerto_toml::read(base)
        .map(|c| c.project.name)
        .unwrap_or_default();
    ctx.insert("host", &name.replace('_', "-"));
    crate::generators::naming::write_file(
        &path,
        &crate::generators::render::render("scripts/sembrar.sh.tera", &ctx)?,
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

pub fn apply_base_env(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = "SERVICE_PORT=8080\nENV=development\nRUST_LOG=info\nCORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173\n";
    write_file(&base.join(".env"), content)?;
    write_file(&base.join(".env.example"), content)?;
    add_env_to_gitignore(base)?;
    Ok(())
}

/// Write the extra files that `puerto nuevo --db` adds on top of the base template.
pub fn apply_db_to_new_project(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let project_name = base.file_name().and_then(|n| n.to_str()).unwrap_or("myapp");
    let db_name = project_name.replace('-', "_");

    // docker-compose.yml
    write_file(
        &base.join("docker-compose.yml"),
        &docker_compose_content(project_name, &db_name),
    )?;

    // Append DB vars to .env and .env.example (apply_base_env already created them)
    let db_vars = format!(
        "\n# Base de datos\nDB_HOST_PORT=5432\nDATABASE_URL=postgres://devuser:password@localhost:5432/{db_name}\n"
    );
    for file in &[".env", ".env.example"] {
        let path = base.join(file);
        let mut content = if path.exists() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };
        if !content.contains("DATABASE_URL") {
            if !content.ends_with('\n') && !content.is_empty() {
                content.push('\n');
            }
            content.push_str(&db_vars);
            fs::write(&path, content)?;
        }
    }

    // .cargo/config.toml — SQLX_OFFLINE so CI compiles without a live DB
    write_file(
        &base.join(".cargo/config.toml"),
        "[env]\nSQLX_OFFLINE = \"true\"\n",
    )?;

    // migraciones/ (empty: sqlx needs it) and the crate that embeds it.
    fs::create_dir_all(base.join(layout::MIGRATIONS_DIR))?;
    add_migrador(base)?;

    // Patch Makefile: setup target + docker/sqlx targets
    patch_makefile_setup_for_db(base)?;
    add_db_makefile_targets(base)?;

    // Mark the project as db-enabled in puerto.toml
    let mut config = crate::puerto_toml::read(base)?;
    config.project.db = true;
    crate::puerto_toml::write(base, &config)?;

    Ok(())
}

/// The root `AGENTS.md` section an `events` project gets (ADR-0010: what it names is tested).
const AGENTS_EVENTS_SECTION: &str = r#"## Eventos de dominio (`[proyecto] eventos`)

Toda escritura de una entidad CRUD pasa por su unidad de trabajo, que guarda la entidad y registra
un evento en la tabla `outbox` en una sola transacción (ADR-0018 de Puerto). El repositorio solo
lee.

- **No edites nunca a mano `contextos/<c>/dominio/src/eventos.rs`**: se reescribe a partir de
  `puerto.toml` (el snapshot sigue los campos de la entidad). Añade un evento con
  `puerto generar evento <Entidad> <nombre>`.
- **La entidad emite sus eventos** (ADR-0026 de Puerto): `Producto::crear(props)` devuelve la
  entidad y su `Creado`; `actualizar` y `borrar`, su evento. Un evento propio lo emite un método
  tuyo de la entidad: `pub fn publicar(&mut self) -> EventoProducto { …;
  EventoProducto::Publicado(SnapshotProducto::from_entidad(self)) }`. Un evento que nadie guarda
  es un aviso (`#[must_use]`), y `make lint` lo rechaza.
- Un caso de uso de escritura guarda la entidad y su evento con `uow_factory.guardar(&entidad,
  evento)`: los dos en una transacción, o ninguno. Para escribir más de una vez, la unidad de
  trabajo a mano: `begin()` → `guardar` → `registrar` → `commit`; salir antes con `?`, sin llegar
  al `commit`, deshace todo.
- El snapshot es un contrato público: los consumidores lo decodifican. Renombrar un campo lo cambia.
- `presentacion/worker/` es el segundo proceso: hace de relay del outbox y ejecuta los consumidores
  (`make worker`). **No edites nunca a mano `presentacion/worker/src/generado/registro.rs`.**
  Añade un consumidor con `puerto generar consumidor <nombre> --tras <tipo_evento>`; su handler
  debe ser idempotente: la entrega es al menos una vez. Ver `presentacion/worker/AGENTS.md`.

"#;

/// Turns a fresh `--db` project into an `events` one (`puerto nuevo --db --eventos <transport>`):
/// `[proyecto] eventos`, the outbox migration, serde for the event snapshots and the `AGENTS.md`
/// section that says how writes work.
pub fn apply_events_to_new_project(
    base: &Path,
    transport: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::validation::validate_events_transport(transport)?;
    let mut config = crate::puerto_toml::read(base)?;
    if !config.project.db {
        return Err("--eventos necesita una base de datos: añade --db".into());
    }
    config.project.events = Some(transport.to_string());
    crate::puerto_toml::write(base, &config)?;

    let outbox = render("infrastructure/outbox.sql", &tera::Context::new())?;
    let outbox = if transport == crate::generators::worker::KAFKA_TRANSPORT {
        without_event_deliveries(&outbox)?
    } else {
        outbox
    };
    crate::generators::migration::run_migration("outbox", base, Some(&outbox))?;

    let agents_path = base.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    if !agents.contains(AGENTS_EVENTS_SECTION) {
        // Before the conventions: next to the rules about generated files.
        let out = match agents.find("## Convenciones") {
            Some(at) => format!("{}{AGENTS_EVENTS_SECTION}{}", &agents[..at], &agents[at..]),
            None => format!("{agents}\n{AGENTS_EVENTS_SECTION}"),
        };
        fs::write(&agents_path, out)?;
    }

    crate::generators::worker::apply_worker_to_new_project(base)?;
    Ok(())
}

/// The outbox migration of the `kafka` transport: Kafka holds the deliveries, so the `db`
/// transport's `entregas_eventos` table and its index are left out. The template stays the byte
/// copy of `puerto-sqlx`'s migration (tested), so this cuts it rather than keeping a second one.
fn without_event_deliveries(sql: &str) -> Result<String, Box<dyn std::error::Error>> {
    let start = sql.find("CREATE TABLE entregas_eventos");
    let index = "CREATE INDEX entregas_eventos_vencidas";
    let end = sql
        .find(index)
        .and_then(|at| sql[at..].find(";\n\n").map(|i| at + i + ";\n\n".len()));
    match (start, end) {
        (Some(start), Some(end)) if start < end => Ok(format!("{}{}", &sql[..start], &sql[end..])),
        _ => Err(
            "la migración del outbox no tiene el bloque `entregas_eventos` que hay que quitar"
                .into(),
        ),
    }
}

/// An already-absent file is what a removal wants; any other failure is a real error.
fn ignore_not_found(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Strip the Saludo demo entity from a freshly generated project (`puerto nuevo --sin-demo`):
/// its context's four crates and their declarations, its entity in puerto.toml, and the wiring
/// regenerated without it.
pub fn apply_no_demo(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    ignore_not_found(fs::remove_dir_all(base.join(layout::context_dir("saludo"))))?;
    crate::generators::crates::remove_contexts_member(base)?;
    for layer in Layer::ALL {
        let name = layout::crate_name("saludo", layer);
        crate::generators::crates::remove_internal_dependency(base, &name)?;
        crate::generators::crates::remove_dependency_line(base, layout::API_MANIFEST, &name)?;
    }

    let mut config = crate::puerto_toml::read(base)?;
    config.entity.clear();
    crate::puerto_toml::write(base, &config)?;

    crate::generators::bootstrap::regenerate_bootstrap(base)?;

    Ok(())
}

/// The heading of the root `AGENTS.md` section that describes `web/`.
const AGENTS_WEB_SECTION: &str = "## Frontend (`web/`)";

/// Strip the web frontend from a freshly generated project (`puerto nuevo --sin-web`): the `web/`
/// directory, `[proyecto] web`, the root `AGENTS.md` section that describes it and the spec export
/// test in `bootstrap.rs`.
pub fn apply_no_web(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    ignore_not_found(fs::remove_dir_all(base.join("web")))?;

    let agents_path = base.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    let agents = without_section(&agents, AGENTS_WEB_SECTION);
    fs::write(&agents_path, web_blocks(&agents, false))?;

    let mut config = crate::puerto_toml::read(base)?;
    config.project.web = false;
    crate::puerto_toml::write(base, &config)?;

    crate::generators::bootstrap::regenerate_bootstrap(base)?;

    Ok(())
}

/// The markers around the root `AGENTS.md` lines that only hold with `web/` — inside sections
/// that stay, so [`without_section`] cannot drop them.
const WEB_BLOCK_START: &str = "<!-- web -->";
const WEB_BLOCK_END: &str = "<!-- /web -->";

/// `markdown` with its web-only blocks kept (`web`) or dropped, and the marker lines gone.
fn web_blocks(markdown: &str, web: bool) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut inside = false;
    for line in markdown.split_inclusive('\n') {
        match line.trim_end() {
            WEB_BLOCK_START => inside = true,
            WEB_BLOCK_END => inside = false,
            _ if inside && !web => {}
            _ => out.push_str(line),
        }
    }
    out
}

/// Keeps the root `AGENTS.md`'s web-only blocks of a project with `web/`, without their markers.
pub fn keep_web_blocks(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let agents_path = base.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    fs::write(&agents_path, web_blocks(&agents, true))?;
    Ok(())
}

/// `puerto nuevo --portada` (ADR-0028): `[proyecto] portada` and `ruta_app`, the welcome portada
/// at `/` (`portada/bienvenida`) with its test, and `rutaApp` in `generado/rutas.tsx`.
pub fn apply_portada(base: &Path, app_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    crate::validation::validate_app_path(app_path)?;
    let mut config = crate::puerto_toml::read(base)?;
    config.project.portada = true;
    config.project.app_path = Some(app_path.to_string());
    crate::puerto_toml::write(base, &config)?;
    crate::generators::blocks::run_install(
        "portada/bienvenida",
        base,
        crate::generators::conflict::OverwritePolicy::Force,
    )?;
    write_file(
        &base
            .join(layout::WEB_DIR)
            .join("src/paginas/portada.test.tsx"),
        &render("web/portada.test.tsx.tera", &tera::Context::new())?,
    )?;
    crate::generators::web::regenerate_web(base, &config)?;
    write_main(base)
}

/// `web/src/main.tsx` of a project with auth or a portada: the pages outside the app's frame (the
/// portada, the access pages) and the app in `Marco`. Without either, the template's stays.
pub fn write_main(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let mut ctx = tera::Context::new();
    ctx.insert("auth", &config.project.has_auth());
    ctx.insert("local", &(config.project.auth.as_deref() == Some("local")));
    ctx.insert("portada", &config.project.portada);
    write_file(
        &base.join(layout::WEB_DIR).join("src/main.tsx"),
        &render("web/main.tsx.tera", &ctx)?,
    )?;
    Ok(())
}

/// `markdown` without the `## ` section that starts at `heading`, up to the next `## ` heading.
fn without_section(markdown: &str, heading: &str) -> String {
    let Some(start) = markdown.find(&format!("{heading}\n")) else {
        return markdown.to_string();
    };
    let body = start + heading.len();
    let end = markdown[body..]
        .find("\n## ")
        .map_or(markdown.len(), |offset| body + offset + 1);
    format!("{}{}", &markdown[..start], &markdown[end..])
}

#[cfg(test)]
mod tests {
    use super::without_section;

    #[test]
    fn should_drop_web_blocks_and_their_markers_when_there_is_no_web() {
        let markdown = "a\n<!-- web -->\nw\n<!-- /web -->\nb\n";
        assert_eq!(super::web_blocks(markdown, false), "a\nb\n");
    }

    #[test]
    fn should_keep_web_blocks_without_their_markers_when_there_is_web() {
        let markdown = "a\n<!-- web -->\nw\n<!-- /web -->\nb\n";
        assert_eq!(super::web_blocks(markdown, true), "a\nw\nb\n");
    }

    #[test]
    fn should_keep_the_next_section_when_removing_one_in_the_middle() {
        let markdown = "# T\n\n## A\n\na\n\n## Web\n\nw\n\n## B\n\nb\n";
        assert_eq!(
            without_section(markdown, "## Web"),
            "# T\n\n## A\n\na\n\n## B\n\nb\n"
        );
    }

    #[test]
    fn should_drop_up_to_the_end_when_the_section_is_the_last_one() {
        assert_eq!(
            without_section("## A\n\na\n\n## Web\n\nw\n", "## Web"),
            "## A\n\na\n\n"
        );
    }

    #[test]
    fn should_leave_the_markdown_alone_when_the_section_is_absent() {
        assert_eq!(without_section("## A\n\na\n", "## Web"), "## A\n\na\n");
    }
}
