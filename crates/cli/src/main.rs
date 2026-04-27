mod harbor_toml;
mod scaffold;
mod snippets;

use cargo_generate::{GenerateArgs, TemplatePath, generate};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use include_dir::{Dir, include_dir};
use std::path::PathBuf;

static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/template");

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "harbor", about = "Harbor — Rust full-stack DDD framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new Harbor project
    New {
        /// Project name (skips interactive prompt)
        #[arg(long)]
        name: Option<String>,
        /// Include database support (SQLx + Postgres) (skips interactive prompt)
        #[arg(long, conflicts_with = "no_db")]
        db: bool,
        /// Explicitly skip database support without prompting
        #[arg(long, conflicts_with = "db")]
        no_db: bool,
        /// Directory where the project will be created (defaults to current directory)
        #[arg(long)]
        destination: Option<PathBuf>,
    },
    /// Code generators for existing Harbor projects
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// List entities and use cases defined in harbor.toml
    List,
    /// Print shell completion script
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Scaffold all DDD layers for a new entity
    Scaffold {
        /// Entity name in PascalCase (e.g. Product, OrderItem)
        name: String,
        /// Generate a SQLx (Postgres) repository instead of InMemory
        #[arg(long)]
        db: bool,
        /// Generate full CRUD use cases (create, get, list, update, delete)
        #[arg(long)]
        crud: bool,
    },
    /// Add a use case to an existing entity
    UseCase {
        /// Entity name in PascalCase (e.g. Product)
        entity: String,
        /// Use case action in snake_case (e.g. delete_product)
        action: String,
    },
    /// Regenerate presentation/src/generated/bootstrap.rs from harbor.toml
    Bootstrap,
    /// Create a new SQLx migration file
    Migration {
        /// Migration name in snake_case (e.g. add_products_table)
        name: String,
    },
    /// Write IDE snippet files (.zed/snippets/rust.json, .vscode/harbor.code-snippets)
    Snippets {
        /// Target IDE: zed or vscode (default: both)
        #[arg(long)]
        ide: Option<String>,
    },
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Extract the embedded template to a temp directory and return its path.
/// The caller is responsible for cleaning up the directory when done.
fn extract_template() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    TEMPLATE_DIR.extract(tmp.path())?;
    Ok(tmp)
}

/// Core project generation. `name = None` lets cargo-generate prompt interactively.
fn generate_new_project(
    name: Option<String>,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let tmp = extract_template()?;

    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(tmp.path().to_string_lossy().into_owned()),
            ..Default::default()
        },
        name: name.clone(),
        destination,
        no_workspace: true,
        // Suppress cargo-generate's [1/N] Skipped/Done progress output
        quiet: true,
        ..Default::default()
    };

    let output = generate(args)?;
    // tmp is dropped here — extracted template cleaned up after generation
    Ok(output)
}

/// Runs `harbor new`. Prompts for any values not provided as flags.
fn new_project(
    name: Option<String>,
    db: bool,
    no_db: bool,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let resolved_db = if db {
        true
    } else if no_db {
        false
    } else if dialoguer::console::user_attended() {
        dialoguer::Confirm::new()
            .with_prompt("Include database support (SQLx + Postgres)?")
            .default(false)
            .interact()?
    } else {
        false
    };

    eprintln!("Constructing project skeleton...");

    let output = generate_new_project(name, destination)?;

    if resolved_db {
        scaffold::apply_db_to_new_project(&output)?;
    }

    snippets::apply(&output, None)?;

    Ok(output)
}

fn require_harbor_project(cwd: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    if !cwd.join("harbor.toml").exists() {
        return Err(
            "harbor.toml not found — run this command from the root of a Harbor project".into(),
        );
    }
    Ok(())
}

fn run_list(cwd: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    require_harbor_project(cwd)?;
    let config = harbor_toml::read(cwd)?;
    println!("Project: {}", config.project.name);
    if config.project.db {
        println!("Database: enabled");
    }
    println!();
    if config.entity.is_empty() {
        println!("No entities defined.");
        return Ok(());
    }
    for entity in &config.entity {
        let repo_kind = if entity.db { "SQLx" } else { "InMemory" };
        println!("  {} [{}]", entity.name, repo_kind);
        for uc in &entity.use_cases {
            println!("    · {uc}");
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Commands::New {
            name,
            db,
            no_db,
            destination,
        } => new_project(name, db, no_db, destination).map(|path| {
            let project_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            println!("├── business/        (domain logic)");
            println!("├── infrastructure/  (adapters)");
            println!("├── presentation/    (openapi routes)");
            println!("└── Cargo.toml");
            println!("✓ Project '{project_name}' created.\n");
            println!("Next steps:");
            println!("  cd {project_name}");
            println!("  cargo run -p {project_name}");
            println!();
            println!("API docs available at  http://localhost:8080");
            println!("Add an entity with:    harbor generate scaffold <Name> --crud");
        }),
        Commands::Generate {
            action: GenerateAction::Scaffold { name, db, crud },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            require_harbor_project(&cwd).and_then(|_| scaffold::run(&name, &cwd, db, crud))
        }
        Commands::Generate {
            action: GenerateAction::UseCase { entity, action },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            require_harbor_project(&cwd)
                .and_then(|_| scaffold::run_use_case(&entity, &action, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::Bootstrap,
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            require_harbor_project(&cwd).and_then(|_| {
                scaffold::regenerate_bootstrap(&cwd)
                    .map(|_| println!("✓ bootstrap.rs regenerated."))
            })
        }
        Commands::Generate {
            action: GenerateAction::Migration { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            require_harbor_project(&cwd).and_then(|_| scaffold::run_migration(&name, &cwd, None))
        }
        Commands::Generate {
            action: GenerateAction::Snippets { ide },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            require_harbor_project(&cwd).and_then(|_| snippets::run(&cwd, ide.as_deref()))
        }
        Commands::List => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            run_list(&cwd)
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "harbor", &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_generate::Vcs;
    use serde_json;
    use std::fs;
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("harbor_test_{name}"))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn generate_project(
        name: &str,
        destination: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let tmp = extract_template()?;

        let args = GenerateArgs {
            template_path: TemplatePath {
                path: Some(tmp.path().to_string_lossy().into_owned()),
                ..Default::default()
            },
            name: Some(name.to_string()),
            destination: Some(destination.to_path_buf()),
            vcs: Some(Vcs::None),
            no_workspace: true,
            ..Default::default()
        };

        let output_dir = generate(args)?;
        drop(tmp); // extracted template no longer needed
        Ok(output_dir)
    }

    // ── harbor new (non-interactive / flag-driven) ────────────────────────────

    #[test]
    fn new_name_flag_sets_project_name() {
        let dir = temp_dir("new_name_flag");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output =
            new_project_non_interactive(Some("explicit-name".into()), false, &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/Cargo.toml")).unwrap();
        assert!(content.contains("name = \"explicit-name\""));

        cleanup(&dir);
    }

    #[test]
    fn new_db_flag_creates_db_files() {
        let dir = temp_dir("new_db_flag");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = new_project_non_interactive(Some("db-project".into()), true, &dir).unwrap();

        assert!(output.join("docker-compose.yml").exists());
        assert!(output.join(".env").exists());
        assert!(output.join(".env.example").exists());
        assert!(output.join(".cargo/config.toml").exists());
        assert!(output.join("infrastructure/migrations").is_dir());
        assert!(output.join("infrastructure/src/db.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn new_without_db_flag_has_no_db_files() {
        let dir = temp_dir("new_no_db");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output =
            new_project_non_interactive(Some("plain-project".into()), false, &dir).unwrap();

        assert!(!output.join(".env.example").exists());
        assert!(!output.join(".cargo/config.toml").exists());
        assert!(!output.join("infrastructure/src/db.rs").exists());

        cleanup(&dir);
    }

    // ── harbor new ────────────────────────────────────────────────────────────

    #[test]
    fn creates_project_structure() {
        let dir = temp_dir("cg_structure");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("test-app", &dir).unwrap();

        assert!(output.join("Cargo.toml").exists());
        assert!(output.join("business/src/lib.rs").exists());
        assert!(output.join("infrastructure/src/lib.rs").exists());
        assert!(output.join("presentation/src/main.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn presentation_cargo_toml_has_project_name() {
        let dir = temp_dir("cg_cargo");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("my-harbor-app", &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/Cargo.toml")).unwrap();
        assert!(content.contains("name = \"my-harbor-app\""));

        cleanup(&dir);
    }

    #[test]
    fn main_rs_is_minimal_entry_point() {
        let dir = temp_dir("cg_main");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("cool-project", &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();
        // main.rs now delegates to generated::bootstrap — no raw DI wiring here
        assert!(content.contains("generated::bootstrap::build_app"));
        assert!(content.contains("mod generated"));
        assert!(!content.contains("{{project-name}}"));

        cleanup(&dir);
    }

    #[test]
    fn bootstrap_rs_wires_ddd_layers() {
        let dir = temp_dir("cg_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("api-test", &dir).unwrap();

        let content =
            fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("GetGreetingUseCaseImpl"));
        assert!(content.contains("InMemoryGreetingRepository"));
        assert!(content.contains("GreetingApi"));
        assert!(content.contains("OpenApiService"));
        assert!(content.contains("8080"));

        cleanup(&dir);
    }

    #[test]
    fn ddd_layers_exist() {
        let dir = temp_dir("cg_ddd");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("ddd-app", &dir).unwrap();

        assert!(
            output
                .join("business/src/domain/greeting/model.rs")
                .exists()
        );
        assert!(
            output
                .join("business/src/domain/greeting/errors.rs")
                .exists()
        );
        assert!(
            output
                .join("business/src/domain/greeting/repository.rs")
                .exists()
        );
        assert!(
            output
                .join("business/src/domain/greeting/use_cases/get_greeting.rs")
                .exists()
        );
        assert!(
            output
                .join("business/src/application/greeting/get_greeting.rs")
                .exists()
        );
        assert!(
            output
                .join("infrastructure/src/greeting/repository.rs")
                .exists()
        );
        assert!(
            output
                .join("presentation/src/api/greeting/routes.rs")
                .exists()
        );
        assert!(output.join("presentation/src/api/greeting/dto.rs").exists());
        assert!(
            output
                .join("presentation/src/api/greeting/error_mapper.rs")
                .exists()
        );
        assert!(
            output
                .join("presentation/src/api/greeting/responses.rs")
                .exists()
        );
        // New bootstrap infrastructure
        assert!(output.join("harbor.toml").exists());
        assert!(output.join("presentation/src/generated.rs").exists());
        assert!(
            output
                .join("presentation/src/generated/bootstrap.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn harbor_toml_has_project_name_and_greeting_entity() {
        let dir = temp_dir("cg_harbor_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("my-app", &dir).unwrap();

        let content = fs::read_to_string(output.join("harbor.toml")).unwrap();
        assert!(content.contains("name = \"my-app\""));
        assert!(content.contains("name = \"Greeting\""));
        assert!(content.contains("get_greeting"));

        cleanup(&dir);
    }

    #[test]
    #[ignore = "slow: compiles and tests a full generated project"]
    fn generated_project_compiles_and_tests_pass() {
        let dir = temp_dir("cg_compile");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("compile-test", &dir).unwrap();

        let result = std::process::Command::new("cargo")
            .args(["test", "--workspace"])
            .current_dir(&output)
            .output()
            .expect("failed to run cargo test");

        if !result.status.success() {
            eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
            eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
            panic!("cargo test failed in generated project");
        }

        cleanup(&dir);
    }

    // ── harbor generate scaffold ──────────────────────────────────────────────

    #[test]
    fn scaffold_creates_all_files_for_single_word_entity() {
        let dir = temp_dir("scaffold_single_word");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("Product", &dir, false, false).unwrap();

        // Domain
        assert!(dir.join("business/src/domain/product/model.rs").exists());
        assert!(dir.join("business/src/domain/product/errors.rs").exists());
        assert!(
            dir.join("business/src/domain/product/repository.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases/create_product.rs")
                .exists()
        );
        // Application
        assert!(
            dir.join("business/src/application/product/create_product.rs")
                .exists()
        );
        // Infrastructure
        assert!(
            dir.join("infrastructure/src/product/repository.rs")
                .exists()
        );
        // Presentation
        assert!(dir.join("presentation/src/api/product.rs").exists());
        assert!(dir.join("presentation/src/api/product/dto.rs").exists());
        assert!(
            dir.join("presentation/src/api/product/responses.rs")
                .exists()
        );
        assert!(
            dir.join("presentation/src/api/product/error_mapper.rs")
                .exists()
        );
        assert!(dir.join("presentation/src/api/product/routes.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn scaffold_creates_all_files_for_multi_word_entity() {
        let dir = temp_dir("scaffold_multi_word");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("OrderItem", &dir, false, false).unwrap();

        assert!(dir.join("business/src/domain/order_item/model.rs").exists());
        assert!(
            dir.join("business/src/domain/order_item/use_cases/create_order_item.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/application/order_item/create_order_item.rs")
                .exists()
        );
        assert!(
            dir.join("infrastructure/src/order_item/repository.rs")
                .exists()
        );
        assert!(
            dir.join("presentation/src/api/order_item/routes.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn scaffold_normalizes_lowercase_input() {
        let dir = temp_dir("scaffold_lowercase");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("product", &dir, false, false).unwrap();

        assert!(dir.join("business/src/domain/product/model.rs").exists());
        assert!(
            dir.join("business/src/application/product/create_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn scaffold_substitutes_pascal_name_in_model() {
        let dir = temp_dir("scaffold_model_content");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("Product", &dir, false, false).unwrap();

        let content = fs::read_to_string(dir.join("business/src/domain/product/model.rs")).unwrap();
        assert!(content.contains("pub struct ProductProps"));
        assert!(content.contains("pub struct Product {"));
        assert!(content.contains("ProductError::ValidationError"));
        assert!(content.contains("product.validation_error.name_empty"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_substitutes_pascal_name_in_errors() {
        let dir = temp_dir("scaffold_errors_content");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("Product", &dir, false, false).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/domain/product/errors.rs")).unwrap();
        assert!(content.contains("pub enum ProductError"));
        assert!(content.contains("product.not_found"));
        assert!(content.contains("product.repository_error"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_substitutes_pascal_name_in_use_case_impl() {
        let dir = temp_dir("scaffold_impl_content");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("Product", &dir, false, false).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/application/product/create_product.rs"))
                .unwrap();
        assert!(!content.contains("Create{Pascal}UseCaseImpl"));
        assert!(content.contains("CreateProductUseCaseImpl"));
        assert!(content.contains("ProductRepositoryTrait"));
        assert!(content.contains("product.validation_error.name_empty"));

        cleanup(&dir);
    }

    // ── lib.rs auto-patching ──────────────────────────────────────────────────

    fn setup_harbor_stubs(base: &Path) {
        fs::create_dir_all(base.join("business/src")).unwrap();
        fs::write(
            base.join("business/src/lib.rs"),
            "pub mod domain {\n  pub mod greeting {\n    pub mod errors;\n    pub mod model;\n    pub mod repository;\n    pub mod use_cases {\n      pub mod get_greeting;\n    }\n  }\n}\npub mod application {\n  pub mod greeting {\n    pub mod get_greeting;\n  }\n}\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("infrastructure/src")).unwrap();
        fs::write(
            base.join("infrastructure/src/lib.rs"),
            "pub mod greeting {\n    pub mod repository;\n}\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("presentation/src")).unwrap();
        fs::write(
            base.join("presentation/src/api.rs"),
            "pub mod error;\npub mod greeting;\n",
        )
        .unwrap();

        fs::write(
            base.join("harbor.toml"),
            "[project]\nname = \"test-app\"\n\n[[entity]]\nname = \"Greeting\"\nuse_cases = [\"get_greeting\"]\n",
        )
        .unwrap();
    }

    #[test]
    fn scaffold_patches_business_lib_rs() {
        let dir = temp_dir("scaffold_patch_business");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();

        let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
        // domain block now contains product
        assert!(content.contains("pub mod product {"));
        assert!(content.contains("pub mod errors;"));
        assert!(content.contains("pub mod use_cases {"));
        assert!(!content.contains("pub mod use_cases;")); // never the bare form
        // application block now contains product
        assert!(content.contains("pub mod create_product;"));
        // greeting still present
        assert!(content.contains("pub mod greeting {"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_patches_infra_lib_rs() {
        let dir = temp_dir("scaffold_patch_infra");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();

        let content = fs::read_to_string(dir.join("infrastructure/src/lib.rs")).unwrap();
        assert!(content.contains("pub mod product {"));
        assert!(content.contains("pub mod repository;"));
        // greeting still present
        assert!(content.contains("pub mod greeting {"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_patches_api_rs() {
        let dir = temp_dir("scaffold_patch_api");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();

        let content = fs::read_to_string(dir.join("presentation/src/api.rs")).unwrap();
        assert!(content.contains("pub mod product;"));
        // error and greeting still present
        assert!(content.contains("pub mod error;"));
        assert!(content.contains("pub mod greeting;"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_updates_harbor_toml_and_regenerates_bootstrap() {
        let dir = temp_dir("scaffold_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();

        // harbor.toml now contains both entities
        let toml = fs::read_to_string(dir.join("harbor.toml")).unwrap();
        assert!(toml.contains("name = \"Product\""));
        assert!(toml.contains("create_product"));
        assert!(toml.contains("name = \"Greeting\"")); // original still present

        // bootstrap.rs was regenerated with both entities wired
        let bootstrap =
            fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(bootstrap.contains("CreateProductUseCaseImpl"));
        assert!(bootstrap.contains("InMemoryProductRepository"));
        assert!(bootstrap.contains("ProductApi"));
        assert!(bootstrap.contains("GetGreetingUseCaseImpl"));
        assert!(bootstrap.contains("InMemoryGreetingRepository"));
        assert!(bootstrap.contains("GreetingApi"));

        cleanup(&dir);
    }

    // ── harbor generate use-case ──────────────────────────────────────────────

    fn setup_use_case_stubs(base: &Path) {
        fs::write(
            base.join("harbor.toml"),
            "[project]\nname = \"test-app\"\n\n[[entity]]\nname = \"Product\"\nuse_cases = [\"create_product\"]\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("business/src/domain/product")).unwrap();
        fs::write(
            base.join("business/src/lib.rs"),
            "pub mod domain {\n    pub mod product {\n        pub mod errors;\n        pub mod model;\n        pub mod repository;\n        pub mod use_cases {\n            pub mod create_product;\n        }\n    }\n}\npub mod application {\n    pub mod product {\n        pub mod create_product;\n    }\n}\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("business/src/application/product")).unwrap();
    }

    #[test]
    fn use_case_creates_trait_file() {
        let dir = temp_dir("uc_trait");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        assert!(
            dir.join("business/src/domain/product/use_cases/delete_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn use_case_creates_impl_file() {
        let dir = temp_dir("uc_impl");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        assert!(
            dir.join("business/src/application/product/delete_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn use_case_trait_file_has_correct_content() {
        let dir = temp_dir("uc_trait_content");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/domain/product/use_cases/delete_product.rs"))
                .unwrap();
        assert!(content.contains("DeleteProductParams"));
        assert!(content.contains("DeleteProductUseCaseTrait"));

        cleanup(&dir);
    }

    #[test]
    fn use_case_impl_file_has_correct_content() {
        let dir = temp_dir("uc_impl_content");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/application/product/delete_product.rs"))
                .unwrap();
        assert!(content.contains("DeleteProductUseCaseImpl"));
        assert!(content.contains("ProductRepositoryTrait"));
        assert!(content.contains("DeleteProductUseCaseTrait"));

        cleanup(&dir);
    }

    #[test]
    fn use_case_patches_business_lib_rs() {
        let dir = temp_dir("uc_patch_lib");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
        // domain use_cases block has new entry
        assert!(content.contains("pub mod delete_product;"));
        assert!(content.contains("pub mod create_product;")); // existing preserved
        // application block also has new entry
        assert_eq!(content.matches("pub mod delete_product;").count(), 2); // domain + application

        cleanup(&dir);
    }

    #[test]
    fn use_case_updates_harbor_toml() {
        let dir = temp_dir("uc_harbor_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content = fs::read_to_string(dir.join("harbor.toml")).unwrap();
        assert!(content.contains("delete_product"));
        assert!(content.contains("create_product")); // existing preserved

        cleanup(&dir);
    }

    #[test]
    fn use_case_regenerates_bootstrap() {
        let dir = temp_dir("uc_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content =
            fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("DeleteProductUseCaseImpl"));
        assert!(content.contains("CreateProductUseCaseImpl"));

        cleanup(&dir);
    }

    #[test]
    fn use_case_normalizes_entity_casing() {
        let dir = temp_dir("uc_normalize");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        // lowercase "product" should resolve to "Product"
        scaffold::run_use_case("product", "delete_product", &dir).unwrap();

        assert!(
            dir.join("business/src/domain/product/use_cases/delete_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn use_case_errors_when_entity_not_in_harbor_toml() {
        let dir = temp_dir("uc_missing_entity");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);

        let result = scaffold::run_use_case("NonExistent", "do_something", &dir);
        assert!(result.is_err());

        cleanup(&dir);
    }

    // ── harbor new --db ───────────────────────────────────────────────────────

    /// Non-interactive wrapper for tests — always passes name so no TTY is needed.
    fn new_project_non_interactive(
        name: Option<String>,
        db: bool,
        destination: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let tmp = extract_template()?;
        let args = GenerateArgs {
            template_path: TemplatePath {
                path: Some(tmp.path().to_string_lossy().into_owned()),
                ..Default::default()
            },
            name: name.clone(),
            destination: Some(destination.to_path_buf()),
            vcs: Some(Vcs::None),
            no_workspace: true,
            ..Default::default()
        };
        let output = generate(args)?;
        drop(tmp);
        if db {
            scaffold::apply_db_to_new_project(&output)?;
        }
        Ok(output)
    }

    fn generate_db_project(
        name: &str,
        destination: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let output = generate_project(name, destination)?;
        scaffold::apply_db_to_new_project(&output)?;
        Ok(output)
    }

    #[test]
    fn db_project_has_env_example() {
        let dir = temp_dir("db_env");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let content = fs::read_to_string(output.join(".env.example")).unwrap();
        assert!(content.contains("DATABASE_URL"));
        assert!(content.contains("db_app")); // project name embedded
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_env_file_with_project_db_name() {
        let dir = temp_dir("db_env_file");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("my-api", &dir).unwrap();
        let content = fs::read_to_string(output.join(".env")).unwrap();
        assert!(content.contains("DATABASE_URL"));
        assert!(content.contains("my_api")); // hyphens → underscores
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_docker_compose_with_project_name() {
        let dir = temp_dir("db_docker_compose");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("my-api", &dir).unwrap();
        let content = fs::read_to_string(output.join("docker-compose.yml")).unwrap();
        assert!(content.contains("postgres:16"));
        assert!(content.contains("my-api-postgres")); // container name
        assert!(content.contains("my_api")); // db name
        assert!(content.contains("5432"));
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_gitignore_with_env() {
        let dir = temp_dir("db_gitignore");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let gitignore_path = output.join(".gitignore");
        // either cargo-generate created it (and we appended .env) or we created it
        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path).unwrap();
            assert!(content.contains(".env"));
        }
        cleanup(&dir);
    }

    #[test]
    fn db_project_makefile_has_docker_and_sqlx_targets() {
        let dir = temp_dir("db_makefile_targets");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let content = fs::read_to_string(output.join("Makefile")).unwrap();
        assert!(content.contains("docker/up"));
        assert!(content.contains("docker/down"));
        assert!(content.contains("sqlx/migrate"));
        assert!(content.contains("sqlx/prepare"));
        assert!(content.contains("sqlx/online"));
        assert!(content.contains("sqlx/offline"));
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_cargo_config_with_sqlx_offline() {
        let dir = temp_dir("db_cargo_config");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let content = fs::read_to_string(output.join(".cargo/config.toml")).unwrap();
        assert!(content.contains("SQLX_OFFLINE"));
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_migrations_dir() {
        let dir = temp_dir("db_migrations");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        assert!(output.join("infrastructure/migrations").is_dir());
        cleanup(&dir);
    }

    #[test]
    fn db_project_has_db_rs_with_pool_function() {
        let dir = temp_dir("db_db_rs");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let content = fs::read_to_string(output.join("infrastructure/src/db.rs")).unwrap();
        assert!(content.contains("create_postgres_pool"));
        assert!(content.contains("PgPool"));
        cleanup(&dir);
    }

    #[test]
    fn db_project_infrastructure_cargo_has_sqlx() {
        let dir = temp_dir("db_infra_cargo");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let content = fs::read_to_string(output.join("infrastructure/Cargo.toml")).unwrap();
        assert!(content.contains("sqlx"));
        assert!(content.contains("postgres"));
        cleanup(&dir);
    }

    #[test]
    fn db_project_harbor_toml_has_project_db_true() {
        let dir = temp_dir("db_proj_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_db_project("db-app", &dir).unwrap();
        let config = harbor_toml::read(&output).unwrap();
        assert!(
            config.project.db,
            "project.db should be true for --db projects"
        );
        cleanup(&dir);
    }

    #[test]
    fn no_db_project_harbor_toml_omits_project_db() {
        let dir = temp_dir("no_db_proj_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        let output = generate_project("plain-app", &dir).unwrap();
        let config = harbor_toml::read(&output).unwrap();
        assert!(
            !config.project.db,
            "project.db should be false for non-db projects"
        );
        let content = fs::read_to_string(output.join("harbor.toml")).unwrap();
        assert!(
            !content.contains("db = true"),
            "harbor.toml should not contain db = true for non-db projects"
        );
        cleanup(&dir);
    }

    // ── harbor generate scaffold --db ─────────────────────────────────────────

    fn setup_db_harbor_stubs(base: &Path) {
        setup_harbor_stubs(base);
        // add the db infrastructure files that harbor new --db would create
        fs::create_dir_all(base.join("infrastructure/migrations")).unwrap();
    }

    #[test]
    fn scaffold_db_creates_entity_rs() {
        let dir = temp_dir("scaffold_db_entity");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_db_harbor_stubs(&dir);
        scaffold::run("Product", &dir, true, false).unwrap();
        assert!(dir.join("infrastructure/src/product/entity.rs").exists());
        cleanup(&dir);
    }

    #[test]
    fn scaffold_db_repository_uses_pgpool() {
        let dir = temp_dir("scaffold_db_repo");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_db_harbor_stubs(&dir);
        scaffold::run("Product", &dir, true, false).unwrap();
        let content =
            fs::read_to_string(dir.join("infrastructure/src/product/repository.rs")).unwrap();
        assert!(content.contains("PgPool"));
        assert!(content.contains("PgProductRepository"));
        assert!(!content.contains("InMemoryProductRepository"));
        cleanup(&dir);
    }

    #[test]
    fn scaffold_db_harbor_toml_has_db_true() {
        let dir = temp_dir("scaffold_db_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_db_harbor_stubs(&dir);
        scaffold::run("Product", &dir, true, false).unwrap();
        let content = fs::read_to_string(dir.join("harbor.toml")).unwrap();
        assert!(content.contains("db = true"));
        cleanup(&dir);
    }

    #[test]
    fn scaffold_db_bootstrap_uses_pg_repo() {
        let dir = temp_dir("scaffold_db_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_db_harbor_stubs(&dir);
        scaffold::run("Product", &dir, true, false).unwrap();
        let content =
            fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("PgProductRepository"));
        assert!(!content.contains("InMemoryProductRepository"));
        // Greeting (non-db) still uses InMemory
        assert!(content.contains("InMemoryGreetingRepository"));
        cleanup(&dir);
    }

    #[test]
    fn scaffold_without_db_still_uses_inmemory() {
        let dir = temp_dir("scaffold_no_db");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();
        let content =
            fs::read_to_string(dir.join("infrastructure/src/product/repository.rs")).unwrap();
        assert!(content.contains("InMemoryProductRepository"));
        assert!(!content.contains("PgPool"));
        assert!(!dir.join("infrastructure/src/product/entity.rs").exists());
        cleanup(&dir);
    }

    // ── harbor generate migration ─────────────────────────────────────────────

    #[test]
    fn migration_errors_when_sqlx_not_in_path() {
        let dir = temp_dir("migration_no_sqlx");
        cleanup(&dir);
        fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();

        let result =
            scaffold::run_migration("add_products_table", &dir, Some("nonexistent_sqlx_bin"));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("sqlx CLI not found"),
            "expected 'sqlx CLI not found' in: {msg}"
        );
        assert!(
            msg.contains("cargo install sqlx-cli"),
            "expected install hint in: {msg}"
        );

        cleanup(&dir);
    }

    #[test]
    fn migration_creates_migrations_dir_when_missing() {
        let dir = temp_dir("migration_no_dir");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        // infrastructure/migrations does NOT exist — should be created automatically

        // /bin/true acts as a stub sqlx: passes the existence check, returns exit 0
        let _ = scaffold::run_migration("add_products_table", &dir, Some("/bin/true"));

        assert!(
            dir.join("infrastructure/migrations").exists(),
            "expected infrastructure/migrations to be created automatically"
        );

        cleanup(&dir);
    }

    #[test]
    fn use_case_is_idempotent() {
        let dir = temp_dir("uc_idempotent");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap(); // second run

        let toml = fs::read_to_string(dir.join("harbor.toml")).unwrap();
        assert_eq!(toml.matches("delete_product").count(), 1);

        let lib = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
        assert_eq!(lib.matches("pub mod delete_product;").count(), 2); // domain + application

        cleanup(&dir);
    }

    // ── Phase 5: logger ───────────────────────────────────────────────────────

    #[test]
    fn new_project_has_domain_logger_file() {
        let dir = temp_dir("logger_domain_file");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("logger-test", &dir).unwrap();

        assert!(output.join("business/src/domain/logger.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn new_project_has_infrastructure_logger_file() {
        let dir = temp_dir("logger_infra_file");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("logger-test", &dir).unwrap();

        assert!(output.join("infrastructure/src/logger.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn new_project_bootstrap_wires_tracing_logger() {
        let dir = temp_dir("logger_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("logger-test", &dir).unwrap();

        let content =
            fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("TracingLogger"));
        assert!(content.contains("let logger: Arc<dyn LoggerTrait> = Arc::new(TracingLogger)"));
        assert!(content.contains("Arc::clone(&logger)"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_use_case_impl_has_logger_field() {
        let dir = temp_dir("logger_scaffold_impl");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        scaffold::run("Product", &dir, false, false).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/application/product/create_product.rs"))
                .unwrap();
        assert!(content.contains("pub logger: Arc<dyn LoggerTrait>"));
        assert!(content.contains("use crate::domain::logger::LoggerTrait"));

        cleanup(&dir);
    }

    #[test]
    fn use_case_generator_impl_has_logger_field() {
        let dir = temp_dir("logger_uc_impl");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_use_case_stubs(&dir);
        scaffold::run_use_case("Product", "delete_product", &dir).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/application/product/delete_product.rs"))
                .unwrap();
        assert!(content.contains("pub logger: Arc<dyn LoggerTrait>"));
        assert!(content.contains("use crate::domain::logger::LoggerTrait"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_bootstrap_wires_logger_for_all_entities() {
        let dir = temp_dir("logger_bootstrap_scaffold");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs(&dir);
        scaffold::run("Product", &dir, false, false).unwrap();

        let content =
            fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("TracingLogger"));
        assert!(content.contains("let logger: Arc<dyn LoggerTrait> = Arc::new(TracingLogger)"));
        // 3 clones per entity (repo, use case, api struct) × 2 entities (Greeting + Product)
        assert_eq!(content.matches("Arc::clone(&logger)").count(), 6);

        cleanup(&dir);
    }

    // ── Phase 6: IDE snippets ─────────────────────────────────────────────────

    #[test]
    fn new_project_creates_zed_snippet_file() {
        let dir = temp_dir("snippets_zed_new");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = new_project_non_interactive(Some("snip-test".into()), false, &dir).unwrap();
        snippets::apply(&output, None).unwrap();

        assert!(output.join(".zed/snippets/rust.json").exists());

        cleanup(&dir);
    }

    #[test]
    fn new_project_creates_vscode_snippet_file() {
        let dir = temp_dir("snippets_vscode_new");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = new_project_non_interactive(Some("snip-test".into()), false, &dir).unwrap();
        snippets::apply(&output, None).unwrap();

        assert!(output.join(".vscode/harbor.code-snippets").exists());

        cleanup(&dir);
    }

    #[test]
    fn snippet_files_contain_valid_json() {
        let dir = temp_dir("snippets_valid_json");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        snippets::apply(&dir, None).unwrap();

        let zed = fs::read_to_string(dir.join(".zed/snippets/rust.json")).unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&zed).is_ok(),
            "zed snippet JSON invalid"
        );

        let vscode = fs::read_to_string(dir.join(".vscode/harbor.code-snippets")).unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&vscode).is_ok(),
            "vscode snippet JSON invalid"
        );

        cleanup(&dir);
    }

    #[test]
    fn generate_snippets_ide_zed_creates_only_zed_file() {
        let dir = temp_dir("snippets_ide_zed");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        snippets::run(&dir, Some("zed")).unwrap();

        assert!(dir.join(".zed/snippets/rust.json").exists());
        assert!(!dir.join(".vscode/harbor.code-snippets").exists());

        cleanup(&dir);
    }

    #[test]
    fn generate_snippets_ide_vscode_creates_only_vscode_file() {
        let dir = temp_dir("snippets_ide_vscode");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        snippets::run(&dir, Some("vscode")).unwrap();

        assert!(!dir.join(".zed/snippets/rust.json").exists());
        assert!(dir.join(".vscode/harbor.code-snippets").exists());

        cleanup(&dir);
    }

    #[test]
    fn generate_snippets_is_idempotent() {
        let dir = temp_dir("snippets_idempotent");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        snippets::run(&dir, None).unwrap();
        snippets::run(&dir, None).unwrap(); // second run must not error

        assert!(dir.join(".zed/snippets/rust.json").exists());
        assert!(dir.join(".vscode/harbor.code-snippets").exists());

        cleanup(&dir);
    }

    #[test]
    fn generate_snippets_unknown_ide_returns_error() {
        let dir = temp_dir("snippets_unknown_ide");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = snippets::run(&dir, Some("neovim"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown IDE"));

        cleanup(&dir);
    }

    // ── harbor generate scaffold --crud ──────────────────────────────────────

    fn setup_harbor_stubs_for_crud(base: &Path) {
        setup_harbor_stubs(base);
    }

    #[test]
    fn scaffold_crud_creates_all_domain_use_case_files() {
        let dir = temp_dir("scaffold_crud_domain_uc");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        scaffold::run("Product", &dir, false, true).unwrap();

        assert!(
            dir.join("business/src/domain/product/use_cases/create_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases/get_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases/list_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases/update_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases/delete_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn scaffold_crud_creates_all_application_use_case_files() {
        let dir = temp_dir("scaffold_crud_app_uc");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        scaffold::run("Product", &dir, false, true).unwrap();

        assert!(
            dir.join("business/src/application/product/create_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/application/product/get_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/application/product/list_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/application/product/update_product.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/application/product/delete_product.rs")
                .exists()
        );

        cleanup(&dir);
    }

    #[test]
    fn scaffold_crud_repository_has_find_all() {
        let dir = temp_dir("scaffold_crud_repo");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        scaffold::run("Product", &dir, false, true).unwrap();

        let content =
            fs::read_to_string(dir.join("business/src/domain/product/repository.rs")).unwrap();
        assert!(content.contains("async fn find_all"));
        assert!(content.contains("async fn find_by_id"));
        assert!(content.contains("async fn save"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_crud_routes_has_all_http_methods() {
        let dir = temp_dir("scaffold_crud_routes");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        scaffold::run("Product", &dir, false, true).unwrap();

        let content =
            fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap();
        assert!(content.contains("method = \"post\""));
        assert!(content.contains("method = \"get\""));
        assert!(content.contains("method = \"put\""));
        assert!(content.contains("method = \"delete\""));
        assert!(content.contains("pub struct ProductApi"));
        assert!(content.contains("pub create_product"));
        assert!(content.contains("pub get_product"));
        assert!(content.contains("pub list_product"));
        assert!(content.contains("pub update_product"));
        assert!(content.contains("pub delete_product"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_crud_patches_business_lib_with_all_use_cases() {
        let dir = temp_dir("scaffold_crud_lib");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        scaffold::run("Product", &dir, false, true).unwrap();

        let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
        assert!(content.contains("pub mod create_product;"));
        assert!(content.contains("pub mod get_product;"));
        assert!(content.contains("pub mod list_product;"));
        assert!(content.contains("pub mod update_product;"));
        assert!(content.contains("pub mod delete_product;"));

        cleanup(&dir);
    }

    #[test]
    fn scaffold_crud_bootstrap_wires_all_use_cases() {
        let dir = temp_dir("scaffold_crud_bootstrap");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();
        setup_harbor_stubs_for_crud(&dir);
        fs::create_dir_all(dir.join("presentation/src/generated")).unwrap();
        fs::write(
            dir.join("presentation/src/generated/bootstrap.rs"),
            "// placeholder\n",
        )
        .unwrap();
        scaffold::run("Product", &dir, false, true).unwrap();

        let content =
            fs::read_to_string(dir.join("presentation/src/generated/bootstrap.rs")).unwrap();
        assert!(content.contains("CreateProductUseCaseImpl"));
        assert!(content.contains("GetProductUseCaseImpl"));
        assert!(content.contains("ListProductUseCaseImpl"));
        assert!(content.contains("UpdateProductUseCaseImpl"));
        assert!(content.contains("DeleteProductUseCaseImpl"));

        cleanup(&dir);
    }

    // ── harbor list ──────────────────────────────────────────────────────────

    #[test]
    fn list_fails_outside_harbor_project() {
        let dir = temp_dir("list_no_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = run_list(&dir);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("harbor.toml not found"));

        cleanup(&dir);
    }

    #[test]
    fn list_succeeds_inside_harbor_project() {
        let dir = temp_dir("list_with_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("list-app", &dir).unwrap();
        // should not error — project has a harbor.toml with Greeting entity
        run_list(&output).unwrap();

        cleanup(&dir);
    }

    // ── require_harbor_project ────────────────────────────────────────────────

    #[test]
    fn generate_scaffold_fails_outside_harbor_project() {
        let dir = temp_dir("gen_no_toml");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = require_harbor_project(&dir);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("harbor.toml not found")
        );

        cleanup(&dir);
    }
}
