mod commands;
mod generators;
mod patchers;
mod puerto_toml;
mod scaffold;
mod snippets;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use include_dir::{Dir, include_dir};
use std::path::PathBuf;

pub(crate) static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/template");

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "puerto", about = "Puerto — Rust full-stack DDD framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new Puerto project
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
        /// Skip the Greeting demo entity (creates an empty project)
        #[arg(long)]
        no_demo: bool,
        /// Directory where the project will be created (defaults to current directory)
        #[arg(long)]
        destination: Option<PathBuf>,
    },
    /// Code generators for existing Puerto projects
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// List entities and use cases defined in puerto.toml
    List,
    /// Validate puerto.toml (field types, names, entity consistency)
    Validate,
    /// Print shell completion script
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell)
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Scaffold all DDD layers for a new entity (db and CRUD inferred from puerto.toml)
    Scaffold {
        /// Entity name in PascalCase (e.g. Product, OrderItem)
        name: String,
        /// Fields after -- separator. Primitives: title:String price:i64! desc:opt:String tags:vec:String
        /// Value Objects: name:Name:String age:Age:i64 status:Status:enum:Active/Inactive mid:Mid:opt:String
        /// Shared VO (type inferred from puerto.toml): email:Email
        #[arg(raw = true)]
        fields: Vec<String>,
    },
    /// Add a use case to an existing entity
    UseCase {
        /// Entity name in PascalCase (e.g. Product)
        entity: String,
        /// Use case action in snake_case (e.g. delete_product)
        action: String,
    },
    /// Regenerate presentation/src/generated/bootstrap.rs from puerto.toml
    Bootstrap,
    /// Create a new SQLx migration file
    Migration {
        /// Migration name in snake_case (e.g. add_products_table)
        name: String,
    },
    /// Write IDE snippet files (.zed/snippets/rust.json, .vscode/puerto.code-snippets)
    Snippets {
        /// Target IDE: zed or vscode (default: both)
        #[arg(long)]
        ide: Option<String>,
    },
    /// Scaffold only the domain layer for a new entity (domain-first workflow)
    Domain {
        /// Entity name in PascalCase (e.g. Product, OrderItem)
        name: String,
    },
    /// Scaffold only the application layer for an existing entity
    Application {
        /// Entity name in PascalCase (e.g. Product)
        name: String,
    },
    /// Scaffold only the repository (infrastructure) layer for an existing entity
    Repository {
        /// Entity name in PascalCase (e.g. Product)
        name: String,
    },
    /// Scaffold only the presentation layer for an existing entity (regenerates bootstrap.rs)
    Presentation {
        /// Entity name in PascalCase (e.g. Product)
        name: String,
    },
    /// Declare a shared value object in puerto.toml (reusable across entities)
    ValueObject {
        /// Value object name in PascalCase (e.g. Email, Money)
        name: String,
        /// Inner primitive type: String, i64, bool, f64, Uuid, DateTime
        inner_type: String,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Commands::New {
            name,
            db,
            no_db,
            no_demo,
            destination,
        } => commands::new::new_project(name, db, no_db, no_demo, destination).map(|path| {
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
            println!("Add an entity with:    puerto generate scaffold <Name>");
        }),
        Commands::Generate {
            action: GenerateAction::Scaffold { name, fields },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            let mut parsed_fields: Vec<crate::puerto_toml::Field> = match fields
                .iter()
                .map(|s| crate::puerto_toml::parse_field_arg(s))
                .collect::<Result<_, _>>()
            {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            };
            if let Ok(config) = crate::puerto_toml::read(&cwd) {
                parsed_fields = crate::puerto_toml::apply_shared_vo_inference(
                    parsed_fields,
                    &config.value_object,
                );
            }
            if let Err(e) = crate::generators::types::validate_fields(&parsed_fields) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_scaffold(&name, &cwd, None, &parsed_fields))
        }
        Commands::Generate {
            action: GenerateAction::UseCase { entity, action },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_use_case(&entity, &action, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::Bootstrap,
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd).and_then(|_| {
                scaffold::regenerate_bootstrap(&cwd)
                    .map(|_| println!("✓ bootstrap.rs regenerated."))
            })
        }
        Commands::Generate {
            action: GenerateAction::Migration { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_migration(&name, &cwd, None, None))
        }
        Commands::Generate {
            action: GenerateAction::Snippets { ide },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| snippets::run(&cwd, ide.as_deref()))
        }
        Commands::Generate {
            action: GenerateAction::Domain { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_domain(&name, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::Application { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_application(&name, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::Repository { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_repository(&name, &cwd, None))
        }
        Commands::Generate {
            action: GenerateAction::Presentation { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_presentation(&name, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::ValueObject { name, inner_type },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            let expanded = match inner_type.as_str() {
                "DateTime" => "DateTime<Utc>".to_string(),
                t => t.to_string(),
            };
            commands::list::require_puerto_project(&cwd).and_then(|_| {
                crate::puerto_toml::add_value_object(&cwd, &name, &expanded)
                    .map(|_| println!("✓ Shared value object '{name}' added to puerto.toml"))
            })
        }
        Commands::List => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::run_list(&cwd)
        }
        Commands::Validate => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::validate::run_validate(&cwd)
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "puerto", &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
