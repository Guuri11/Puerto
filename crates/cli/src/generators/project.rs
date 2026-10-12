use std::{fs, path::Path};

use crate::generators::naming::write_file;
use crate::generators::render::render;

const DB_MAKEFILE_TARGETS: &str = r#"
.PHONY: db-up db-down docker-compose/up docker-compose/down reset-db \
        sqlx/online sqlx/offline sqlx/migrate sqlx/prepare sqlx/check \
        generate/migration

DOCKER         := docker
DOCKER_COMPOSE := docker compose

db-up: docker-compose/up ## Start database containers

db-down: docker-compose/down ## Stop database containers

docker-compose/up: ## Start all containers
	@echo "${CYAN}Starting containers...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Stop all containers
	@echo "${CYAN}Stopping containers...${NC}"
	$(DOCKER_COMPOSE) down

reset-db: ## Wipe database and re-run migrations (DESTRUCTIVE)
	@printf "${RED}This destroys ALL database data. Type DESTROY to confirm: ${NC}"; \
	read CONFIRM; \
	if [ "$$CONFIRM" != "DESTROY" ]; then \
		echo "${GREEN}Cancelled${NC}"; \
		exit 1; \
	fi; \
	echo "${CYAN}Stopping containers and removing volumes...${NC}"; \
	$(DOCKER_COMPOSE) down -v; \
	echo "${CYAN}Restarting containers...${NC}"; \
	$(DOCKER_COMPOSE) up -d --wait; \
	echo "${GREEN}Running migrations...${NC}"; \
	sqlx migrate run --source infrastructure/migrations

# The template's `run` and `test/infrastructure` keep their recipes; here they gain the database.
run test/infrastructure: docker-compose/up

sqlx/online: ## Switch SQLx to ONLINE mode (check against live DB)
	@printf '[env]\nSQLX_OFFLINE = "false"\n' > .cargo/config.toml
	@echo "${GREEN}SQLx ONLINE mode${NC}"

sqlx/offline: ## Switch SQLx to OFFLINE mode (use saved cache)
	@printf '[env]\nSQLX_OFFLINE = "true"\n' > .cargo/config.toml
	@echo "${GREEN}SQLx OFFLINE mode${NC}"

sqlx/migrate: docker-compose/up ## Run pending database migrations
	@echo "${GREEN}Running migrations...${NC}"
	sqlx migrate run --source infrastructure/migrations

sqlx/prepare: docker-compose/up ## Regenerate SQLx offline cache (requires live DB)
	@echo "${GREEN}Preparing SQLx cache...${NC}"
	SQLX_OFFLINE=false $(CARGO) sqlx prepare --workspace

sqlx/check: docker-compose/up ## Verify SQLx cache matches current queries
	@echo "${GREEN}Checking SQLx cache...${NC}"
	SQLX_OFFLINE=false $(CARGO) sqlx prepare --workspace --check

generate/migration: ## Create a new SQLx migration — make generate/migration NAME=add_users
	@if [ -z "$$NAME" ]; then \
		echo "${RED}Error: provide name — make generate/migration NAME=add_users_table${NC}"; \
		exit 1; \
	fi
	@echo "${GREEN}Creating migration '$$NAME'...${NC}"
	puerto generate migration $$NAME
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
            "Starting API server (in memory: data is lost on restart)...",
            "Starting API server...",
        );
    fs::write(&path, patched)?;
    Ok(())
}

fn patch_infra_cargo_toml(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("infrastructure/Cargo.toml");
    let mut src = fs::read_to_string(&path)?;
    if src.contains("sqlx") {
        return Ok(()); // idempotent
    }
    if !src.ends_with('\n') {
        src.push('\n');
    }
    // The adapter is the runtime's (ADR-0017); `sqlx` stays for the repositories' queries and the
    // project's `migrate!`.
    src = src.replace(
        "puerto-core = { workspace = true }\n",
        "puerto-core = { workspace = true }\npuerto-sqlx = { workspace = true }\n",
    );
    // Before `[dev-dependencies]`, so the runtime dependencies stay together.
    let sqlx = "[dependencies.sqlx]\nversion = \"0.8\"\nfeatures = [\"runtime-tokio-rustls\", \"postgres\", \"macros\", \"migrate\", \"uuid\", \"chrono\", \"json\"]\n\n";
    src = match src.find("[dev-dependencies]") {
        Some(at) => format!("{}{sqlx}{}", &src[..at], &src[at..]),
        None => format!("{src}\n{}", sqlx.trim_end_matches('\n')) + "\n",
    };
    fs::write(&path, src)?;
    add_workspace_dependency(base, "puerto-sqlx")
}

/// Adds runtime crate `name` to the root `[workspace.dependencies]`, next to the ones the template
/// declares and on the same minor as this CLI.
pub(crate) fn add_workspace_dependency(
    base: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Cargo.toml");
    let src = fs::read_to_string(&path)?;
    if src.contains(&format!("{name} = ")) {
        return Ok(());
    }
    let header = "[workspace.dependencies]\n";
    let Some(at) = src.find(header) else {
        return Err(format!("{}: no [workspace.dependencies] table", path.display()).into());
    };
    let line = format!(
        "{name} = \"{}\"",
        crate::runtime::requirement(env!("CARGO_PKG_VERSION"))
    );
    // After the table's last entry: before the next table (and the blank line ahead of it), or at
    // the end of the file.
    let body = at + header.len();
    let table_end = src[body..].find("\n[").map_or(src.len(), |i| body + i + 1);
    let end = body + src[body..table_end].trim_end().len();
    let out = if end > body {
        format!("{}\n{line}{}", &src[..end], &src[end..])
    } else {
        format!("{}{line}\n{}", &src[..body], &src[body..])
    };
    fs::write(&path, out)?;
    Ok(())
}

/// Write the base `.env` and `.env.example` for every new project (db or not).
pub fn apply_base_env(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = "SERVICE_PORT=8080\nENV=development\nRUST_LOG=info\nCORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173\n";
    write_file(&base.join(".env"), content)?;
    write_file(&base.join(".env.example"), content)?;
    add_env_to_gitignore(base)?;
    Ok(())
}

/// Write the extra files that `puerto new --db` adds on top of the base template.
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
        "\n# Database\nDB_HOST_PORT=5432\nDATABASE_URL=postgres://devuser:password@localhost:5432/{db_name}\n"
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

    // infrastructure/migrations/ directory (empty, sqlx needs it)
    fs::create_dir_all(base.join("infrastructure/migrations"))?;

    // infrastructure/src/db.rs
    write_file(
        &base.join("infrastructure/src/db.rs"),
        &render("infrastructure/db.tera", &tera::Context::new())?,
    )?;
    // …declared now, not by the first scaffold: the `--events` worker names `infrastructure::db`.
    let infra_lib = base.join("infrastructure/src/lib.rs");
    let mut lib = fs::read_to_string(&infra_lib)?;
    if !lib.contains("pub mod db;") {
        lib.push_str("pub mod db;\n");
        fs::write(&infra_lib, lib)?;
    }

    // Patch infrastructure/Cargo.toml to add sqlx
    patch_infra_cargo_toml(base)?;

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
const AGENTS_EVENTS_SECTION: &str = r#"## Domain events (`[project] events`)

Every write of a CRUD entity goes through its unit of work, which saves the entity and records an
event in the `outbox` table in one transaction (ADR-0018 of Puerto). The repository only reads.

- **Never hand-edit `business/src/domain/<entity>/events.rs`** — it is rewritten from
  `puerto.toml` (the snapshot follows the entity's fields). Add an event with
  `puerto generate event <Entity> <name>`, then record it from a use case:
  `uow.record(ProductEvent::Published(ProductSnapshot::from_entity(&entity)))`.
- A write use case: `uow_factory.begin()` → `save` → `record` → `commit`. Returning early with
  `?` before `commit` rolls everything back.
- The snapshot is a public contract: consumers decode it. Renaming a field changes it.
- `worker/` is the second process: it relays the outbox and runs the consumers (`make worker`).
  **Never hand-edit `worker/src/generated/registry.rs`.** Add a consumer with
  `puerto generate consumer <name> --on <event_type>`; its handler must be idempotent — delivery
  is at-least-once. See `worker/AGENTS.md`.

"#;

/// Turns a fresh `--db` project into an `events` one (`puerto new --db --events <transport>`):
/// `[project] events`, the outbox migration, serde for the event snapshots and the `AGENTS.md`
/// section that says how writes work.
pub fn apply_events_to_new_project(
    base: &Path,
    transport: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::validation::validate_events_transport(transport)?;
    let mut config = crate::puerto_toml::read(base)?;
    if !config.project.db {
        return Err("--events needs a database: add --db".into());
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

    // `events.rs` derives serde on the snapshots; the domain models stay serde-free.
    let manifest = base.join("business/Cargo.toml");
    let src = fs::read_to_string(&manifest)?;
    if !src.contains("serde_json") {
        let patched = src.replace(
            "uuid = { version = \"1\", features = [\"v4\"] }\nchrono = \"0.4\"\n",
            "uuid = { version = \"1\", features = [\"v4\", \"serde\"] }\n\
                 chrono = { version = \"0.4\", features = [\"serde\"] }\n\
                 serde = { version = \"1\", features = [\"derive\"] }\n\
                 serde_json = \"1\"\n",
        );
        if !patched.contains("serde_json") {
            return Err(format!(
                "{}: no uuid/chrono dependencies to extend",
                manifest.display()
            )
            .into());
        }
        fs::write(&manifest, patched)?;
    }

    let agents_path = base.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    if !agents.contains(AGENTS_EVENTS_SECTION) {
        // Before the conventions: next to the rules about generated files.
        let out = match agents.find("## Conventions") {
            Some(at) => format!("{}{AGENTS_EVENTS_SECTION}{}", &agents[..at], &agents[at..]),
            None => format!("{agents}\n{AGENTS_EVENTS_SECTION}"),
        };
        fs::write(&agents_path, out)?;
    }

    crate::generators::worker::apply_worker_to_new_project(base)?;
    Ok(())
}

/// The outbox migration of the `kafka` transport: Kafka holds the deliveries, so the `db`
/// transport's `event_deliveries` table and its index are left out. The template stays the byte
/// copy of `puerto-sqlx`'s migration (tested), so this cuts it rather than keeping a second one.
fn without_event_deliveries(sql: &str) -> Result<String, Box<dyn std::error::Error>> {
    let start = sql.find("CREATE TABLE event_deliveries");
    let index = "CREATE INDEX event_deliveries_due";
    let end = sql
        .find(index)
        .and_then(|at| sql[at..].find(";\n\n").map(|i| at + i + ";\n\n".len()));
    match (start, end) {
        (Some(start), Some(end)) if start < end => Ok(format!("{}{}", &sql[..start], &sql[end..])),
        _ => Err("the outbox migration has no `event_deliveries` block to leave out".into()),
    }
}

/// An already-absent file is what a removal wants; any other failure is a real error.
fn ignore_not_found(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Strip the Greeting demo entity from a freshly generated project (`puerto new --no-demo`).
/// Removes greeting files, rewrites the four affected source files, clears puerto.toml entities,
/// and regenerates bootstrap.rs with an empty API.
pub fn apply_no_demo(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Remove greeting directories and the sibling declaration file.
    for dir in [
        "business/src/domain/greeting",
        "business/src/application/greeting",
        "infrastructure/src/greeting",
        "presentation/src/api/greeting",
    ] {
        ignore_not_found(fs::remove_dir_all(base.join(dir)))?;
    }
    ignore_not_found(fs::remove_file(
        base.join("presentation/src/api/greeting.rs"),
    ))?;

    // Minimal business lib.rs: empty domain and application blocks for the patchers to fill.
    // `RepositoryError` and the page types come from `puerto-core`; `pub mod shared;` arrives
    // with the first shared value object.
    fs::write(
        base.join("business/src/lib.rs"),
        "pub mod domain {\n}\npub mod application {\n}\n",
    )?;

    // Minimal infrastructure lib.rs: no adapters yet. A `--db` project keeps its `db` module.
    let infra_lib = if crate::puerto_toml::read(base).is_ok_and(|c| c.project.db) {
        "pub mod db;\n"
    } else {
        ""
    };
    fs::write(base.join("infrastructure/src/lib.rs"), infra_lib)?;

    // Minimal presentation api.rs. `tags` must survive the demo strip: `regenerate_bootstrap`
    // always writes `api/tags.rs`, and every generated `routes.rs` does
    // `use crate::api::tags::ApiTags;` — dropping the declaration made the first scaffold into a
    // `--no-demo` project fail to compile (E0432).
    fs::write(
        base.join("presentation/src/api.rs"),
        "pub mod error;\npub mod tags;\n",
    )?;

    // Clear entities in puerto.toml.
    let mut config = crate::puerto_toml::read(base)?;
    config.entity.clear();
    crate::puerto_toml::write(base, &config)?;

    // Regenerate bootstrap.rs with no entities.
    crate::generators::bootstrap::regenerate_bootstrap(base)?;

    Ok(())
}

/// The heading of the root `AGENTS.md` section that describes `web/`.
const AGENTS_WEB_SECTION: &str = "## Frontend (`web/`)";

/// Strip the web frontend from a freshly generated project (`puerto new --no-web`): the `web/`
/// directory, `[project] web`, the root `AGENTS.md` section that describes it and the spec export
/// test in `bootstrap.rs`.
pub fn apply_no_web(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    ignore_not_found(fs::remove_dir_all(base.join("web")))?;

    let agents_path = base.join("AGENTS.md");
    let agents = fs::read_to_string(&agents_path)?;
    fs::write(&agents_path, without_section(&agents, AGENTS_WEB_SECTION))?;

    let mut config = crate::puerto_toml::read(base)?;
    config.project.web = false;
    crate::puerto_toml::write(base, &config)?;

    crate::generators::bootstrap::regenerate_bootstrap(base)?;

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
