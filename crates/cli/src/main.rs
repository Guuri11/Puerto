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
    about = "Puerto: framework full-stack DDD en Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Crea un proyecto Puerto nuevo
    #[command(name = "nuevo")]
    New {
        /// Nombre del proyecto (no lo pregunta)
        #[arg(value_name = "NOMBRE", long = "nombre")]
        name: Option<String>,
        /// Con base de datos (SQLx + Postgres) (no lo pregunta)
        #[arg(long, conflicts_with = "no_db")]
        db: bool,
        /// Sin base de datos, sin preguntar
        #[arg(long = "sin-db", conflicts_with = "db")]
        no_db: bool,
        /// Sin la entidad de demo Saludo (un proyecto vacío)
        #[arg(long = "sin-demo")]
        no_demo: bool,
        /// Sin el frontend web (`web/`: React + daisyUI sobre Bun)
        #[arg(long = "sin-web")]
        no_web: bool,
        /// Eventos de dominio (necesita --db): cada escritura registra un evento en un outbox, que
        /// sale por este transporte
        #[arg(value_name = "TRANSPORTE", long = "eventos", value_parser = ["db", "kafka"], conflicts_with = "no_db")]
        events: Option<String>,
        /// Auth: `oidc` comprueba los tokens de un proveedor OpenID Connect (Auth0, Keycloak;
        /// Keycloak corre en docker-compose.yml para desarrollo); `local` guarda las cuentas y las
        /// sesiones en la base de datos del proyecto (necesita --db)
        #[arg(value_name = "MODO", long, value_parser = ["oidc", "local"])]
        auth: Option<String>,
        /// Un SaaS con portada pública: la portada en `/` y la app debajo de --ruta-app (no lo
        /// pregunta)
        #[arg(long, conflicts_with_all = ["no_portada", "no_web"])]
        portada: bool,
        /// Sin portada: la app en `/`, sin preguntar
        #[arg(long = "sin-portada", conflicts_with = "portada")]
        no_portada: bool,
        /// Dónde va la app bajo la portada (por defecto /panel); implica --portada
        #[arg(value_name = "RUTA", long = "ruta-app", conflicts_with_all = ["no_portada", "no_web"])]
        app_path: Option<String>,
        /// Directorio donde se crea el proyecto (por defecto, el actual)
        #[arg(value_name = "DIRECTORIO", long = "destino")]
        destination: Option<PathBuf>,
    },
    /// Generadores de código para un proyecto Puerto que ya existe
    #[command(name = "generar")]
    Generate {
        #[command(subcommand)]
        action: GenerateAction,
    },
    /// Lista las entidades y los casos de uso de puerto.toml
    #[command(name = "listar")]
    List,
    /// Valida puerto.toml (tipos de los campos, nombres, coherencia con el código)
    #[command(name = "validar")]
    Validate,
    /// Imprime el script de autocompletado de la shell
    Completions {
        /// La shell (bash, zsh, fish, powershell)
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Todas las capas DDD de una entidad nueva (la base de datos sale de puerto.toml)
    Scaffold {
        /// Nombre de la entidad en PascalCase (p. ej. Producto, LineaPedido)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// Sobrescribe una entidad ya generada, y pierde lo cambiado a mano en sus ficheros
        #[arg(long = "forzar")]
        force: bool,
        /// Proyectos con auth: los casos de uso cuyas rutas no piden token, separados por comas
        /// (p. ej. listar_productos,obtener_producto). Las demás rutas reciben `auth: BearerAuth`
        #[arg(value_name = "CASOS_DE_USO", long = "publico", value_delimiter = ',')]
        public: Vec<String>,
        /// Proyectos con auth: el rol que exige una ruta, caso_de_uso=rol, separados por comas
        /// (p. ej. crear_producto=admin,borrar_producto=admin): `#[requiere_rol]` en el handler
        #[arg(value_name = "CASO=ROL", long = "rol", value_delimiter = ',')]
        role: Vec<String>,
        /// Un único sobre varios campos, separados por comas (p. ej. jugador_id,partido_id): un
        /// índice único parcial en Postgres, que también comprueba el repositorio en memoria. Se
        /// puede repetir
        #[arg(value_name = "CAMPOS", long = "unico")]
        unique: Vec<String>,
        /// El orden de la lista, separado por comas, con `-` para descendente (p. ej.
        /// --orden=-puntos,nombre). Después, lo más nuevo primero
        #[arg(
            value_name = "CAMPOS",
            long = "orden",
            value_delimiter = ',',
            allow_hyphen_values = true
        )]
        order: Vec<String>,
        /// Proyectos con auth: los campos que las rutas públicas solo dan a quien tiene sesión,
        /// separados por comas (p. ej. email,telefono). Solo campos de texto
        #[arg(value_name = "CAMPOS", long = "privado", value_delimiter = ',')]
        private: Vec<String>,
        /// Los campos, tras --. Primitivos: titulo:String precio:i64! nota:opt:String etiquetas:vec:String
        /// Objetos valor: nombre:Nombre:String edad:Edad:i64 estado:Estado:enum:Activo/Inactivo apodo:Apodo:opt:String
        /// Objeto valor compartido (el tipo sale de puerto.toml): email:Email
        #[arg(value_name = "CAMPOS", raw = true)]
        fields: Vec<String>,
    },
    /// Añade un caso de uso a una entidad que ya existe
    #[command(name = "caso-de-uso")]
    UseCase {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "ENTIDAD")]
        entity: String,
        /// La acción del caso de uso en snake_case (p. ej. publicar_producto)
        #[arg(value_name = "ACCION")]
        action: String,
    },
    /// Declara un evento de dominio propio de una entidad CRUD y regenera su eventos.rs
    /// (proyectos creados con --eventos)
    #[command(name = "evento")]
    Event {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "ENTIDAD")]
        entity: String,
        /// El tipo de evento en snake_case y en participio (p. ej. producto_publicado)
        #[arg(value_name = "NOMBRE")]
        name: String,
    },
    /// Añade un consumidor al worker: un manejador de eventos, sus tests y su entrada en el
    /// registro (proyectos creados con --eventos)
    #[command(name = "consumidor")]
    Consumer {
        /// Nombre del consumidor en snake_case (p. ej. avisar_producto_nuevo)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// Los tipos de evento que maneja, separados por comas (p. ej. producto_creado,producto_publicado)
        #[arg(
            value_name = "EVENTOS",
            long = "tras",
            value_delimiter = ',',
            required = true
        )]
        on: Vec<String>,
    },
    /// Añade un correo: su puerto, su implementación con plantillas y las plantillas (asunto.txt,
    /// cuerpo.html, cuerpo.txt). El primero añade además Mailpit, y SMTP_URL / MAIL_FROM a .env
    #[command(name = "correo")]
    Mail {
        /// Nombre del correo en snake_case (p. ej. bienvenida, pedido_enviado)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// Proyectos creados con --eventos: el worker envía el correo tras estos tipos de evento,
        /// separados por comas (p. ej. cliente_creado). Sus campos salen de los snapshots
        #[arg(value_name = "EVENTOS", long = "tras", value_delimiter = ',')]
        on: Vec<String>,
        /// Con --tras: el campo String del snapshot que guarda el destinatario (p. ej. email)
        #[arg(value_name = "CAMPO", long = "para")]
        to: Option<String>,
        /// Los valores que muestra su texto, tras --: nombre:String plazas:i64 plan:opt:String.
        /// Solo primitivos y opt:<tipo>; el destinatario `para` siempre está. Sin campos: nombre:String
        #[arg(value_name = "CAMPOS", raw = true)]
        fields: Vec<String>,
    },
    /// Regenera el cableado (presentacion/cableado, presentacion/api/src/generado/bootstrap.rs)
    /// desde puerto.toml
    Bootstrap {
        /// Lo escribe aunque puerto.toml declare entidades cuyo código no está en disco
        #[arg(long = "permitir-faltantes")]
        allow_missing: bool,
    },
    /// Crea una migración SQLx nueva
    #[command(name = "migracion")]
    Migration {
        /// Nombre de la migración en snake_case (p. ej. anadir_stock_a_productos)
        #[arg(value_name = "NOMBRE")]
        name: String,
    },
    /// Escribe los snippets del IDE (.zed/snippets/rust.json, .vscode/puerto.code-snippets)
    Snippets {
        /// El IDE: zed o vscode (por defecto, los dos)
        #[arg(value_name = "IDE", long)]
        ide: Option<String>,
    },
    /// Solo la capa de dominio de una entidad nueva (empezar por el dominio)
    #[command(name = "dominio")]
    Domain {
        /// Nombre de la entidad en PascalCase (p. ej. Producto, LineaPedido)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// Un único sobre varios campos, separados por comas (p. ej. jugador_id,partido_id). Se
        /// puede repetir
        #[arg(value_name = "CAMPOS", long = "unico")]
        unique: Vec<String>,
        /// El orden de la lista, separado por comas, con `-` para descendente (p. ej.
        /// --orden=-puntos,nombre). Después, lo más nuevo primero
        #[arg(
            value_name = "CAMPOS",
            long = "orden",
            value_delimiter = ',',
            allow_hyphen_values = true
        )]
        order: Vec<String>,
        /// Proyectos con auth: los campos que las rutas públicas solo dan a quien tiene sesión,
        /// separados por comas (p. ej. email,telefono). Solo campos de texto
        #[arg(value_name = "CAMPOS", long = "privado", value_delimiter = ',')]
        private: Vec<String>,
        /// Los campos, tras --. La misma sintaxis que `generar scaffold`. Se escriben en
        /// puerto.toml, así que `generar aplicacion/repositorio/presentacion` los ven.
        #[arg(value_name = "CAMPOS", raw = true)]
        fields: Vec<String>,
    },
    /// Solo la capa de aplicación de una entidad que ya existe
    #[command(name = "aplicacion")]
    Application {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "NOMBRE")]
        name: String,
    },
    /// Solo la capa del repositorio (infraestructura) de una entidad que ya existe
    #[command(name = "repositorio")]
    Repository {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "NOMBRE")]
        name: String,
    },
    /// Solo la capa de presentación de una entidad que ya existe (regenera bootstrap.rs)
    #[command(name = "presentacion")]
    Presentation {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "NOMBRE")]
        name: String,
    },
    /// Solo las pantallas web de una entidad CRUD que ya existe (proyectos con web; regenera
    /// web/src/generado/)
    Web {
        /// Nombre de la entidad en PascalCase (p. ej. Producto)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// Sobrescribe los ficheros web de la entidad, y pierde lo cambiado a mano
        #[arg(long = "forzar")]
        force: bool,
    },
    /// Instala un bloque del escaparate (https://puerto-rs.vercel.app/bloques): otra variante de una
    /// pantalla, una sección o un tema de web/. Sin argumento, lista el catálogo
    #[command(name = "web-bloque")]
    WebBlock {
        /// El bloque, `<hueco>/<variante>` (p. ej. marco/superior)
        #[arg(value_name = "BLOQUE")]
        id: Option<String>,
        /// Sobrescribe el fichero del hueco, y pierde lo cambiado a mano
        #[arg(long = "forzar")]
        force: bool,
    },
    /// Declara un objeto valor compartido en puerto.toml (lo pueden usar varias entidades)
    #[command(name = "objeto-valor")]
    ValueObject {
        /// Nombre del objeto valor en PascalCase (p. ej. Email, Dinero)
        #[arg(value_name = "NOMBRE")]
        name: String,
        /// El tipo primitivo interno: String, i64, bool, f64, Uuid, DateTime
        #[arg(value_name = "TIPO")]
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

/// clap's own text, the fixed pieces of its help and its errors, in the order they are replaced: a
/// phrase that contains another goes first.
const CLAP_EN_CASTELLANO: &[(&str, &str)] = &[
    ("Usage:", "Uso:"),
    ("Commands:", "Comandos:"),
    ("Arguments:", "Argumentos:"),
    ("Options:", "Opciones:"),
    ("[OPTIONS]", "[OPCIONES]"),
    ("<COMMAND>", "<COMANDO>"),
    ("[possible values: ", "[valores posibles: "),
    ("[subcommands: ", "[subcomandos: "),
    (
        "Print this message or the help of the given subcommand(s)",
        "Muestra esta ayuda o la de los subcomandos dados",
    ),
    (
        "Print help (see more with '--help')",
        "Muestra la ayuda (más con '--help')",
    ),
    (
        "Print help (see a summary with '-h')",
        "Muestra la ayuda (un resumen con '-h')",
    ),
    ("Print help", "Muestra la ayuda"),
    ("Print version", "Muestra la versión"),
    (
        "' cannot be used multiple times",
        "' no se puede usar varias veces",
    ),
    ("' cannot be used with", "' no se puede usar con"),
    ("the argument '", "el argumento '"),
    ("the subcommand '", "el subcomando '"),
    (
        " one or more of the other specified arguments",
        " uno o más de los otros argumentos",
    ),
    (
        "equal sign is needed when assigning values to '",
        "hace falta un signo igual para dar valores a '",
    ),
    ("a value is required for '", "hace falta un valor para '"),
    ("' but none was supplied", "', y no se ha dado ninguno"),
    ("invalid value '", "valor inválido '"),
    ("unexpected value '", "valor inesperado '"),
    ("' found; no more were expected", "'; no se esperaban más"),
    ("unexpected argument '", "argumento inesperado '"),
    ("unrecognized subcommand '", "subcomando desconocido '"),
    ("' for '", "' para '"),
    ("' found", "'"),
    (
        "the following required arguments were not provided:",
        "faltan estos argumentos obligatorios:",
    ),
    (
        "' requires a subcommand but one was not provided",
        "' necesita un subcomando, y no se ha dado ninguno",
    ),
    (
        " a similar argument exists: ",
        " hay un argumento parecido: ",
    ),
    (
        " a similar subcommand exists: ",
        " hay un subcomando parecido: ",
    ),
    (" a similar value exists: ", " hay un valor parecido: "),
    (
        " some similar arguments exist: ",
        " hay argumentos parecidos: ",
    ),
    (
        " some similar subcommands exist: ",
        " hay subcomandos parecidos: ",
    ),
    (" some similar values exist: ", " hay valores parecidos: "),
    ("to pass '", "para pasar '"),
    ("' as a value, use '", "' como valor, usa '"),
    ("tip:", "consejo:"),
    (
        "For more information, try '",
        "Para más información, prueba '",
    ),
];

/// `text` with clap's fixed English pieces replaced. Anything it does not know stays as it was.
fn clap_en_castellano(text: &str) -> String {
    CLAP_EN_CASTELLANO
        .iter()
        .fold(text.to_string(), |text, (en, es)| text.replace(en, es))
}

/// What `Error::exit` does — help and version to stdout, errors to stderr, colours only on a
/// terminal — with the text in Spanish.
fn exit_with(e: clap::Error) -> ! {
    use std::io::IsTerminal;
    let to_stderr = e.use_stderr();
    let terminal = if to_stderr {
        std::io::stderr().is_terminal()
    } else {
        std::io::stdout().is_terminal()
    };
    let rendered = e.render();
    let text = if terminal {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    };
    let text = clap_en_castellano(&text);
    if to_stderr {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
    std::process::exit(e.exit_code());
}

fn main() {
    let cli = Cli::try_parse().unwrap_or_else(|e| exit_with(e));
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
            portada,
            no_portada,
            app_path,
            destination,
        } => {
            let options = commands::new::NewOptions {
                db,
                no_db,
                no_demo,
                no_web,
                events,
                auth,
                portada,
                no_portada,
                app_path,
            };
            commands::new::new_project(name, options, destination).map(|path| {
                format_after_generation(&path);
                let project_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let web = path.join("web").is_dir();
                println!("├── nucleo/          (objetos valor compartidos)");
                println!("├── contextos/       (un crate por contexto y capa)");
                println!("├── presentacion/    (los binarios y su cableado)");
                if web {
                    println!("├── web/             (el frontend React)");
                }
                println!("└── Cargo.toml");
                println!("✓ Proyecto '{project_name}' creado.\n");
                println!("Siguientes pasos:");
                println!("  cd {project_name}");
                println!("  make run");
                if web {
                    println!("  make web/install && make web/dev   # en otra terminal");
                }
                println!();
                println!("Docs de la API en:     http://localhost:8080");
                if web {
                    println!("Frontend en:           http://localhost:5173");
                    let config = crate::puerto_toml::read(&path).ok();
                    if let Some(project) = config.map(|c| c.project).filter(|p| p.portada) {
                        println!(
                            "La app, bajo la portada: http://localhost:5173{}",
                            project.app_path()
                        );
                    }
                }
                println!("Añade una entidad con: puerto generar scaffold <Nombre>");
                use crate::generators::ports;
                for warning in ports::taken(&path, ports::PROJECT_PORTS) {
                    eprintln!("{warning}");
                }
                if web && ports::is_taken(ports::WEB_PORT) {
                    eprintln!(
                        "aviso: el puerto {} está ocupado; `make web/dev` usará el siguiente libre",
                        ports::WEB_PORT
                    );
                }
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
            action: GenerateAction::WebBlock { id: None, .. },
        } => {
            println!("Bloques de web/ (puerto generar web-bloque <hueco>/<variante>):\n");
            print!("{}", crate::generators::blocks::catalog_text());
            println!("\nCómo se ve cada uno: https://puerto-rs.vercel.app/bloques");
            Ok(())
        }
        Commands::Generate {
            action:
                GenerateAction::WebBlock {
                    id: Some(id),
                    force,
                },
        } => {
            let cwd = std::env::current_dir()?;
            let policy = if force {
                crate::generators::conflict::OverwritePolicy::Force
            } else {
                crate::generators::conflict::OverwritePolicy::Ask
            };
            commands::list::require_puerto_project(&cwd)
                .and_then(|_| crate::generators::blocks::run_install(&id, &cwd, policy))
                .map(|installed| {
                    for path in &installed.files {
                        if installed.written.contains(path) {
                            println!("✓ {}", path.display());
                        } else {
                            println!("= {}  (ya lo tenía)", path.display());
                        }
                    }
                })
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
