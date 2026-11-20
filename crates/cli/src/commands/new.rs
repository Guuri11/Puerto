use cargo_generate::{GenerateArgs, TemplatePath, generate};
use std::path::PathBuf;

pub fn extract_template() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    crate::TEMPLATE_DIR.extract(tmp.path())?;
    // The runtime crates' version follows this CLI's, so it is rendered here, not in the template.
    let manifest = tmp.path().join("Cargo.toml.liquid");
    std::fs::write(
        &manifest,
        crate::runtime::render_manifest(
            &std::fs::read_to_string(&manifest)?,
            env!("CARGO_PKG_VERSION"),
        ),
    )?;
    Ok(tmp)
}

pub fn generate_new_project(
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
        quiet: true,
        ..Default::default()
    };

    let output = generate(args)?;
    Ok(output)
}

/// A project name becomes the directory, the binary and the Postgres database name. cargo-generate
/// silently rewrites anything else (`"Bad Name"` → `bad-name`), so the project would not be called
/// what the user typed.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let valid = matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        return Err(format!(
            "nombre de proyecto inválido '{name}'. Usa minúsculas, dígitos, '-' y '_', y empieza \
             por una letra (p. ej. 'mi-app')"
        ));
    }
    Ok(())
}

/// What `puerto nuevo` was asked for on the command line; what is left out is prompted for.
#[derive(Debug, Default)]
pub struct NewOptions {
    pub db: bool,
    pub no_db: bool,
    pub no_demo: bool,
    pub no_web: bool,
    /// `--eventos <transport>`: `db` or `kafka`.
    pub events: Option<String>,
    /// `--auth <mode>`: `oidc` or `local` (ADR-0021).
    pub auth: Option<String>,
    /// `--portada`: a public portada at `/` and the app under `app_path` (ADR-0028).
    pub portada: bool,
    /// `--sin-portada`, without asking.
    pub no_portada: bool,
    /// `--ruta-app <ruta>`: where the app goes under the portada (`/panel` by default); implies it.
    pub app_path: Option<String>,
}

pub fn new_project(
    name: Option<String>,
    options: NewOptions,
    destination: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let NewOptions {
        db,
        no_db,
        no_demo,
        no_web,
        events,
        auth,
        portada,
        no_portada,
        app_path,
    } = options;
    let name = match name {
        Some(name) => {
            validate_project_name(&name)?;
            name
        }
        // Asked here, not by cargo-generate: its prompt is in English and accepts names we reject.
        None => dialoguer::Input::<String>::new()
            .with_prompt("Nombre del proyecto")
            .validate_with(|n: &String| validate_project_name(n))
            .interact_text()?,
    };
    if let Some(transport) = &events {
        crate::validation::validate_events_transport(transport)?;
    }
    if let Some(mode) = &auth {
        crate::generators::auth::check_mode(mode)?;
    }

    let resolved_db = if db {
        true
    } else if no_db {
        false
    } else if dialoguer::console::user_attended() {
        dialoguer::Confirm::new()
            .with_prompt("¿Incluir base de datos (SQLx + Postgres)?")
            .default(false)
            .interact()?
    } else {
        false
    };

    // The outbox only works inside a database transaction (ADR-0018 § 1).
    if events.is_some() && !resolved_db {
        return Err("--eventos necesita una base de datos: añade --db".into());
    }
    let resolved_events = if events.is_some() || !resolved_db {
        events
    } else if dialoguer::console::user_attended() {
        let transports = ["ninguno", "db", "kafka"];
        let choice = dialoguer::Select::new()
            .with_prompt("¿Eventos de dominio?")
            .items(&transports)
            .default(0)
            .interact()?;
        (choice > 0).then(|| transports[choice].to_string())
    } else {
        None
    };

    let resolved_auth = if auth.is_some() {
        auth
    } else if dialoguer::console::user_attended() {
        let mut modes = vec![(
            "oidc",
            "oidc (Auth0, Keycloak, cualquier proveedor OpenID Connect)",
        )];
        // Local accounts live in the database.
        if resolved_db {
            modes.push(("local", "local (cuentas y sesiones en esta base de datos)"));
        }
        let labels: Vec<&str> = std::iter::once("ninguna")
            .chain(modes.iter().map(|(_, label)| *label))
            .collect();
        let choice = dialoguer::Select::new()
            .with_prompt("¿Auth?")
            .items(&labels)
            .default(0)
            .interact()?;
        (choice > 0).then(|| modes[choice - 1].0.to_string())
    } else {
        None
    };

    let resolved_no_demo = if no_demo {
        true
    } else if dialoguer::console::user_attended() {
        !dialoguer::Confirm::new()
            .with_prompt("¿Incluir la entidad de demo Saludo?")
            .default(true)
            .interact()?
    } else {
        false
    };

    let resolved_no_web = if no_web {
        true
    } else if dialoguer::console::user_attended() {
        !dialoguer::Confirm::new()
            .with_prompt("¿Incluir el frontend web (React + daisyUI, necesita Bun)?")
            .default(true)
            .interact()?
    } else {
        false
    };

    if let Some(mode) = &resolved_auth {
        crate::generators::auth::check_new_project(mode, resolved_db)?;
    }

    // Where the app goes when the project is a SaaS with a public portada (ADR-0028).
    let resolved_portada = if portada || app_path.is_some() {
        if resolved_no_web {
            return Err("--portada necesita el frontend web: quita --sin-web".into());
        }
        Some(app_path.unwrap_or_else(|| crate::puerto_toml::DEFAULT_APP_PATH.to_string()))
    } else if no_portada || resolved_no_web {
        None
    } else if dialoguer::console::user_attended()
        && dialoguer::Confirm::new()
            .with_prompt("¿Es un SaaS con portada pública?")
            .default(false)
            .interact()?
    {
        Some(
            dialoguer::Input::<String>::new()
                .with_prompt("¿En qué ruta va la app?")
                .default(crate::puerto_toml::DEFAULT_APP_PATH.to_string())
                .validate_with(|ruta: &String| crate::validation::validate_app_path(ruta))
                .interact_text()?,
        )
    } else {
        None
    };
    if let Some(path) = &resolved_portada {
        crate::validation::validate_app_path(path)?;
    }

    eprintln!("Construyendo el esqueleto del proyecto...");

    let output = generate_new_project(Some(name), destination)?;

    crate::generators::project::apply_base_env(&output)?;

    if resolved_db {
        crate::generators::project::apply_db_to_new_project(&output)?;
    }

    if let Some(transport) = &resolved_events {
        crate::generators::project::apply_events_to_new_project(&output, transport)?;
    }

    if resolved_no_demo {
        crate::generators::project::apply_no_demo(&output)?;
    }

    if resolved_no_web {
        crate::generators::project::apply_no_web(&output)?;
    } else {
        crate::generators::project::keep_web_blocks(&output)?;
    }

    if let Some(path) = &resolved_portada {
        crate::generators::project::apply_portada(&output, path)?;
    }

    // Last: the demo strip rewrites the `lib.rs` and `api.rs` files auth declares its modules in,
    // and auth writes the frontend's files only when `web/` stays.
    if let Some(mode) = &resolved_auth {
        crate::generators::auth::apply_auth_to_new_project(&output, mode)?;
    }

    // After auth: the script's helpers depend on it (27.21).
    crate::generators::project::write_seed_script(&output, resolved_auth.as_deref())?;
    crate::snippets::apply(&output, None)?;

    Ok(output)
}
