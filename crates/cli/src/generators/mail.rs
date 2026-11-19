//! `puerto generar correo <name> [-- field:Type …]` (ADR-0020): a mail's port in the domain, its
//! template implementation in infrastructure and its templates — written once, the user's from
//! then on. The first mail of a project also brings `puerto-correo`, Mailpit and `SMTP_URL`.

use crate::generators::layout::{self, Layer};
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::generators::effective_fields;
use crate::generators::naming::{to_pascal_case, write_file};
use crate::generators::project::add_workspace_dependency;
use crate::generators::render::render;
use crate::generators::types::resolve_type;
use crate::generators::worker::{
    DeclaredEvent, declared_events, has_worker, regenerate_registry, subscribed, with_module,
};
use crate::puerto_toml::{Consumer, Field, Mail, PuertoConfig, parse_field_arg};

/// The SMTP server development runs against (ADR-0020 § 5): SMTP on 1025, UI and API on 8025
/// (`SMTP_HOST_PORT`, `MAILPIT_UI_PORT` in `.env`).
pub const MAILPIT_IMAGE: &str = "axllent/mailpit:v1.31.1";

/// A mail field as the templates see it.
#[derive(Debug, Clone, PartialEq, Serialize)]
struct MailField {
    name: String,
    rust_type: String,
    /// A valid value for the generated tests: `Some(…)` for an `Option`, so the template's
    /// optional branch renders too.
    sample: String,
}

/// Checks a field a mail can carry: a primitive or its `Option` — what a template prints.
fn mail_field(field: &Field) -> Result<MailField, String> {
    let name = &field.name;
    if name == "para" {
        return Err(
            "el campo 'para' es el destinatario que ya tiene todo correo: elige otro nombre"
                .to_string(),
        );
    }
    if field.value_object.is_some() {
        return Err(format!(
            "campo '{name}': un correo lleva primitivos, no un objeto valor: usa \
             {name}:<tipo>"
        ));
    }
    if field.unique {
        return Err(format!(
            "campo '{name}': `!` (único) es una restricción de la base de datos, y en un correo no \
             significa nada"
        ));
    }
    let mapping = resolve_type(&field.field_type).map_err(|e| format!("campo '{name}': {e}"))?;
    if mapping.rust_type.starts_with("Vec<") || mapping.rust_type.starts_with("HashMap<") {
        return Err(format!(
            "campo '{name}': {} no es un valor que imprima una plantilla de correo: usa String, \
             i64, bool, f64, Uuid, DateTime u opt:<tipo>",
            mapping.rust_type
        ));
    }
    let sample = match mapping
        .rust_type
        .strip_prefix("Option<")
        .and_then(|t| t.strip_suffix('>'))
    {
        Some(inner) => format!("Some({})", resolve_type(inner)?.default_expr),
        None => mapping.default_expr.to_string(),
    };
    Ok(MailField {
        name: name.clone(),
        rust_type: mapping.rust_type.to_string(),
        sample,
    })
}

/// The fields of a mail from its CLI arguments; none given → `nombre: String`.
fn mail_fields(args: &[String]) -> Result<Vec<MailField>, String> {
    let fields: Vec<Field> = if args.is_empty() {
        vec![Field {
            name: "nombre".into(),
            field_type: "String".into(),
            ..Field::default()
        }]
    } else {
        args.iter()
            .map(|arg| parse_field_arg(arg))
            .collect::<Result<_, _>>()?
    };
    let mut out: Vec<MailField> = vec![];
    for field in &fields {
        if out.iter().any(|f| f.name == field.name) {
            return Err(format!("el campo '{}' está repetido", field.name));
        }
        out.push(mail_field(field)?);
    }
    Ok(out)
}

/// What naming the fields' types needs, sorted and once each.
fn imports(fields: &[MailField]) -> Vec<String> {
    let mut imports: Vec<String> = fields
        .iter()
        .filter_map(|f| resolve_type(&f.rust_type).ok()?.needs_import)
        .map(str::to_string)
        .collect();
    imports.sort();
    imports.dedup();
    imports
}

/// `order_shipped` → `Order shipped`.
fn humanize(snake: &str) -> String {
    let words = snake.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => words,
    }
}

fn is_optional(field: &MailField) -> bool {
    field.rust_type.starts_with("Option<")
}

/// The starter templates: the mail's title, then every field once. They are the user's to rewrite;
/// an `Option` prints only when it is set.
fn starter_templates(name: &str, fields: &[MailField]) -> (String, String, String) {
    let title = humanize(name);
    let subject = match fields.iter().find(|f| f.rust_type == "String") {
        Some(f) => format!("{title}, {{{{ {} }}}}\n", f.name),
        None => format!("{title}\n"),
    };

    let mut items = String::new();
    let mut lines = String::new();
    for f in fields {
        let label = humanize(&f.name);
        let value = format!("{{{{ {} }}}}", f.name);
        if is_optional(f) {
            items.push_str(&format!(
                "      {{% if {0} is not none %}}<li>{label}: {value}</li>{{% endif %}}\n",
                f.name
            ));
            lines.push_str(&format!(
                "{{% if {0} is not none %}}{label}: {value}\n{{% endif %}}",
                f.name
            ));
        } else {
            items.push_str(&format!("      <li>{label}: {value}</li>\n"));
            lines.push_str(&format!("{label}: {value}\n"));
        }
    }
    let html = format!(
        "<!doctype html>\n<html>\n  <body>\n    <h1>{title}</h1>\n    <ul>\n{items}    </ul>\n  </body>\n</html>\n"
    );
    let text = format!("{title}\n\n{lines}");
    (subject, html, text)
}

fn render_files(
    name: &str,
    fields: &[MailField],
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut ctx = tera::Context::new();
    ctx.insert("name", name);
    ctx.insert("pascal", &to_pascal_case(name));
    ctx.insert("fields", fields);
    ctx.insert("imports", &imports(fields));
    Ok((
        render("mail/port.tera", &ctx)?,
        render("mail/template_mailer.tera", &ctx)?,
    ))
}

/// The `correo` context's crates (ADR-0023): `dominio_correo` holds the mails' ports,
/// `infra_correo` their template mailers on `puerto-correo`. Their `lib.rs` declares every mail on
/// disk.
fn write_mail_crates(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let c = layout::MAIL_CONTEXT;
    for (layer, template) in [
        (Layer::Domain, "contexto/correo_dominio_cargo.tera"),
        (Layer::Infrastructure, "contexto/correo_infra_cargo.tera"),
    ] {
        let manifest = base.join(layout::crate_manifest(c, layer));
        if !manifest.exists() {
            write_file(&manifest, &render(template, &tera::Context::new())?)?;
        }
        crate::generators::crates::write_lib(base, &layout::crate_dir(c, layer), "")?;
        crate::generators::crates::add_internal_dependency(
            base,
            &layout::crate_name(c, layer),
            &layout::crate_dir(c, layer),
        )?;
    }
    crate::generators::crates::sync_workspace_dependencies(base)?;
    Ok(())
}

fn mailpit_service(project: &str) -> String {
    format!(
        r#"  mailpit:
    image: {MAILPIT_IMAGE}
    container_name: {project}-mailpit
    ports:
      - "${{SMTP_HOST_PORT:-1025}}:1025"
      - "${{MAILPIT_UI_PORT:-8025}}:8025"
    restart: unless-stopped
"#
    )
}

/// Mailpit in `docker-compose.yml`: next to Postgres in a `--db` project, alone otherwise.
fn add_mailpit_to_compose(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    crate::generators::project::add_compose_service(base, "mailpit", &mailpit_service(project))
}

fn add_mail_env(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    // A host name has no `_`: `MAIL_FROM` has to parse as an address.
    let host = project.replace('_', "-");
    let vars = format!(
        "\n# Correo — Mailpit (docker-compose.yml) recoge cada correo en desarrollo: `make correo/ui`\n\
         SMTP_HOST_PORT=1025\nMAILPIT_UI_PORT=8025\n\
         SMTP_URL=smtp://localhost:1025\nMAIL_FROM=no-reply@{host}.localhost\n"
    );
    for file in [".env", ".env.example"] {
        let path = base.join(file);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.contains("SMTP_URL") {
            continue;
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&vars);
        fs::write(&path, content)?;
    }
    Ok(())
}

/// The compose targets a project without a database lacks (a `--db` one has them already).
const COMPOSE_MAKEFILE_TARGETS: &str = r#"
.PHONY: docker-compose/up docker-compose/down

DOCKER_COMPOSE := docker compose

docker-compose/up: ## Arranca los contenedores (Mailpit)
	@echo "${CYAN}Arrancando los contenedores...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Para los contenedores
	@echo "${CYAN}Parando los contenedores...${NC}"
	$(DOCKER_COMPOSE) down
"#;

const MAIL_MAKEFILE_TARGETS: &str = r#"
.PHONY: correo/ui

correo/ui: docker-compose/up ## Dónde llegan los correos enviados en desarrollo (Mailpit)
	@set -a; [ ! -f .env ] || . ./.env; set +a; \
	echo "${GREEN}Mailpit: http://localhost:$${MAILPIT_UI_PORT:-8025}${NC}"
"#;

fn add_mail_makefile_targets(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(());
    }
    let mut src = fs::read_to_string(&path)?;
    if src.contains("correo/ui:") {
        return Ok(());
    }
    if !src.ends_with('\n') {
        src.push('\n');
    }
    if !src.contains("docker-compose/up:") {
        src.push_str(COMPOSE_MAKEFILE_TARGETS);
    }
    src.push_str(MAIL_MAKEFILE_TARGETS);
    fs::write(&path, src)?;
    Ok(())
}

/// The root `AGENTS.md` section a project gets with its first mail (ADR-0010: tested).
const AGENTS_MAIL_SECTION: &str = r#"
## Correo

Un correo son tres piezas, que escribe una vez `puerto generar correo <nombre> [-- campo:Tipo …]`
y que desde entonces son tuyas (ADR-0020 de Puerto):

- `contextos/correo/dominio/src/<nombre>.rs`: `Correo<Nombre>` (el destinatario y los valores que
  muestra su texto) y el puerto `EnviadorCorreo<Nombre>`. El código que lo envía depende del
  puerto; los tests usan `dobles::MockEnviadorCorreo<Nombre>`.
- `contextos/correo/infraestructura/src/<nombre>.rs`: `EnviadorCorreo<Nombre>Plantilla`, que
  renderiza las plantillas y entrega el mensaje al `EnviadorCorreo` (SMTP). Sus tests renderizan
  todas las plantillas, así que una errata en una hace fallar `make test`.
- `contextos/correo/infraestructura/plantillas/<nombre>/`: `asunto.txt`, `cuerpo.html`,
  `cuerpo.txt`; minijinja, estricto (una variable desconocida es un error). Envuelve un campo
  opcional en `{% if x is not none %}`.

`SMTP_URL` y `MAIL_FROM` están en `.env`. En desarrollo Mailpit (`make docker-compose/up`) recoge
todos los correos (`make correo/ui`), y ninguno llega a un buzón real hasta que `SMTP_URL` apunta a
un proveedor.

**Quién lo envía.** En un proyecto con eventos de dominio, `puerto generar correo <nombre> --tras
<tipo_evento> --para <campo>` también escribe
`presentacion/worker/src/consumidores/correo_<nombre>.rs` (tuyo) y lo cablea en el registro del
worker: el worker construye el correo a partir del snapshot del evento y lo envía una vez por
evento (`puerto_correo::enviar_una_vez` guarda una fila en `correos_enviados`; una caída entre el
envío y esa fila lo envía dos veces, nunca ninguna). Puerto aún no cablea el envío desde un caso
de uso.
"#;

fn add_agents_section(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("AGENTS.md");
    let Ok(mut src) = fs::read_to_string(&path) else {
        return Ok(());
    };
    if src.contains("\n## Correo\n") {
        return Ok(());
    }
    if !src.ends_with('\n') {
        src.push('\n');
    }
    src.push_str(AGENTS_MAIL_SECTION);
    fs::write(&path, src)?;
    Ok(())
}

/// Everything the first mail of a project needs beyond its own files.
fn set_up_mail(base: &Path, config: &PuertoConfig) -> Result<(), Box<dyn std::error::Error>> {
    let project = &config.project.name;
    add_workspace_dependency(base, "puerto-correo")?;
    add_mailpit_to_compose(base, project)?;
    add_mail_env(base, project)?;
    add_mail_makefile_targets(base)?;
    add_agents_section(base)?;
    Ok(())
}

/// `puerto generar correo <name> [-- field:Type …]`, sent by nothing yet: see
/// [`run_generate_mail_on`].
#[cfg(test)]
pub fn run_generate_mail(
    name: &str,
    field_args: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    run_generate_mail_on(name, field_args, &[], None, base)
}

/// The events a mail is sent on and the snapshot field holding its recipient (`--tras`, `--para`).
struct Trigger {
    events: Vec<DeclaredEvent>,
    to: String,
}

/// A snapshot's fields, `(name, type)`: the entity's fields as the snapshot carries them (value
/// objects unwrapped to their primitive).
fn snapshot_fields(config: &PuertoConfig, event: &DeclaredEvent) -> Vec<(String, String)> {
    config
        .entity
        .iter()
        .find(|e| e.name == event.entity_pascal)
        .map(|e| {
            effective_fields(&e.fields)
                .into_iter()
                .map(|f| (f.name, f.field_type))
                .collect()
        })
        .unwrap_or_default()
}

fn has_field(fields: &[(String, String)], name: &str, rust_type: &str) -> bool {
    fields.iter().any(|(n, t)| n == name && t == rust_type)
}

/// Checks `--tras` / `--para` against the project: a worker, declared events, a `String` recipient in
/// every one of their snapshots.
fn trigger(
    config: &PuertoConfig,
    on: &[String],
    to: Option<&str>,
) -> Result<Option<Trigger>, String> {
    let to = match (on.is_empty(), to) {
        (true, None) => return Ok(None),
        (true, Some(_)) => {
            return Err(
                "--para nombra el destinatario de un correo que se envía tras eventos: necesita \
                 --tras <tipo_evento>"
                    .to_string(),
            );
        }
        (false, None) => {
            return Err(
                "--tras necesita --para <campo>: el campo String del snapshot del evento que \
                 guarda el destinatario"
                    .to_string(),
            );
        }
        (false, Some(to)) => to,
    };
    if !has_worker(config) {
        return Err(
            "un correo que se envía tras eventos necesita el worker: un proyecto creado con \
             `puerto nuevo --db --eventos db|kafka`"
                .to_string(),
        );
    }
    let mut unique: Vec<String> = vec![];
    for event in on {
        if !unique.contains(event) {
            unique.push(event.clone());
        }
    }
    let consumer = Consumer {
        name: String::new(),
        events: unique,
    };
    let events = subscribed(&consumer, &declared_events(config))?;
    for event in &events {
        let fields = snapshot_fields(config, event);
        if !has_field(&fields, to, "String") {
            let strings: Vec<&str> = fields
                .iter()
                .filter(|(_, t)| t == "String")
                .map(|(n, _)| n.as_str())
                .collect();
            return Err(format!(
                "--para {to}: el snapshot de {} no tiene el campo String '{to}' (sus campos String: {})",
                event.event_type,
                if strings.is_empty() {
                    "ninguno".to_string()
                } else {
                    strings.join(", ")
                }
            ));
        }
    }
    Ok(Some(Trigger {
        events,
        to: to.to_string(),
    }))
}

/// The fields of a mail sent on events: the ones named after `--` — each a field of every event's
/// snapshot, of the same type — or, none named, every snapshot field a template prints but the
/// recipient.
fn trigger_fields(
    config: &PuertoConfig,
    trigger: &Trigger,
    field_args: &[String],
) -> Result<Vec<MailField>, String> {
    let snapshots: Vec<(&DeclaredEvent, Vec<(String, String)>)> = trigger
        .events
        .iter()
        .map(|e| (e, snapshot_fields(config, e)))
        .collect();
    let in_every = |name: &str, rust_type: &str| {
        snapshots
            .iter()
            .find(|(_, fields)| !has_field(fields, name, rust_type))
            .map(|(event, _)| event.event_type.clone())
    };
    if field_args.is_empty() {
        let first = &snapshots[0].1;
        return Ok(first
            .iter()
            .filter(|(name, _)| *name != trigger.to)
            .filter_map(|(name, field_type)| {
                mail_field(&Field {
                    name: name.clone(),
                    field_type: field_type.clone(),
                    ..Field::default()
                })
                .ok()
            })
            .filter(|f| in_every(&f.name, &f.rust_type).is_none())
            .collect());
    }
    let fields = mail_fields(field_args)?;
    for f in &fields {
        if let Some(event) = in_every(&f.name, &f.rust_type) {
            return Err(format!(
                "campo '{}': el snapshot de {event} no tiene el campo {} '{}'. Un correo que se \
                 envía tras un evento se construye con su snapshot; añade otros valores al correo a \
                 mano después",
                f.name, f.rust_type, f.name
            ));
        }
    }
    Ok(fields)
}

/// `presentacion/worker/src/consumidores/correo_<name>.rs`: decodes each event's snapshot, builds the mail and
/// sends it through `enviar_una_vez` — with its tests.
fn render_handler(
    name: &str,
    fields: &[MailField],
    trigger: &Trigger,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut by_entity: Vec<(String, Vec<String>)> = vec![];
    for event in &trigger.events {
        let names = match by_entity.iter_mut().find(|(e, _)| *e == event.entity_snake) {
            Some((_, names)) => names,
            None => {
                by_entity.push((
                    event.entity_snake.clone(),
                    vec![format!("Snapshot{}", event.entity_pascal)],
                ));
                &mut by_entity.last_mut().ok_or("ninguna entidad")?.1
            }
        };
        names.push(event.constant.clone());
    }
    let imports: Vec<String> = by_entity
        .into_iter()
        .map(|(snake, mut names)| {
            names.sort();
            let names = if names.len() == 1 {
                names[0].clone()
            } else {
                format!("{{{}}}", names.join(", "))
            };
            format!("dominio_{snake}::eventos::{names}")
        })
        .collect();
    let mut ctx = tera::Context::new();
    ctx.insert("name", name);
    ctx.insert("pascal", &to_pascal_case(name));
    ctx.insert("fields", fields);
    ctx.insert("to", &trigger.to);
    ctx.insert("events", &trigger.events);
    ctx.insert("first", &trigger.events[0]);
    ctx.insert("imports", &imports);
    Ok(render("worker/mail_consumer.tera", &ctx)?)
}

/// What the worker needs once a mail is sent on events: `puerto-correo` and the `correos_enviados` table.
fn set_up_worker_mail(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join(layout::WORKER_MANIFEST);
    let mut src = fs::read_to_string(&path)?;
    if !src.contains("puerto-correo") {
        let events = "puerto-eventos = { workspace = true }\n";
        if !src.contains(events) {
            return Err(format!("{}: no tiene `{}`", path.display(), events.trim_end()).into());
        }
        src = src.replacen(
            events,
            &format!(
                "{events}# Correo (ADR-0020): el cliente SMTP y `enviar_una_vez`, para los correos que se envían \
                 tras un evento.\npuerto-correo = {{ workspace = true }}\n"
            ),
            1,
        );
        let dev = "puerto-correo = { workspace = true, features = [\"pruebas\"] }\n";
        src = match src.find("[dev-dependencies]\n") {
            Some(at) => {
                let at = at + "[dev-dependencies]\n".len();
                format!("{}{dev}{}", &src[..at], &src[at..])
            }
            None => format!("{src}\n[dev-dependencies]\n{dev}"),
        };
        fs::write(&path, src)?;
    }

    let migrations = base.join(layout::MIGRATIONS_DIR);
    let has_log = fs::read_dir(&migrations).is_ok_and(|entries| {
        entries.flatten().any(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with("_correos_enviados.sql")
        })
    });
    if !has_log {
        let sql = render("infrastructure/sent_mails.sql", &tera::Context::new())?;
        crate::generators::migration::run_migration("correos_enviados", base, Some(&sql))?;
    }
    crate::generators::crates::sync_workspace_dependencies(base)?;
    Ok(())
}

/// `puerto generar correo <name> [--tras <event_type>[,…] --para <field>] [-- field:Type …]`.
///
/// Refuses a mail already declared or already on disk (its files are never overwritten), an
/// invalid name and a field a mail cannot carry. With `--tras` (an `events` project) the worker
/// sends it: its consumer `correo_<name>` is written and wired, and the fields come from the
/// events' snapshots.
pub fn run_generate_mail_on(
    name: &str,
    field_args: &[String],
    on: &[String],
    to: Option<&str>,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    crate::validation::validate_mail_name(name)?;
    if config.mail.iter().any(|m| m.name == name) {
        return Err(format!("el correo '{name}' ya está declarado en puerto.toml").into());
    }
    let consumer_name = format!("correo_{name}");
    if config.consumer.iter().any(|c| c.name == consumer_name) {
        return Err(format!(
            "el worker ejecuta el correo '{name}' como el consumidor '{consumer_name}', que ya usa \
             un [[consumidor]]: elige otro nombre"
        )
        .into());
    }
    let trigger = trigger(&config, on, to)?;
    let fields = match &trigger {
        Some(trigger) => trigger_fields(&config, trigger, field_args)?,
        None => mail_fields(field_args)?,
    };

    let domain = base.join(layout::mail_port(name));
    let infra = base.join(layout::mail_adapter(name));
    let templates = base.join(layout::mail_templates_dir(name));
    let handler = base.join(layout::consumer(&consumer_name));
    let mut paths = vec![&domain, &infra, &templates];
    if trigger.is_some() {
        paths.push(&handler);
    }
    for path in paths {
        if path.exists() {
            return Err(format!(
                "{} ya existe, y nunca se sobrescribe. Bórralo o elige otro nombre",
                path.display()
            )
            .into());
        }
    }
    let (port, template_mailer) = render_files(name, &fields)?;
    let (subject, html, text) = starter_templates(name, &fields);
    let handler_src = match &trigger {
        Some(trigger) => Some(render_handler(name, &fields, trigger)?),
        None => None,
    };

    let first = config.mail.is_empty();
    crate::puerto_toml::add_mail(
        base,
        Mail {
            name: name.to_string(),
            on: trigger
                .as_ref()
                .map(|t| t.events.iter().map(|e| e.event_type.clone()).collect())
                .unwrap_or_default(),
        },
    )?;
    write_file(&domain, &port)?;
    write_file(&infra, &template_mailer)?;
    write_file(&templates.join("asunto.txt"), &subject)?;
    write_file(&templates.join("cuerpo.html"), &html)?;
    write_file(&templates.join("cuerpo.txt"), &text)?;
    write_mail_crates(base)?;
    if first {
        set_up_mail(base, &config)?;
    }
    if let Some(src) = handler_src {
        set_up_worker_mail(base)?;
        write_file(&handler, &src)?;
        let modules = base.join(layout::CONSUMERS_RS);
        fs::write(
            &modules,
            with_module(&fs::read_to_string(&modules)?, &consumer_name),
        )?;
        regenerate_registry(base, &crate::puerto_toml::read(base)?)?;
    }

    let pascal = to_pascal_case(name);
    println!("✓ Correo '{name}' añadido; desde aquí es tuyo:");
    println!(
        "  {}  Correo{pascal} y el puerto EnviadorCorreo{pascal}",
        layout::mail_port(name)
    );
    println!(
        "  {}  EnviadorCorreo{pascal}Plantilla, con sus tests",
        layout::mail_adapter(name)
    );
    println!(
        "  {}/  asunto.txt, cuerpo.html, cuerpo.txt",
        layout::mail_templates_dir(name)
    );
    if let Some(trigger) = &trigger {
        println!(
            "  {}  lo envía tras {}, a `{}`: una vez por evento",
            layout::consumer(&consumer_name),
            trigger
                .events
                .iter()
                .map(|e| e.event_type.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            trigger.to
        );
    }
    if first {
        println!(
            "  Mailpit añadido a docker-compose.yml; SMTP_URL y MAIL_FROM, a .env. \
             Mira los correos con `make correo/ui`."
        );
        for warning in crate::generators::ports::taken(base, crate::generators::ports::MAIL_PORTS) {
            eprintln!("{warning}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(arg: &str) -> Field {
        parse_field_arg(arg).unwrap()
    }

    #[test]
    fn should_sample_an_option_with_a_value_so_its_branch_renders() {
        let f = mail_field(&field("plan:opt:String")).unwrap();

        assert_eq!(f.rust_type, "Option<String>");
        assert_eq!(f.sample, r#"Some("example".to_string())"#);
    }

    #[test]
    fn should_import_chrono_once_when_two_fields_are_dates() {
        let fields = mail_fields(&[
            "paid_at:DateTime".to_string(),
            "shipped_at:opt:DateTime".to_string(),
            "order_id:Uuid".to_string(),
        ])
        .unwrap();

        assert_eq!(
            imports(&fields),
            [
                "chrono::{DateTime, Utc}".to_string(),
                "uuid::Uuid".to_string()
            ]
        );
    }

    #[test]
    fn should_title_the_mail_after_its_name() {
        assert_eq!(humanize("order_shipped"), "Order shipped");
        assert_eq!(humanize("welcome"), "Welcome");
    }

    #[test]
    fn should_put_the_first_string_in_the_subject_when_there_is_one() {
        let fields = mail_fields(&["seats:i64".to_string(), "name:String".to_string()]).unwrap();

        let (subject, _, text) = starter_templates("welcome", &fields);

        assert_eq!(subject, "Welcome, {{ name }}\n");
        assert_eq!(text, "Welcome\n\nSeats: {{ seats }}\nName: {{ name }}\n");
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("puerto_mail_unit_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn should_keep_the_last_variable_intact_when_env_has_no_final_newline() {
        let dir = scratch("env");
        fs::write(dir.join(".env"), "RUST_LOG=info").unwrap();

        add_mail_env(&dir, "shop").unwrap();

        let env = fs::read_to_string(dir.join(".env")).unwrap();
        assert!(env.starts_with("RUST_LOG=info\n\n# Correo"), "{env}");
        assert!(
            env.ends_with("MAIL_FROM=no-reply@shop.localhost\n"),
            "{env}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn should_title_the_subject_alone_when_no_field_is_a_string() {
        let fields = mail_fields(&["seats:i64".to_string()]).unwrap();

        let (subject, _, _) = starter_templates("welcome", &fields);

        assert_eq!(subject, "Welcome\n");
    }
}
