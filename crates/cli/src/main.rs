mod harbor_toml;
mod scaffold;

use cargo_generate::{GenerateArgs, TemplatePath, generate};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    New,
    /// Code generators for existing Harbor projects
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Scaffold all DDD layers for a new entity
    Scaffold {
        /// Entity name in PascalCase (e.g. Product, OrderItem)
        name: String,
    },
    /// Regenerate presentation/src/generated/bootstrap.rs from harbor.toml
    Bootstrap,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

fn templates_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("../template");
    dir
}

fn new_project() -> Result<String, Box<dyn std::error::Error>> {
    let template_dir = templates_dir().join("basic");

    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(template_dir.to_string_lossy().into_owned()),
            ..Default::default()
        },
        no_workspace: true,
        ..Default::default()
    };

    let output_dir = generate(args)?;
    Ok(output_dir.display().to_string())
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Commands::New => new_project().map(|path| println!("Project created at: {path}")),
        Commands::Generate {
            action: GenerateAction::Scaffold { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            scaffold::run(&name, &cwd)
        }
        Commands::Generate {
            action: GenerateAction::Bootstrap,
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            scaffold::regenerate_bootstrap(&cwd).map(|_| println!("✓ bootstrap.rs regenerated."))
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
        let template_dir = templates_dir().join("basic");

        let args = GenerateArgs {
            template_path: TemplatePath {
                path: Some(template_dir.to_string_lossy().into_owned()),
                ..Default::default()
            },
            name: Some(name.to_string()),
            destination: Some(destination.to_path_buf()),
            vcs: Some(Vcs::None),
            no_workspace: true,
            ..Default::default()
        };

        let output_dir = generate(args)?;
        Ok(output_dir)
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
        scaffold::run("Product", &dir).unwrap();

        // Domain
        assert!(dir.join("business/src/domain/product/model.rs").exists());
        assert!(dir.join("business/src/domain/product/errors.rs").exists());
        assert!(
            dir.join("business/src/domain/product/repository.rs")
                .exists()
        );
        assert!(
            dir.join("business/src/domain/product/use_cases.rs")
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
        scaffold::run("OrderItem", &dir).unwrap();

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
        scaffold::run("product", &dir).unwrap();

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
        scaffold::run("Product", &dir).unwrap();

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
        scaffold::run("Product", &dir).unwrap();

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
        scaffold::run("Product", &dir).unwrap();

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
            "pub mod domain {\n  pub mod greeting {\n    pub mod errors;\n    pub mod model;\n    pub mod repository;\n    pub mod use_cases;\n  }\n}\npub mod application {\n  pub mod greeting {\n    pub mod get_greeting;\n  }\n}\n",
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
        scaffold::run("Product", &dir).unwrap();

        let content = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
        // domain block now contains product
        assert!(content.contains("pub mod product {"));
        assert!(content.contains("pub mod errors;"));
        assert!(content.contains("pub mod use_cases;"));
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
        scaffold::run("Product", &dir).unwrap();

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
        scaffold::run("Product", &dir).unwrap();

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
        scaffold::run("Product", &dir).unwrap();

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
}
