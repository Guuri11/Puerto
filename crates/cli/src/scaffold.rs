use std::{fs, path::Path};

// ── Name helpers ─────────────────────────────────────────────────────────────

/// Normalize any casing to PascalCase: `order_item` → `OrderItem`, `product` → `Product`.
pub fn to_pascal_case(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

/// Convert PascalCase to snake_case: `OrderItem` → `order_item`.
pub fn pascal_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

// ── Template substitution ─────────────────────────────────────────────────────

fn apply(template: &str, pascal: &str, snake: &str) -> String {
    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
}

// ── File writer ───────────────────────────────────────────────────────────────

fn write(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

// ── Lib.rs patching ───────────────────────────────────────────────────────────

/// Find `pub mod <block_name> { ... }` and insert `content` just before the closing `}`.
fn insert_before_block_end(
    source: &str,
    block_name: &str,
    content: &str,
) -> Result<String, String> {
    let marker = format!("pub mod {block_name} {{");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("block '{block_name}' not found"))?;

    let after_open = start + marker.len();
    let mut depth = 1usize;
    let mut close = None;

    for (i, ch) in source[after_open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(after_open + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close = close.ok_or_else(|| format!("unclosed block '{block_name}'"))?;
    Ok(format!(
        "{}{}{}",
        &source[..close],
        content,
        &source[close..]
    ))
}

/// Navigate nested `pub mod` blocks and insert `content` before the innermost closing `}`.
/// `path = &["domain", "product", "use_cases"]` finds `domain { ... product { ... use_cases { <here> } } }`.
fn patch_lib_block(source: &str, path: &[&str], content: &str) -> Result<String, String> {
    match path {
        [] => Err("empty path".to_string()),
        [name] => insert_before_block_end(source, name, content),
        [name, rest @ ..] => {
            let marker = format!("pub mod {name} {{");
            let start = source
                .find(&marker)
                .ok_or_else(|| format!("block '{name}' not found"))?;
            let after_open = start + marker.len();
            let mut depth = 1usize;
            let mut close = None;
            for (i, ch) in source[after_open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(after_open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.ok_or_else(|| format!("unclosed block '{name}'"))?;
            let inner = &source[after_open..close];
            let new_inner = patch_lib_block(inner, rest, content)?;
            Ok(format!(
                "{}{}{}",
                &source[..after_open],
                new_inner,
                &source[close..]
            ))
        }
    }
}

fn patch_business_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    let domain_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {{\n            pub mod create_{snake};\n        }}\n    }}\n"
    );
    let after_domain = insert_before_block_end(&src, "domain", &domain_mod)?;

    let app_mod = format!("\n    pub mod {snake} {{\n        pub mod create_{snake};\n    }}\n");
    let after_app = insert_before_block_end(&after_domain, "application", &app_mod)?;

    fs::write(&path, after_app)?;
    Ok(())
}

fn patch_business_lib_crud(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    let domain_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {{\n            pub mod create_{snake};\n            pub mod get_{snake};\n            pub mod list_{snake};\n            pub mod update_{snake};\n            pub mod delete_{snake};\n        }}\n    }}\n"
    );
    let after_domain = insert_before_block_end(&src, "domain", &domain_mod)?;

    let app_mod = format!(
        "\n    pub mod {snake} {{\n        pub mod create_{snake};\n        pub mod get_{snake};\n        pub mod list_{snake};\n        pub mod update_{snake};\n        pub mod delete_{snake};\n    }}\n"
    );
    let after_app = insert_before_block_end(&after_domain, "application", &app_mod)?;

    fs::write(&path, after_app)?;
    Ok(())
}

fn patch_infra_lib(base: &Path, snake: &str, db: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("infrastructure/src/lib.rs");
    let mut src = fs::read_to_string(&path)?;

    if !src.ends_with('\n') {
        src.push('\n');
    }
    if db {
        src.push_str(&format!(
            "pub mod {snake} {{\n    pub mod entity;\n    pub mod repository;\n}}\n"
        ));
        // Ensure `pub mod db;` is declared (idempotent)
        if !src.contains("pub mod db;") {
            src.push_str("pub mod db;\n");
        }
    } else {
        src.push_str(&format!(
            "pub mod {snake} {{\n    pub mod repository;\n}}\n"
        ));
    }

    fs::write(&path, src)?;
    Ok(())
}

fn patch_api_rs(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("presentation/src/api.rs");
    let mut src = fs::read_to_string(&path)?;

    if !src.ends_with('\n') {
        src.push('\n');
    }
    src.push_str(&format!("pub mod {snake};\n"));

    fs::write(&path, src)?;
    Ok(())
}

fn try_patch_libs(snake: &str, base: &Path, db: bool, crud: bool) -> bool {
    let business_ok = if crud {
        patch_business_lib_crud(base, snake).is_ok()
    } else {
        patch_business_lib(base, snake).is_ok()
    };
    business_ok && patch_infra_lib(base, snake, db).is_ok() && patch_api_rs(base, snake).is_ok()
}

// ── Bootstrap generation ──────────────────────────────────────────────────────

/// Generate the full content of `presentation/src/generated/bootstrap.rs`
/// from a list of entities read from harbor.toml.
pub fn generate_bootstrap_content(entities: &[crate::harbor_toml::Entity]) -> String {
    let mut out = String::new();

    out.push_str(
        "// AUTO-GENERATED by harbor. Edit harbor.toml, then run `harbor generate bootstrap`.\n",
    );
    out.push_str("use std::sync::Arc;\n\n");

    // Use case impl imports
    for entity in entities {
        let snake = pascal_to_snake(&entity.name);
        for uc in &entity.use_cases {
            let uc_pascal = to_pascal_case(uc);
            out.push_str(&format!(
                "use business::application::{snake}::{uc}::{uc_pascal}UseCaseImpl;\n"
            ));
        }
    }
    out.push('\n');

    // Repo + trait imports
    let has_db = entities.iter().any(|e| e.db);
    for entity in entities {
        let snake = pascal_to_snake(&entity.name);
        out.push_str(&format!(
            "use business::domain::{snake}::repository::{}RepositoryTrait;\n",
            entity.name
        ));
        if entity.db {
            out.push_str(&format!(
                "use infrastructure::{snake}::repository::Pg{}Repository;\n",
                entity.name
            ));
        } else {
            out.push_str(&format!(
                "use infrastructure::{snake}::repository::InMemory{}Repository;\n",
                entity.name
            ));
        }
    }
    out.push_str("use infrastructure::logger::TracingLogger;\n");
    out.push_str("use business::domain::logger::LoggerTrait;\n");
    out.push('\n');

    out.push_str("use poem::{EndpointExt, Route, middleware::Tracing};\n");
    out.push_str("use poem_openapi::OpenApiService;\n\n");

    // API imports
    for entity in entities {
        let snake = pascal_to_snake(&entity.name);
        out.push_str(&format!(
            "use crate::api::{snake}::routes::{}Api;\n",
            entity.name
        ));
    }
    out.push('\n');

    // build_app signature: async when any entity needs a pool
    if has_db {
        out.push_str("pub async fn build_app() -> impl poem::Endpoint {\n");
        // (always async — main.rs calls build_app().await in both cases)
        out.push_str("    dotenvy::dotenv().ok();\n");
        out.push_str(
            "    let database_url = std::env::var(\"DATABASE_URL\").expect(\"DATABASE_URL must be set\");\n",
        );
        out.push_str(
            "    let pool = infrastructure::db::create_postgres_pool(&database_url)\n        .await\n        .expect(\"Failed to connect to database\");\n",
        );
        out.push_str(
            "    infrastructure::db::run_migrations(&pool)\n        .await\n        .expect(\"Failed to run migrations\");\n\n",
        );
    } else {
        out.push_str("pub async fn build_app() -> impl poem::Endpoint {\n");
    }
    out.push_str("    let logger: Arc<dyn LoggerTrait> = Arc::new(TracingLogger);\n\n");

    // Wire each entity
    for entity in entities {
        let pascal = &entity.name;
        let snake = pascal_to_snake(pascal);
        let uc_count = entity.use_cases.len();
        let repo_type = if entity.db {
            format!("Pg{pascal}Repository::new(pool.clone(), Arc::clone(&logger))")
        } else {
            format!("InMemory{pascal}Repository::new(Arc::clone(&logger))")
        };

        for (i, uc) in entity.use_cases.iter().enumerate() {
            let uc_pascal = to_pascal_case(uc);
            // Single use case: inline the repo. Multiple: bind repo once then clone.
            let repo_expr = if uc_count == 1 {
                format!("Arc::new({repo_type}) as Arc<dyn {pascal}RepositoryTrait>")
            } else if i == 0 {
                out.push_str(&format!(
                    "    let {snake}_repo: Arc<dyn {pascal}RepositoryTrait> = Arc::new({repo_type});\n"
                ));
                format!("Arc::clone(&{snake}_repo)")
            } else if i < uc_count - 1 {
                format!("Arc::clone(&{snake}_repo)")
            } else {
                format!("{snake}_repo")
            };
            out.push_str(&format!(
                "    let {uc} = Arc::new({uc_pascal}UseCaseImpl {{ repository: {repo_expr}, logger: Arc::clone(&logger) }});\n"
            ));
        }

        let fields = entity.use_cases.join(", ");
        out.push_str(&format!(
            "    let {snake}_api = {pascal}Api {{ {fields}, logger: Arc::clone(&logger) }};\n\n"
        ));
    }

    // OpenApiService — single entity: direct; multiple: tuple
    let api_args = if entities.len() == 1 {
        format!("{}_api", pascal_to_snake(&entities[0].name))
    } else {
        let apis = entities
            .iter()
            .map(|e| format!("{}_api", pascal_to_snake(&e.name)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("({apis})")
    };

    out.push_str("    let api_service = OpenApiService::new(\n");
    out.push_str(&format!("        {api_args},\n"));
    out.push_str("        env!(\"CARGO_PKG_NAME\"),\n");
    out.push_str("        env!(\"CARGO_PKG_VERSION\"),\n");
    out.push_str("    )\n");
    out.push_str("    .server(\"http://localhost:8080/api\");\n");
    out.push_str("    let ui = api_service.swagger_ui();\n\n");
    out.push_str("    Route::new().nest(\"/api\", api_service).nest(\"/\", ui).with(Tracing)\n");
    out.push_str("}\n");

    out
}

/// Read harbor.toml and overwrite `presentation/src/generated/bootstrap.rs`.
pub fn regenerate_bootstrap(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::harbor_toml::read(base)?;
    let content = generate_bootstrap_content(&config.entity);
    let path = base.join("presentation/src/generated/bootstrap.rs");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn patch_business_lib_use_case(
    base: &Path,
    snake: &str,
    uc: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    // Idempotency: if already registered, skip
    let line = format!("pub mod {uc};");
    if src.contains(&line) {
        return Ok(());
    }

    let uc_content = format!("\n            pub mod {uc};\n");
    let after_uc = patch_lib_block(&src, &["domain", snake, "use_cases"], &uc_content)?;

    let app_content = format!("\n        pub mod {uc};\n");
    let final_src = patch_lib_block(&after_uc, &["application", snake], &app_content)?;

    fs::write(&path, final_src)?;
    Ok(())
}

fn apply_uc(template: &str, pascal: &str, snake: &str, uc_pascal: &str, uc: &str) -> String {
    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
        .replace("{uc_pascal}", uc_pascal)
        .replace("{uc}", uc)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(
    name: &str,
    base: &Path,
    db: bool,
    crud: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if crud {
        write_files_crud(&pascal, &snake, base, db)?;
    } else {
        write_files(&pascal, &snake, base, db)?;
    }
    try_patch_libs(&snake, base, db, crud);

    let use_cases = if crud {
        vec![
            format!("create_{snake}"),
            format!("get_{snake}"),
            format!("list_{snake}"),
            format!("update_{snake}"),
            format!("delete_{snake}"),
        ]
    } else {
        vec![format!("create_{snake}")]
    };

    let bootstrapped = crate::harbor_toml::add_entity(base, &pascal, use_cases, db)
        .and_then(|_| regenerate_bootstrap(base))
        .is_ok();

    let repo_name = if db {
        format!("Pg{pascal}Repository")
    } else {
        format!("InMemory{pascal}Repository")
    };
    let use_case_label = if crud {
        "create, get, list, update, delete"
    } else {
        "create"
    };
    println!("✓ business/        — model, errors, repository trait, use cases ({use_case_label})");
    println!("✓ infrastructure/  — {repo_name}");
    println!("✓ presentation/    — routes, dto, responses, error mapper");
    if bootstrapped {
        println!("✓ Done. Zero manual wiring.");
    } else {
        println!("✓ Done. Run `harbor generate bootstrap` to wire DI.");
    }

    Ok(())
}

/// Write the extra files that `harbor new --db` adds on top of the base template.
pub fn apply_db_to_new_project(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let project_name = base.file_name().and_then(|n| n.to_str()).unwrap_or("myapp");
    let db_name = project_name.replace('-', "_");

    // docker-compose.yml
    write(
        &base.join("docker-compose.yml"),
        &docker_compose_content(project_name, &db_name),
    )?;

    // .env (real dev defaults, gitignored)
    write(
        &base.join(".env"),
        &format!("DATABASE_URL=postgres://devuser:password@localhost:5432/{db_name}\n"),
    )?;

    // .env.example (committed, same format)
    write(
        &base.join(".env.example"),
        &format!("DATABASE_URL=postgres://devuser:password@localhost:5432/{db_name}\n"),
    )?;

    // .gitignore — add .env entry (create or append)
    add_env_to_gitignore(base)?;

    // .cargo/config.toml — SQLX_OFFLINE so CI compiles without a live DB
    write(
        &base.join(".cargo/config.toml"),
        "[env]\nSQLX_OFFLINE = \"true\"\n",
    )?;

    // infrastructure/migrations/ directory (empty, sqlx needs it)
    fs::create_dir_all(base.join("infrastructure/migrations"))?;

    // infrastructure/src/db.rs
    write(&base.join("infrastructure/src/db.rs"), DB_RS)?;

    // Patch infrastructure/Cargo.toml to add sqlx
    patch_infra_cargo_toml(base)?;

    // Patch Makefile: setup target + docker/sqlx targets
    patch_makefile_setup_for_db(base)?;
    add_db_makefile_targets(base)?;

    // Mark the project as db-enabled in harbor.toml
    let mut config = crate::harbor_toml::read(base)?;
    config.project.db = true;
    crate::harbor_toml::write(base, &config)?;

    Ok(())
}

fn docker_compose_content(project_name: &str, db_name: &str) -> String {
    format!(
        "services:\n  postgres:\n    image: postgres:16\n    container_name: {project_name}-postgres\n    ports:\n      - \"5432:5432\"\n    environment:\n      POSTGRES_USER: devuser\n      POSTGRES_PASSWORD: password\n      POSTGRES_DB: {db_name}\n    volumes:\n      - postgres-data:/var/lib/postgresql/data\n    restart: unless-stopped\n    healthcheck:\n      test: [\"CMD-SHELL\", \"pg_isready -U devuser -d {db_name}\"]\n      interval: 5s\n      timeout: 5s\n      retries: 5\n\nvolumes:\n  postgres-data:\n"
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
        write(&path, "/target\n.env\n/.sqlx\n")?;
    }
    Ok(())
}

fn add_db_makefile_targets(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(());
    }
    let src = fs::read_to_string(&path)?;
    if src.contains("docker/up") {
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
    let sqlx_line = "\tcargo install sqlx-cli --no-default-features --features postgres --locked\n";
    if src.contains("sqlx-cli") {
        return Ok(()); // idempotent
    }
    // Append sqlx-cli install after the nextest install line inside setup target
    let patched = src.replace(
        "\tcargo install cargo-nextest --locked\n",
        &format!("\tcargo install cargo-nextest --locked\n{sqlx_line}"),
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
    src.push_str(
        "\n[dependencies.sqlx]\nversion = \"0.8\"\nfeatures = [\"runtime-tokio-rustls\", \"postgres\", \"macros\", \"migrate\", \"uuid\", \"chrono\"]\n",
    );
    fs::write(&path, src)?;
    Ok(())
}

pub fn run_use_case(
    entity: &str,
    action: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(entity);
    let snake = pascal_to_snake(&pascal);
    let uc = action.to_string();
    let uc_pascal = to_pascal_case(&uc);

    // Errors if entity not in harbor.toml
    crate::harbor_toml::add_use_case(base, &pascal, &uc)?;

    write(
        &base.join(format!("business/src/domain/{snake}/use_cases/{uc}.rs")),
        &apply_uc(UC_TRAIT, &pascal, &snake, &uc_pascal, &uc),
    )?;
    write(
        &base.join(format!("business/src/application/{snake}/{uc}.rs")),
        &apply_uc(UC_IMPL, &pascal, &snake, &uc_pascal, &uc),
    )?;

    patch_business_lib_use_case(base, &snake, &uc)?;
    regenerate_bootstrap(base)?;

    println!("✓ Use case {uc_pascal} added to {pascal} (2 files).");
    println!("  harbor.toml updated + bootstrap.rs regenerated.");

    Ok(())
}

/// Add a new SQLx migration file.
///
/// `sqlx_bin` overrides the binary name/path — used in tests to pass a stub binary.
/// Pass `None` to use the default `"sqlx"` from `$PATH`.
pub fn run_migration(
    name: &str,
    base: &Path,
    sqlx_bin: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bin = sqlx_bin.unwrap_or("sqlx");

    // Pre-flight 1: check sqlx binary is reachable.
    let sqlx_check = std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if sqlx_check.is_err() {
        return Err(
            "sqlx CLI not found\ninstall it with: cargo install sqlx-cli --no-default-features --features postgres"
                .to_string()
                .into(),
        );
    }

    // Ensure migrations directory exists — create it if needed.
    let migrations_dir = base.join("infrastructure/migrations");
    fs::create_dir_all(&migrations_dir)?;

    // Normalise name: spaces → underscores, lowercase.
    let normalised = name.replace(' ', "_").to_lowercase();

    // Delegate to sqlx CLI.
    let status = std::process::Command::new(bin)
        .args([
            "migrate",
            "add",
            &normalised,
            "--source",
            migrations_dir
                .to_str()
                .unwrap_or("infrastructure/migrations"),
        ])
        .current_dir(base)
        .status()?;

    if !status.success() {
        return Err(format!("sqlx migrate add failed (exit {:?})", status.code()).into());
    }

    // Find the newly created file and prepend the Harbor header.
    if let Some(entry) = fs::read_dir(&migrations_dir)?
        .flatten()
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.contains(&normalised) && s.ends_with(".sql")
        })
        .max_by_key(|e| e.file_name())
    {
        let path = entry.path();
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let header = format!(
            "-- Harbor migration: {normalised}\n-- Run `make sqlx/prepare` after editing this file.\n\n"
        );
        fs::write(&path, format!("{header}{existing}"))?;
    }

    println!("✓ Migration '{normalised}' created in infrastructure/migrations/");
    println!("  Edit the file, then run: make sqlx/migrate");

    Ok(())
}

fn write_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Domain layer
    write(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &apply(MODEL, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &apply(ERRORS, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/domain/{snake}/repository.rs")),
        &apply(REPOSITORY, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &apply(USE_CASE_TRAIT, pascal, snake),
    )?;

    // Application layer
    write(
        &base.join(format!(
            "business/src/application/{snake}/create_{snake}.rs"
        )),
        &apply(USE_CASE_IMPL, pascal, snake),
    )?;

    // Infrastructure layer
    if db {
        write(
            &base.join(format!("infrastructure/src/{snake}/entity.rs")),
            &apply(INFRA_ENTITY, pascal, snake),
        )?;
        write(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &apply(INFRA_DB_REPOSITORY, pascal, snake),
        )?;
    } else {
        write(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &apply(INFRA_REPOSITORY, pascal, snake),
        )?;
    }

    // Presentation layer
    write(
        &base.join(format!("presentation/src/api/{snake}.rs")),
        "pub mod dto;\npub mod error_mapper;\npub mod responses;\npub mod routes;\n",
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/dto.rs")),
        &apply(DTO, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/responses.rs")),
        &apply(RESPONSES, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
        &apply(ERROR_MAPPER, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/routes.rs")),
        &apply(ROUTES, pascal, snake),
    )?;

    Ok(())
}

fn write_files_crud(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Domain layer
    write(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &apply(MODEL, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &apply(ERRORS, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/domain/{snake}/repository.rs")),
        &apply(CRUD_REPOSITORY, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &apply(USE_CASE_TRAIT, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/get_{snake}.rs"
        )),
        &apply(GET_USE_CASE_TRAIT, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/list_{snake}.rs"
        )),
        &apply(LIST_USE_CASE_TRAIT, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/update_{snake}.rs"
        )),
        &apply(UPDATE_USE_CASE_TRAIT, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/delete_{snake}.rs"
        )),
        &apply(DELETE_USE_CASE_TRAIT, pascal, snake),
    )?;

    // Application layer
    write(
        &base.join(format!(
            "business/src/application/{snake}/create_{snake}.rs"
        )),
        &apply(USE_CASE_IMPL, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/application/{snake}/get_{snake}.rs")),
        &apply(GET_USE_CASE_IMPL, pascal, snake),
    )?;
    write(
        &base.join(format!("business/src/application/{snake}/list_{snake}.rs")),
        &apply(LIST_USE_CASE_IMPL, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/application/{snake}/update_{snake}.rs"
        )),
        &apply(UPDATE_USE_CASE_IMPL, pascal, snake),
    )?;
    write(
        &base.join(format!(
            "business/src/application/{snake}/delete_{snake}.rs"
        )),
        &apply(DELETE_USE_CASE_IMPL, pascal, snake),
    )?;

    // Infrastructure layer
    if db {
        write(
            &base.join(format!("infrastructure/src/{snake}/entity.rs")),
            &apply(INFRA_ENTITY, pascal, snake),
        )?;
        write(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &apply(CRUD_INFRA_DB_REPOSITORY, pascal, snake),
        )?;
    } else {
        write(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &apply(CRUD_INFRA_REPOSITORY, pascal, snake),
        )?;
    }

    // Presentation layer
    write(
        &base.join(format!("presentation/src/api/{snake}.rs")),
        "pub mod dto;\npub mod error_mapper;\npub mod responses;\npub mod routes;\n",
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/dto.rs")),
        &apply(CRUD_DTO, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/responses.rs")),
        &apply(CRUD_RESPONSES, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
        &apply(ERROR_MAPPER, pascal, snake),
    )?;
    write(
        &base.join(format!("presentation/src/api/{snake}/routes.rs")),
        &apply(CRUD_ROUTES, pascal, snake),
    )?;

    Ok(())
}

// ── Makefile DB targets ───────────────────────────────────────────────────────

const DB_MAKEFILE_TARGETS: &str = "\ndocker/up: ## Start development containers\n\tdocker compose up -d\n\ndocker/down: ## Stop development containers\n\tdocker compose down\n\nsqlx/migrate: ## Run pending database migrations\n\tsqlx migrate run --source infrastructure/migrations\n\nsqlx/prepare: ## Regenerate SQLx offline cache (requires live DB)\n\tSQLX_OFFLINE=false cargo sqlx prepare --workspace\n\nsqlx/online: ## Switch SQLx to ONLINE mode\n\t@printf '[env]\\nSQLX_OFFLINE = \"false\"\\n' > .cargo/config.toml\n\nsqlx/offline: ## Switch SQLx to OFFLINE mode\n\t@printf '[env]\\nSQLX_OFFLINE = \"true\"\\n' > .cargo/config.toml\n";

// ── Templates ─────────────────────────────────────────────────────────────────

const MODEL: &str = r#"use chrono::NaiveDateTime;
use uuid::Uuid;

use super::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct {Pascal}Props {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct {Pascal} {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
    pub name: String,
}

impl {Pascal} {
    pub fn new(props: {Pascal}Props) -> Result<Self, {Pascal}Error> {
        if props.name.trim().is_empty() {
            return Err({Pascal}Error::ValidationError("name_empty".into()));
        }
        let now = chrono::Utc::now().naive_utc();
        Ok(Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            deleted: false,
            deleted_at: None,
            name: props.name,
        })
    }

    pub fn from_repository(data: {Pascal}) -> Self {
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_{snake}_when_name_is_valid() {
        let result = {Pascal}::new({Pascal}Props { name: "example".into() });
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[test]
    fn should_reject_{snake}_when_name_is_empty() {
        let result = {Pascal}::new({Pascal}Props { name: "".into() });
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }

    #[test]
    fn should_reject_{snake}_when_name_is_only_whitespace() {
        let result = {Pascal}::new({Pascal}Props { name: "   ".into() });
        assert!(result.is_err());
    }
}
"#;

const ERRORS: &str = r#"use thiserror::Error;

#[derive(Debug, Error)]
pub enum {Pascal}Error {
    #[error("{snake}.validation_error.{0}")]
    ValidationError(String),
    #[error("{snake}.not_found")]
    NotFound,
    #[error("{snake}.repository_error")]
    RepositoryError,
}
"#;

const REPOSITORY: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use super::{errors::{Pascal}Error, model::{Pascal}};

#[async_trait]
pub trait {Pascal}RepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use uuid::Uuid;

    use super::*;

    mock! {
        pub {Pascal}Repository {}

        #[async_trait]
        impl {Pascal}RepositoryTrait for {Pascal}Repository {
            async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
            async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
        }
    }
}
"#;

const USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Create{Pascal}Params {
    pub name: String,
}

#[async_trait]
pub trait Create{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{{Pascal}, {Pascal}Props},
    repository::{Pascal}RepositoryTrait,
    use_cases::create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Create{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Create{Pascal}UseCaseTrait for Create{Pascal}UseCaseImpl {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Creating {snake}: {}", params.name));
        let entity = {Pascal}::new({Pascal}Props { name: params.name }).map_err(|e| {
            self.logger.warn(&e.to_string());
            e
        })?;
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        self.logger.info(&format!("{Pascal} created: {}", entity.name));
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{Pascal}Repository;
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_create_{snake}_when_name_is_valid() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Create{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Create{Pascal}Params { name: "example".into() })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[tokio::test]
    async fn should_return_error_when_name_is_empty() {
        // Arrange
        let mock_repo = Mock{Pascal}Repository::new();
        let use_case = Create{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Create{Pascal}Params { name: "".into() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }
}
"#;

const INFRA_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

pub struct InMemory{Pascal}Repository {
    logger: Arc<dyn LoggerTrait>,
}

impl InMemory{Pascal}Repository {
    pub fn new(logger: Arc<dyn LoggerTrait>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for InMemory{Pascal}Repository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        Ok(None)
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        Ok(())
    }
}
"#;

const DTO: &str = r#"use business::domain::{snake}::model::{Pascal};
use poem_openapi::Object;
use uuid::Uuid;

#[derive(Debug, Object)]
pub struct {Pascal}Dto {
    pub id: Uuid,
    pub name: String,
}

impl {Pascal}Dto {
    pub fn from_domain(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
        }
    }
}

#[derive(Debug, Object)]
pub struct Create{Pascal}Request {
    pub name: String,
}
"#;

const RESPONSES: &str = r#"use crate::api::{error::ErrorResponse, {snake}::dto::{Pascal}Dto};
use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};

#[derive(ApiResponse)]
pub enum Create{Pascal}Response {
    #[oai(status = 201)]
    Created(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Create{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            _ => Self::InternalError(error),
        }
    }
}
"#;

const ERROR_MAPPER: &str = r#"use business::domain::{snake}::errors::{Pascal}Error;
use poem::http::StatusCode;
use poem_openapi::payload::Json;

use crate::api::error::{ErrorResponse, IntoErrorResponse};

impl IntoErrorResponse for {Pascal}Error {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>) {
        let (status, message) = match &self {
            {Pascal}Error::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            {Pascal}Error::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            {Pascal}Error::RepositoryError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (
            status,
            Json(ErrorResponse {
                name: "{snake}_error".into(),
                message,
            }),
        )
    }
}
"#;

const ROUTES: &str = r#"use std::sync::Arc;

use business::{
    application::{snake}::create_{snake}::Create{Pascal}UseCaseImpl,
    domain::{
        {snake}::use_cases::create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
        logger::LoggerTrait,
    },
};
use poem_openapi::{OpenApi, payload::Json};

use crate::api::error::IntoErrorResponse;
use crate::api::{snake}::dto::{Create{Pascal}Request, {Pascal}Dto};
use crate::api::{snake}::responses::Create{Pascal}Response;

pub struct {Pascal}Api {
    pub create_{snake}: Arc<Create{Pascal}UseCaseImpl>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[OpenApi]
impl {Pascal}Api {
    /// Create a new {Pascal}
    #[oai(path = "/{snake}s", method = "post")]
    async fn create(&self, body: Json<Create{Pascal}Request>) -> Create{Pascal}Response {
        match self
            .create_{snake}
            .execute(Create{Pascal}Params { name: body.name.clone() })
            .await
        {
            Ok(entity) => Create{Pascal}Response::Created(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("create_{snake} error: {}", error.0.message));
                Create{Pascal}Response::from_status(status, error)
            }
        }
    }
}
"#;

// ── SQLx templates ────────────────────────────────────────────────────────────

const INFRA_ENTITY: &str = r#"use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use business::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {Pascal}Db {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted: bool,
    pub deleted_at: Option<NaiveDateTime>,
    pub name: String,
}

impl TryFrom<{Pascal}Db> for {Pascal} {
    type Error = {Pascal}Error;

    fn try_from(row: {Pascal}Db) -> Result<Self, Self::Error> {
        Ok(Self::from_repository({Pascal} {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted: row.deleted,
            deleted_at: row.deleted_at,
            name: row.name,
        }))
    }
}

impl From<&{Pascal}> for {Pascal}Db {
    fn from(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            deleted: entity.deleted,
            deleted_at: entity.deleted_at,
            name: entity.name.clone(),
        }
    }
}
"#;

const INFRA_DB_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

use super::entity::{Pascal}Db;

pub struct Pg{Pascal}Repository {
    pub pool: PgPool,
    logger: Arc<dyn LoggerTrait>,
}

impl Pg{Pascal}Repository {
    pub fn new(pool: PgPool, logger: Arc<dyn LoggerTrait>) -> Self {
        Self { pool, logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for Pg{Pascal}Repository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        let row = sqlx::query_as!(
            {Pascal}Db,
            "SELECT id, created_at, updated_at, deleted, deleted_at, name
             FROM {snake}s WHERE id = $1 AND deleted = false",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_by_id error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        let db = {Pascal}Db::from(entity);
        sqlx::query!(
            "INSERT INTO {snake}s (id, created_at, updated_at, deleted, deleted_at, name)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE
             SET updated_at = $3, deleted = $4, deleted_at = $5, name = $6",
            db.id,
            db.created_at,
            db.updated_at,
            db.deleted,
            db.deleted_at,
            db.name
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("save error: {e}"));
            {Pascal}Error::RepositoryError
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use business::domain::{snake}::model::{Pascal}Props;
    use std::sync::Arc;

    struct TestLogger;
    impl business::domain::logger::LoggerTrait for TestLogger {
        fn info(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn error(&self, _: &str) {}
        fn debug(&self, _: &str) {}
    }

    fn test_repo(pool: PgPool) -> Pg{Pascal}Repository {
        Pg{Pascal}Repository::new(pool, Arc::new(TestLogger))
    }

    async fn seed(pool: &PgPool, name: &str) -> {Pascal} {
        let entity = {Pascal}::new({Pascal}Props { name: name.to_string() }).unwrap();
        test_repo(pool.clone()).save(&entity).await.unwrap();
        entity
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_persist_and_retrieve_by_id(pool: PgPool) {
        // Arrange
        let entity = seed(&pool, "example").await;

        // Act
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();

        // Assert
        assert_eq!(found.id, entity.id);
        assert_eq!(found.name, entity.name);
        assert!(!found.deleted);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_return_none_for_nonexistent_id(pool: PgPool) {
        // Act
        let result = test_repo(pool).find_by_id(Uuid::new_v4()).await.unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_update_entity_on_save_conflict(pool: PgPool) {
        // Arrange
        let mut entity = seed(&pool, "original").await;
        entity.name = "updated".to_string();
        entity.updated_at = chrono::Utc::now().naive_utc();

        // Act
        test_repo(pool.clone()).save(&entity).await.unwrap();

        // Assert
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();
        assert_eq!(found.name, "updated");
    }
}
"#;

const DB_RS: &str = r#"use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database.connection_error")]
    ConnectionError,
    #[error("database.migration_error")]
    MigrationError,
}

pub async fn create_postgres_pool(database_url: &str) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(database_url)
        .await
        .map_err(|_| DatabaseError::ConnectionError)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|_| DatabaseError::MigrationError)
}
"#;

// ── CRUD templates ────────────────────────────────────────────────────────────

const CRUD_REPOSITORY: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use super::{errors::{Pascal}Error, model::{Pascal}};

#[async_trait]
pub trait {Pascal}RepositoryTrait: Send + Sync {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use uuid::Uuid;

    use super::*;

    mock! {
        pub {Pascal}Repository {}

        #[async_trait]
        impl {Pascal}RepositoryTrait for {Pascal}Repository {
            async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
            async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
            async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
        }
    }
}
"#;

const CRUD_INFRA_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

pub struct InMemory{Pascal}Repository {
    logger: Arc<dyn LoggerTrait>,
}

impl InMemory{Pascal}Repository {
    pub fn new(logger: Arc<dyn LoggerTrait>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for InMemory{Pascal}Repository {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.debug("find_all");
        Ok(vec![])
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        Ok(None)
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        Ok(())
    }
}
"#;

const CRUD_INFRA_DB_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

use super::entity::{Pascal}Db;

pub struct Pg{Pascal}Repository {
    pub pool: PgPool,
    logger: Arc<dyn LoggerTrait>,
}

impl Pg{Pascal}Repository {
    pub fn new(pool: PgPool, logger: Arc<dyn LoggerTrait>) -> Self {
        Self { pool, logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for Pg{Pascal}Repository {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.debug("find_all");
        let rows = sqlx::query_as!(
            {Pascal}Db,
            "SELECT id, created_at, updated_at, deleted, deleted_at, name
             FROM {snake}s WHERE deleted = false ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_all error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        let row = sqlx::query_as!(
            {Pascal}Db,
            "SELECT id, created_at, updated_at, deleted, deleted_at, name
             FROM {snake}s WHERE id = $1 AND deleted = false",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_by_id error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        let db = {Pascal}Db::from(entity);
        sqlx::query!(
            "INSERT INTO {snake}s (id, created_at, updated_at, deleted, deleted_at, name)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE
             SET updated_at = $3, deleted = $4, deleted_at = $5, name = $6",
            db.id,
            db.created_at,
            db.updated_at,
            db.deleted,
            db.deleted_at,
            db.name
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("save error: {e}"));
            {Pascal}Error::RepositoryError
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use business::domain::{snake}::model::{Pascal}Props;
    use std::sync::Arc;

    struct TestLogger;
    impl business::domain::logger::LoggerTrait for TestLogger {
        fn info(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn error(&self, _: &str) {}
        fn debug(&self, _: &str) {}
    }

    fn test_repo(pool: PgPool) -> Pg{Pascal}Repository {
        Pg{Pascal}Repository::new(pool, Arc::new(TestLogger))
    }

    async fn seed(pool: &PgPool, name: &str) -> {Pascal} {
        let entity = {Pascal}::new({Pascal}Props { name: name.to_string() }).unwrap();
        test_repo(pool.clone()).save(&entity).await.unwrap();
        entity
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_persist_and_retrieve_by_id(pool: PgPool) {
        // Arrange
        let entity = seed(&pool, "example").await;

        // Act
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();

        // Assert
        assert_eq!(found.id, entity.id);
        assert_eq!(found.name, entity.name);
        assert!(!found.deleted);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_return_none_for_nonexistent_id(pool: PgPool) {
        // Act
        let result = test_repo(pool).find_by_id(Uuid::new_v4()).await.unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_update_entity_on_save_conflict(pool: PgPool) {
        // Arrange
        let mut entity = seed(&pool, "original").await;
        entity.name = "updated".to_string();
        entity.updated_at = chrono::Utc::now().naive_utc();

        // Act
        test_repo(pool.clone()).save(&entity).await.unwrap();

        // Assert
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();
        assert_eq!(found.name, "updated");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn should_list_all_{snake}s_excluding_deleted(pool: PgPool) {
        // Arrange
        seed(&pool, "first").await;
        seed(&pool, "second").await;

        // Act
        let results = test_repo(pool).find_all().await.unwrap();

        // Assert
        assert_eq!(results.len(), 2);
    }
}
"#;

const GET_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Get{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Get{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Get{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const LIST_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug)]
pub struct List{Pascal}Params;

#[async_trait]
pub trait List{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: List{Pascal}Params) -> Result<Vec<{Pascal}>, {Pascal}Error>;
}
"#;

const UPDATE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Update{Pascal}Params {
    pub id: Uuid,
    pub name: String,
}

#[async_trait]
pub trait Update{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Update{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const DELETE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct Delete{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Delete{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Delete{Pascal}Params) -> Result<(), {Pascal}Error>;
}
"#;

const GET_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::get_{snake}::{Get{Pascal}Params, Get{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Get{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Get{Pascal}UseCaseTrait for Get{Pascal}UseCaseImpl {
    async fn execute(&self, params: Get{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Getting {snake}: {}", params.id));
        let result = self.repository.find_by_id(params.id).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        result.ok_or_else(|| {
            let err = {Pascal}Error::NotFound;
            self.logger.warn(&err.to_string());
            err
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{Pascal}Props,
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_return_{snake}_when_id_exists() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "example".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        let use_case = Get{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(Get{Pascal}Params { id: entity_id }).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[tokio::test]
    async fn should_return_not_found_when_id_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Get{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Get{Pascal}Params { id: Uuid::new_v4() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }
}
"#;

const LIST_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::list_{snake}::{List{Pascal}Params, List{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct List{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl List{Pascal}UseCaseTrait for List{Pascal}UseCaseImpl {
    async fn execute(&self, _params: List{Pascal}Params) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.info("Listing {snake}s");
        self.repository.find_all().await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{Pascal}Props,
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_return_all_{snake}s() {
        // Arrange
        let entities = vec![
            {Pascal}::new({Pascal}Props { name: "first".into() }).unwrap(),
            {Pascal}::new({Pascal}Props { name: "second".into() }).unwrap(),
        ];
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_all()
            .returning(move || Ok(entities.clone()));
        let use_case = List{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(List{Pascal}Params).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn should_return_empty_list_when_no_{snake}s_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_all().returning(|| Ok(vec![]));
        let use_case = List{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(List{Pascal}Params).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
"#;

const UPDATE_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::update_{snake}::{Update{Pascal}Params, Update{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Update{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Update{Pascal}UseCaseTrait for Update{Pascal}UseCaseImpl {
    async fn execute(&self, params: Update{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Updating {snake}: {}", params.id));
        let mut entity = self
            .repository
            .find_by_id(params.id)
            .await
            .map_err(|e| {
                self.logger.error(&e.to_string());
                e
            })?
            .ok_or_else(|| {
                let err = {Pascal}Error::NotFound;
                self.logger.warn(&err.to_string());
                err
            })?;
        if params.name.trim().is_empty() {
            let err = {Pascal}Error::ValidationError("name_empty".into());
            self.logger.warn(&err.to_string());
            return Err(err);
        }
        entity.name = params.name;
        entity.updated_at = chrono::Utc::now().naive_utc();
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{Pascal}Props,
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_update_{snake}_when_params_are_valid() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "original".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: entity_id,
                name: "updated".into(),
            })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "updated");
    }

    #[tokio::test]
    async fn should_return_not_found_when_{snake}_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: Uuid::new_v4(),
                name: "new".into(),
            })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }

    #[tokio::test]
    async fn should_return_error_when_name_is_empty() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "original".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: entity_id,
                name: "".into(),
            })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }
}
"#;

const DELETE_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    repository::{Pascal}RepositoryTrait,
    use_cases::delete_{snake}::{Delete{Pascal}Params, Delete{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Delete{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Delete{Pascal}UseCaseTrait for Delete{Pascal}UseCaseImpl {
    async fn execute(&self, params: Delete{Pascal}Params) -> Result<(), {Pascal}Error> {
        self.logger.info(&format!("Deleting {snake}: {}", params.id));
        let mut entity = self
            .repository
            .find_by_id(params.id)
            .await
            .map_err(|e| {
                self.logger.error(&e.to_string());
                e
            })?
            .ok_or_else(|| {
                let err = {Pascal}Error::NotFound;
                self.logger.warn(&err.to_string());
                err
            })?;
        let now = chrono::Utc::now().naive_utc();
        entity.deleted = true;
        entity.deleted_at = Some(now);
        entity.updated_at = now;
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{Pascal}Props,
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_soft_delete_{snake}_when_id_exists() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "example".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Delete{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Delete{Pascal}Params { id: entity_id })
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_return_not_found_when_{snake}_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Delete{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Delete{Pascal}Params { id: Uuid::new_v4() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }
}
"#;

const CRUD_DTO: &str = r#"use business::domain::{snake}::model::{Pascal};
use poem_openapi::Object;
use uuid::Uuid;

#[derive(Debug, Object)]
pub struct {Pascal}Dto {
    pub id: Uuid,
    pub name: String,
}

impl {Pascal}Dto {
    pub fn from_domain(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
        }
    }
}

#[derive(Debug, Object)]
pub struct Create{Pascal}Request {
    pub name: String,
}

#[derive(Debug, Object)]
pub struct Update{Pascal}Request {
    pub name: String,
}
"#;

const CRUD_RESPONSES: &str = r#"use crate::api::{error::ErrorResponse, {snake}::dto::{Pascal}Dto};
use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};

#[derive(ApiResponse)]
pub enum Create{Pascal}Response {
    #[oai(status = 201)]
    Created(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Create{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum Get{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<{Pascal}Dto>),
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Get{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum List{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<Vec<{Pascal}Dto>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl List{Pascal}Response {
    pub fn from_status(_status: StatusCode, error: Json<ErrorResponse>) -> Self {
        Self::InternalError(error)
    }
}

#[derive(ApiResponse)]
pub enum Update{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Update{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum Delete{Pascal}Response {
    #[oai(status = 204)]
    NoContent,
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Delete{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}
"#;

const CRUD_ROUTES: &str = r#"use std::sync::Arc;

use business::{
    application::{snake}::{
        create_{snake}::Create{Pascal}UseCaseImpl,
        delete_{snake}::Delete{Pascal}UseCaseImpl,
        get_{snake}::Get{Pascal}UseCaseImpl,
        list_{snake}::List{Pascal}UseCaseImpl,
        update_{snake}::Update{Pascal}UseCaseImpl,
    },
    domain::{
        {snake}::use_cases::{
            create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
            delete_{snake}::{Delete{Pascal}Params, Delete{Pascal}UseCaseTrait},
            get_{snake}::{Get{Pascal}Params, Get{Pascal}UseCaseTrait},
            list_{snake}::{List{Pascal}Params, List{Pascal}UseCaseTrait},
            update_{snake}::{Update{Pascal}Params, Update{Pascal}UseCaseTrait},
        },
        logger::LoggerTrait,
    },
};
use poem_openapi::{OpenApi, param::Path, payload::Json};
use uuid::Uuid;

use crate::api::error::IntoErrorResponse;
use crate::api::{snake}::dto::{Create{Pascal}Request, Update{Pascal}Request, {Pascal}Dto};
use crate::api::{snake}::responses::{
    Create{Pascal}Response, Delete{Pascal}Response, Get{Pascal}Response, List{Pascal}Response,
    Update{Pascal}Response,
};

pub struct {Pascal}Api {
    pub create_{snake}: Arc<Create{Pascal}UseCaseImpl>,
    pub get_{snake}: Arc<Get{Pascal}UseCaseImpl>,
    pub list_{snake}: Arc<List{Pascal}UseCaseImpl>,
    pub update_{snake}: Arc<Update{Pascal}UseCaseImpl>,
    pub delete_{snake}: Arc<Delete{Pascal}UseCaseImpl>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[OpenApi]
impl {Pascal}Api {
    /// Create a new {Pascal}
    #[oai(path = "/{snake}s", method = "post")]
    async fn create(&self, body: Json<Create{Pascal}Request>) -> Create{Pascal}Response {
        match self
            .create_{snake}
            .execute(Create{Pascal}Params { name: body.name.clone() })
            .await
        {
            Ok(entity) => Create{Pascal}Response::Created(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("create_{snake} error: {}", error.0.message));
                Create{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Get a {Pascal} by ID
    #[oai(path = "/{snake}s/:id", method = "get")]
    async fn get_by_id(&self, id: Path<Uuid>) -> Get{Pascal}Response {
        match self
            .get_{snake}
            .execute(Get{Pascal}Params { id: id.0 })
            .await
        {
            Ok(entity) => Get{Pascal}Response::Ok(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("get_{snake} error: {}", error.0.message));
                Get{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// List all {Pascal}s
    #[oai(path = "/{snake}s", method = "get")]
    async fn list(&self) -> List{Pascal}Response {
        match self.list_{snake}.execute(List{Pascal}Params).await {
            Ok(entities) => {
                List{Pascal}Response::Ok(Json(entities.iter().map({Pascal}Dto::from_domain).collect()))
            }
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.error(&format!("list_{snake} error: {}", error.0.message));
                List{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Update a {Pascal}
    #[oai(path = "/{snake}s/:id", method = "put")]
    async fn update(&self, id: Path<Uuid>, body: Json<Update{Pascal}Request>) -> Update{Pascal}Response {
        match self
            .update_{snake}
            .execute(Update{Pascal}Params {
                id: id.0,
                name: body.name.clone(),
            })
            .await
        {
            Ok(entity) => Update{Pascal}Response::Ok(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("update_{snake} error: {}", error.0.message));
                Update{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Delete a {Pascal}
    #[oai(path = "/{snake}s/:id", method = "delete")]
    async fn delete(&self, id: Path<Uuid>) -> Delete{Pascal}Response {
        match self
            .delete_{snake}
            .execute(Delete{Pascal}Params { id: id.0 })
            .await
        {
            Ok(()) => Delete{Pascal}Response::NoContent,
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("delete_{snake} error: {}", error.0.message));
                Delete{Pascal}Response::from_status(status, error)
            }
        }
    }
}
"#;

// ── Use-case generator templates ──────────────────────────────────────────────

const UC_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct {uc_pascal}Params {
    pub name: String,
}

#[async_trait]
pub trait {uc_pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const UC_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::{uc}::{{uc_pascal}Params, {uc_pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct {uc_pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl {uc_pascal}UseCaseTrait for {uc_pascal}UseCaseImpl {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Executing {uc}: {:?}", params));
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{Pascal}Repository;
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_{uc}_when_valid() {
        // Arrange
        let mock_repo = Mock{Pascal}Repository::new();
        let use_case = {uc_pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        // TODO: implement test body
        let _ = &use_case;
    }
}
"#;
