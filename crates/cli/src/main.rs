mod commands;
mod generators;
mod patchers;
mod puerto_toml;
mod runtime;
mod scaffold;
mod snippets;
mod validation;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use generators::format::format_after_generation;
use include_dir::{Dir, include_dir};
use std::path::PathBuf;

pub(crate) static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/template");

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "puerto",
    version,
    about = "Puerto — Rust full-stack DDD framework"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new Puerto project
    #[command(name = "nuevo")]
    New {
        /// Project name (skips interactive prompt)
        #[arg(long = "nombre")]
        name: Option<String>,
        /// Include database support (SQLx + Postgres) (skips interactive prompt)
        #[arg(long, conflicts_with = "no_db")]
        db: bool,
        /// Explicitly skip database support without prompting
        #[arg(long = "sin-db", conflicts_with = "db")]
        no_db: bool,
        /// Skip the Saludo demo entity (creates an empty project)
        #[arg(long = "sin-demo")]
        no_demo: bool,
        /// Skip the web frontend (`web/`: React + daisyUI on Bun)
        #[arg(long = "sin-web")]
        no_web: bool,
        /// Domain events (needs --db): every write records an event in an outbox, relayed through
        /// this transport
        #[arg(long = "eventos", value_parser = ["db", "kafka"], conflicts_with = "no_db")]
        events: Option<String>,
        /// Auth: `oidc` verifies an OpenID Connect provider's tokens (Auth0, Keycloak — Keycloak
        /// runs in docker-compose.yml for development); `local` keeps accounts and sessions in the
        /// project's database (needs --db)
        #[arg(long, value_parser = ["oidc", "local"])]
        auth: Option<String>,
        /// Directory where the project will be created (defaults to current directory)
        #[arg(long = "destino")]
        destination: Option<PathBuf>,
    },
    /// Code generators for existing Puerto projects
    #[command(name = "generar")]
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// List entities and use cases defined in puerto.toml
    #[command(name = "listar")]
    List,
    /// Validate puerto.toml (field types, names, entity consistency)
    #[command(name = "validar")]
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
        /// Entity name in PascalCase (e.g. Producto, LineaPedido)
        name: String,
        /// Overwrite an already-generated entity, discarding hand-written changes in its files
        #[arg(long = "forzar")]
        force: bool,
        /// Auth projects: use cases whose routes need no token, comma-separated
        /// (e.g. listar_productos,obtener_producto). Every other route takes `auth: BearerAuth`
        #[arg(long = "publico", value_delimiter = ',')]
        public: Vec<String>,
        /// Auth projects: a role a route requires, <use_case>=<role>, comma-separated
        /// (e.g. crear_producto=admin,borrar_producto=admin) — `#[requiere_rol]` on the handler
        #[arg(long = "rol", value_delimiter = ',')]
        role: Vec<String>,
        /// A unique over several fields, comma-separated (e.g. jugador_id,partido_id): one partial
        /// unique index in Postgres, checked by the in-memory repository too. Repeatable
        #[arg(long = "unico")]
        unique: Vec<String>,
        /// The order of the list, comma-separated, a `-` for descending (e.g. --orden=-puntos,nombre).
        /// Then the newest first
        #[arg(long = "orden", value_delimiter = ',', allow_hyphen_values = true)]
        order: Vec<String>,
        /// Auth projects: fields public routes answer only to a caller with a session,
        /// comma-separated (e.g. email,telefono). Text fields only
        #[arg(long = "privado", value_delimiter = ',')]
        private: Vec<String>,
        /// Fields after -- separator. Primitives: titulo:String precio:i64! nota:opt:String etiquetas:vec:String
        /// Value Objects: nombre:Nombre:String edad:Edad:i64 estado:Estado:enum:Activo/Inactivo apodo:Apodo:opt:String
        /// Shared VO (type inferred from puerto.toml): email:Email
        #[arg(raw = true)]
        fields: Vec<String>,
    },
    /// Add a use case to an existing entity
    #[command(name = "caso-de-uso")]
    UseCase {
        /// Entity name in PascalCase (e.g. Producto)
        entity: String,
        /// Use case action in snake_case (e.g. publicar_producto)
        action: String,
    },
    /// Declare a custom domain event of a CRUD entity and regenerate its events.rs (projects
    /// created with --eventos)
    #[command(name = "evento")]
    Event {
        /// Entity name in PascalCase (e.g. Producto)
        entity: String,
        /// Event type in snake_case, past tense (e.g. producto_publicado)
        name: String,
    },
    /// Add a consumer to the worker: an event handler, its tests and its registry entry
    /// (projects created with --eventos)
    #[command(name = "consumidor")]
    Consumer {
        /// Consumer name in snake_case (e.g. avisar_producto_nuevo)
        name: String,
        /// The event types it handles, comma-separated (e.g. producto_creado,producto_publicado)
        #[arg(long = "tras", value_delimiter = ',', required = true)]
        on: Vec<String>,
    },
    /// Add a mail: its port, its template implementation and its templates (asunto.txt,
    /// cuerpo.html, cuerpo.txt). The first one also adds Mailpit and SMTP_URL / MAIL_FROM to .env
    #[command(name = "correo")]
    Mail {
        /// Mail name in snake_case (e.g. bienvenida, pedido_enviado)
        name: String,
        /// Projects created with --eventos: the worker sends the mail on these event types,
        /// comma-separated (e.g. cliente_creado). Its fields come from the events' snapshots
        #[arg(long = "tras", value_delimiter = ',')]
        on: Vec<String>,
        /// With --tras: the String field of the snapshot that holds the recipient (e.g. email)
        #[arg(long = "para")]
        to: Option<String>,
        /// The values its text shows, after --: nombre:String plazas:i64 plan:opt:String.
        /// Primitives and opt:<type> only; the recipient `para` is always there. None: nombre:String
        #[arg(raw = true)]
        fields: Vec<String>,
    },
    /// Regenerate the wiring (presentacion/cableado, presentacion/api/src/generado/bootstrap.rs)
    /// from puerto.toml
    Bootstrap {
        /// Write the file even when puerto.toml declares entities whose code is not on disk
        #[arg(long = "permitir-faltantes")]
        allow_missing: bool,
    },
    /// Create a new SQLx migration file
    #[command(name = "migracion")]
    Migration {
        /// Migration name in snake_case (e.g. anadir_stock_a_productos)
        name: String,
    },
    /// Write IDE snippet files (.zed/snippets/rust.json, .vscode/puerto.code-snippets)
    Snippets {
        /// Target IDE: zed or vscode (default: both)
        #[arg(long)]
        ide: Option<String>,
    },
    /// Scaffold only the domain layer for a new entity (domain-first workflow)
    #[command(name = "dominio")]
    Domain {
        /// Entity name in PascalCase (e.g. Producto, LineaPedido)
        name: String,
        /// A unique over several fields, comma-separated (e.g. jugador_id,partido_id). Repeatable
        #[arg(long = "unico")]
        unique: Vec<String>,
        /// The order of the list, comma-separated, a `-` for descending (e.g. --orden=-puntos,nombre).
        /// Then the newest first
        #[arg(long = "orden", value_delimiter = ',', allow_hyphen_values = true)]
        order: Vec<String>,
        /// Auth projects: fields public routes answer only to a caller with a session,
        /// comma-separated (e.g. email,telefono). Text fields only
        #[arg(long = "privado", value_delimiter = ',')]
        private: Vec<String>,
        /// Fields after -- separator. Same syntax as `generar scaffold`. They are written to
        /// puerto.toml, so `generar aplicacion/repositorio/presentacion` pick them up.
        #[arg(raw = true)]
        fields: Vec<String>,
    },
    /// Scaffold only the application layer for an existing entity
    #[command(name = "aplicacion")]
    Application {
        /// Entity name in PascalCase (e.g. Producto)
        name: String,
    },
    /// Scaffold only the repository (infrastructure) layer for an existing entity
    #[command(name = "repositorio")]
    Repository {
        /// Entity name in PascalCase (e.g. Producto)
        name: String,
    },
    /// Scaffold only the presentation layer for an existing entity (regenerates bootstrap.rs)
    #[command(name = "presentacion")]
    Presentation {
        /// Entity name in PascalCase (e.g. Producto)
        name: String,
    },
    /// Scaffold only the frontend slice of an existing CRUD entity (web projects; regenerates
    /// web/src/generated/)
    Web {
        /// Entity name in PascalCase (e.g. Producto)
        name: String,
        /// Overwrite the entity's existing frontend files, discarding hand-written changes
        #[arg(long = "forzar")]
        force: bool,
    },
    /// Declare a shared value object in puerto.toml (reusable across entities)
    #[command(name = "objeto-valor")]
    ValueObject {
        /// Value object name in PascalCase (e.g. Email, Dinero)
        name: String,
        /// Inner primitive type: String, i64, bool, f64, Uuid, DateTime
        inner_type: String,
    },
}

// ── Shared CLI plumbing ───────────────────────────────────────────────────────

/// Parse the trailing `-- name:Type …` arguments into validated `Field`s.
///
/// Shared by `generar scaffold` and `generar dominio` so the two cannot drift: parse, resolve
/// shared-VO type inference against puerto.toml, then type-check against the registry.
fn parse_cli_fields(
    cwd: &std::path::Path,
    raw: &[String],
) -> Result<Vec<puerto_toml::Field>, Box<dyn std::error::Error>> {
    let mut parsed: Vec<puerto_toml::Field> = raw
        .iter()
        .map(|s| puerto_toml::parse_field_arg(s))
        .collect::<Result<_, _>>()?;

    if let Ok(config) = puerto_toml::read(cwd) {
        parsed = puerto_toml::apply_shared_vo_inference(parsed, &config.value_object);
    }
    generators::types::validate_fields(&parsed)?;
    validation::validate_unique_field_names(&parsed)?;
    Ok(parsed)
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli.command) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::New {
            name,
            db,
            no_db,
            no_demo,
            no_web,
            events,
            auth,
            destination,
        } => {
            let options = commands::new::NewOptions {
                db,
                no_db,
                no_demo,
                no_web,
                events,
                auth,
            };
            commands::new::new_project(name, options, destination).map(|path| {
                format_after_generation(&path);
                let project_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let web = path.join("web").is_dir();
                println!("├── nucleo/          (shared value objects)");
                println!("├── contextos/       (one crate per context and layer)");
                println!("├── presentacion/    (the binaries and their wiring)");
                if web {
                    println!("├── web/             (React frontend)");
                }
                println!("└── Cargo.toml");
                println!("✓ Project '{project_name}' created.\n");
                println!("Next steps:");
                println!("  cd {project_name}");
                println!("  make run");
                if web {
                    println!("  make web/install && make web/dev   # in another terminal");
                }
                println!();
                println!("API docs available at  http://localhost:8080");
                if web {
                    println!("Frontend available at  http://localhost:5173");
                }
                println!("Add an entity with:    puerto generar scaffold <Name>");
            })
        }
        Commands::Generate {
            action:
                GenerateAction::Scaffold {
                    name,
                    force,
                    public,
                    role,
                    unique,
                    order,
                    private,
                    fields,
                },
        } => {
            let cwd = std::env::current_dir()?;
            let policy = if force {
                crate::generators::conflict::OverwritePolicy::Force
            } else {
                crate::generators::conflict::OverwritePolicy::Ask
            };
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| parse_cli_fields(&cwd, &fields))
                .and_then(|parsed| {
                    let access = scaffold::ScaffoldAccess::from_args(&public, &role)?;
                    let shape = scaffold::EntityShape::from_args(&unique, &order, &private);
                    scaffold::run_scaffold_with_options(
                        &name, &cwd, &parsed, policy, &access, &shape,
                    )
                })
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::UseCase { entity, action },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_use_case(&entity, &action, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Event { entity, name },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_event(&entity, &name, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Consumer { name, on },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_consumer(&name, &on, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action:
                GenerateAction::Mail {
                    name,
                    on,
                    to,
                    fields,
                },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| {
                    scaffold::run_generate_mail_on(&name, &fields, &on, to.as_deref(), &cwd)
                })
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Bootstrap { allow_missing },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_bootstrap_command(&cwd, allow_missing))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Migration { name },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_migration(&name, &cwd))
        }
        Commands::Generate {
            action: GenerateAction::Snippets { ide },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| snippets::run(&cwd, ide.as_deref()))
        }
        Commands::Generate {
            action:
                GenerateAction::Domain {
                    name,
                    unique,
                    order,
                    private,
                    fields,
                },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| parse_cli_fields(&cwd, &fields))
                .and_then(|parsed| {
                    let shape = scaffold::EntityShape::from_args(&unique, &order, &private);
                    scaffold::run_generate_domain_with(&name, &cwd, &parsed, &shape)
                })
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Application { name },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_application(&name, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Repository { name },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_repository(&name, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Presentation { name },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_presentation(&name, &cwd))
                .map(|()| format_after_generation(&cwd))
        }
        Commands::Generate {
            action: GenerateAction::Web { name, force },
        } => {
            let cwd = std::env::current_dir()?;
            let policy = if force {
                crate::generators::conflict::OverwritePolicy::Force
            } else {
                crate::generators::conflict::OverwritePolicy::Ask
            };
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_generate_web(&name, &cwd, policy))
        }
        Commands::Generate {
            action: GenerateAction::ValueObject { name, inner_type },
        } => {
            let cwd = std::env::current_dir()?;
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| scaffold::run_value_object(&name, &inner_type, &cwd))
        }
        Commands::List => {
            let cwd = std::env::current_dir()?;
            commands::list::run_list(&cwd)
        }
        Commands::Validate => {
            let cwd = std::env::current_dir()?;
            commands::validate::run_validate(&cwd)
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "puerto", &mut std::io::stdout());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
