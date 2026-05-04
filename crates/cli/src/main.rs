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
            action: GenerateAction::Scaffold { name },
        } => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_scaffold(&name, &cwd, None))
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
        Commands::List => {
            let cwd = std::env::current_dir().expect("cannot read current directory");
            commands::list::run_list(&cwd)
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
