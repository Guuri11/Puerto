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

fn try_patch_libs(snake: &str, base: &Path, db: bool) -> bool {
    patch_business_lib(base, snake).is_ok()
        && patch_infra_lib(base, snake, db).is_ok()
        && patch_api_rs(base, snake).is_ok()
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

    // Repo imports — InMemory for non-db, Pg for db
    let has_db = entities.iter().any(|e| e.db);
    for entity in entities {
        let snake = pascal_to_snake(&entity.name);
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
    if has_db {
        out.push_str("use sqlx::PgPool;\n");
    }
    out.push('\n');

    out.push_str("use poem::Route;\n");
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
        out.push_str("pub async fn build_app() -> Route {\n");
        out.push_str("    dotenvy::dotenv().ok();\n");
        out.push_str(
            "    let database_url = std::env::var(\"DATABASE_URL\").expect(\"DATABASE_URL must be set\");\n",
        );
        out.push_str(
            "    let pool = infrastructure::db::create_postgres_pool(&database_url).await;\n\n",
        );
    } else {
        out.push_str("pub fn build_app() -> Route {\n");
    }

    // Wire each entity
    for entity in entities {
        let pascal = &entity.name;
        let snake = pascal_to_snake(pascal);
        let uc_count = entity.use_cases.len();
        let repo_type = if entity.db {
            format!("Pg{pascal}Repository {{ pool: pool.clone() }}")
        } else {
            format!("InMemory{pascal}Repository")
        };

        for (i, uc) in entity.use_cases.iter().enumerate() {
            let uc_pascal = to_pascal_case(uc);
            // Single use case: inline the repo. Multiple: bind repo once then clone.
            let repo_expr = if uc_count == 1 {
                format!("Arc::new({repo_type})")
            } else if i == 0 {
                out.push_str(&format!("    let {snake}_repo = Arc::new({repo_type});\n"));
                format!("Arc::clone(&{snake}_repo)")
            } else if i < uc_count - 1 {
                format!("Arc::clone(&{snake}_repo)")
            } else {
                format!("{snake}_repo")
            };
            out.push_str(&format!(
                "    let {uc} = Arc::new({uc_pascal}UseCaseImpl {{ repository: {repo_expr} }});\n"
            ));
        }

        let fields = entity.use_cases.join(", ");
        out.push_str(&format!(
            "    let {snake}_api = {pascal}Api {{ {fields} }};\n\n"
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
    out.push_str("    Route::new().nest(\"/api\", api_service).nest(\"/\", ui)\n");
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

pub fn run(name: &str, base: &Path, db: bool) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    write_files(&pascal, &snake, base, db)?;
    try_patch_libs(&snake, base, db);

    let use_case = format!("create_{snake}");
    let bootstrapped = crate::harbor_toml::add_entity(base, &pascal, vec![use_case], db)
        .and_then(|_| regenerate_bootstrap(base))
        .is_ok();

    println!("✓ Scaffolded {pascal} (11 files).");
    if bootstrapped {
        println!("  harbor.toml updated + bootstrap.rs regenerated. No manual wiring needed.");
    } else {
        println!(
            "  Tip: run `harbor generate bootstrap` from your project root to wire automatically."
        );
    }

    Ok(())
}

/// Write the extra files that `harbor new --db` adds on top of the base template.
pub fn apply_db_to_new_project(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // .env.example
    write(
        &base.join(".env.example"),
        "DATABASE_URL=postgres://user:password@localhost/dbname\n",
    )?;

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

    // Patch Makefile setup target to also install sqlx-cli
    patch_makefile_setup_for_db(base)?;

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
        "\n[dependencies.sqlx]\nversion = \"0.8\"\nfeatures = [\"runtime-tokio-rustls\", \"postgres\", \"macros\", \"migrate\", \"uuid\"]\n",
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

// ── Templates ─────────────────────────────────────────────────────────────────

const MODEL: &str = r#"use super::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct {Pascal}Props {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct {Pascal} {
    pub name: String,
}

impl {Pascal} {
    pub fn new(props: {Pascal}Props) -> Result<Self, {Pascal}Error> {
        if props.name.trim().is_empty() {
            return Err({Pascal}Error::ValidationError("name_empty".into()));
        }
        Ok(Self { name: props.name })
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

use super::{errors::{Pascal}Error, model::{Pascal}};

#[async_trait]
pub trait {Pascal}RepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<{Pascal}>, {Pascal}Error>;
    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
        pub {Pascal}Repository {}

        #[async_trait]
        impl {Pascal}RepositoryTrait for {Pascal}Repository {
            async fn find_by_id(&self, id: &str) -> Result<Option<{Pascal}>, {Pascal}Error>;
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

pub struct Create{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
}

#[async_trait]
impl Create{Pascal}UseCaseTrait for Create{Pascal}UseCaseImpl {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        let entity = {Pascal}::new({Pascal}Props { name: params.name })?;
        self.repository.save(&entity).await?;
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{Pascal}Repository;

    #[tokio::test]
    async fn should_create_{snake}_when_name_is_valid() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Create{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
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

const INFRA_REPOSITORY: &str = r#"use async_trait::async_trait;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};

pub struct InMemory{Pascal}Repository;

#[async_trait]
impl {Pascal}RepositoryTrait for InMemory{Pascal}Repository {
    async fn find_by_id(&self, _id: &str) -> Result<Option<{Pascal}>, {Pascal}Error> {
        Ok(None)
    }

    async fn save(&self, _entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        Ok(())
    }
}
"#;

const DTO: &str = r#"use business::domain::{snake}::model::{Pascal};
use poem_openapi::Object;

#[derive(Debug, Object)]
pub struct {Pascal}Dto {
    pub name: String,
}

impl {Pascal}Dto {
    pub fn from_domain(entity: &{Pascal}) -> Self {
        Self {
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
    domain::{snake}::use_cases::create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
};
use poem_openapi::{OpenApi, payload::Json};

use crate::api::error::IntoErrorResponse;
use crate::api::{snake}::dto::{Create{Pascal}Request, {Pascal}Dto};
use crate::api::{snake}::responses::Create{Pascal}Response;

pub struct {Pascal}Api {
    pub create_{snake}: Arc<Create{Pascal}UseCaseImpl>,
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
                Create{Pascal}Response::from_status(status, error)
            }
        }
    }
}
"#;

// ── SQLx templates ────────────────────────────────────────────────────────────

const INFRA_ENTITY: &str = r#"use sqlx::FromRow;
use uuid::Uuid;

use business::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(FromRow)]
pub struct {Pascal}Db {
    pub id: Uuid,
    pub name: String,
}

impl TryFrom<{Pascal}Db> for {Pascal} {
    type Error = {Pascal}Error;

    fn try_from(row: {Pascal}Db) -> Result<Self, Self::Error> {
        Ok(Self::from_repository({Pascal} {
            name: row.name,
        }))
    }
}

impl From<&{Pascal}> for {Pascal}Db {
    fn from(entity: &{Pascal}) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: entity.name.clone(),
        }
    }
}
"#;

const INFRA_DB_REPOSITORY: &str = r#"use async_trait::async_trait;
use sqlx::PgPool;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};

use super::entity::{Pascal}Db;

pub struct Pg{Pascal}Repository {
    pub pool: PgPool,
}

#[async_trait]
impl {Pascal}RepositoryTrait for Pg{Pascal}Repository {
    async fn find_by_id(&self, id: &str) -> Result<Option<{Pascal}>, {Pascal}Error> {
        let row = sqlx::query_as!(
            {Pascal}Db,
            "SELECT id, name FROM {snake}s WHERE id = $1",
            id.parse::<uuid::Uuid>().ok()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| {Pascal}Error::RepositoryError)?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        let db = {Pascal}Db::from(entity);
        sqlx::query!(
            "INSERT INTO {snake}s (id, name) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET name = $2",
            db.id,
            db.name
        )
        .execute(&self.pool)
        .await
        .map_err(|_| {Pascal}Error::RepositoryError)?;
        Ok(())
    }
}
"#;

const DB_RS: &str = r#"use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_postgres_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to database")
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

pub struct {uc_pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
}

#[async_trait]
impl {uc_pascal}UseCaseTrait for {uc_pascal}UseCaseImpl {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{Pascal}Repository;

    #[tokio::test]
    async fn should_{uc}_when_valid() {
        // Arrange
        let mock_repo = Mock{Pascal}Repository::new();
        let use_case = {uc_pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
        };

        // Act
        // TODO: implement test body
        let _ = &use_case;
    }
}
"#;
