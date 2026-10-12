//! `puerto generate mail <name> [-- field:Type …]` (ADR-0020): a mail's port in the domain, its
//! template implementation in infrastructure and its templates — written once, the user's from
//! then on. The first mail of a project also brings `puerto-mail`, Mailpit and `SMTP_URL`.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::generators::naming::{to_pascal_case, write_file};
use crate::generators::project::add_workspace_dependency;
use crate::generators::render::render;
use crate::generators::types::resolve_type;
use crate::patchers::lib_rs::{insert_before_block_end, patch_lib_block};
use crate::puerto_toml::{Field, Mail, PuertoConfig, parse_field_arg};

/// The SMTP server development runs against (ADR-0020 § 5): SMTP on 1025, UI and API on 8025.
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
    if name == "to" {
        return Err(
            "field 'to' is the recipient every mail already has — pick another name".to_string(),
        );
    }
    if field.value_object.is_some() {
        return Err(format!(
            "field '{name}': a mail carries primitives, not a value object — use \
             {name}:<type>"
        ));
    }
    if field.unique {
        return Err(format!(
            "field '{name}': `!` (unique) is a database constraint, it means nothing on a mail"
        ));
    }
    let mapping = resolve_type(&field.field_type).map_err(|e| format!("field '{name}': {e}"))?;
    if mapping.rust_type.starts_with("Vec<") || mapping.rust_type.starts_with("HashMap<") {
        return Err(format!(
            "field '{name}': {} is not a value a mail template prints — use String, i64, bool, \
             f64, Uuid, DateTime or opt:<type>",
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

/// The fields of a mail from its CLI arguments; none given → `name: String`.
fn mail_fields(args: &[String]) -> Result<Vec<MailField>, String> {
    let fields: Vec<Field> = if args.is_empty() {
        vec![Field {
            name: "name".into(),
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
            return Err(format!("duplicate field '{}'", field.name));
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

/// `pub mod <name>;` in `domain { mail { … } }` of `business/src/lib.rs`, opening the block on
/// the first mail.
fn patch_business_lib(base: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;
    let patched = if src.contains("pub mod mail {") {
        // Right before the block's closing brace (indented by 4): no blank line, so rustfmt
        // keeps the declarations sorted.
        patch_lib_block(
            &src,
            &["domain", "mail"],
            &format!("    pub mod {name};\n    "),
        )?
    } else {
        insert_before_block_end(
            &src,
            "domain",
            &format!("\n    pub mod mail {{\n        pub mod {name};\n    }}\n"),
        )?
    };
    fs::write(&path, patched)?;
    Ok(())
}

/// `pub mod <name>;` in `mail { … }` of `infrastructure/src/lib.rs`, opening the block on the
/// first mail.
fn patch_infra_lib(base: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("infrastructure/src/lib.rs");
    let mut src = fs::read_to_string(&path)?;
    if src.contains("pub mod mail {") {
        src = insert_before_block_end(&src, "mail", &format!("    pub mod {name};\n"))?;
    } else {
        if !src.is_empty() && !src.ends_with('\n') {
            src.push('\n');
        }
        src.push_str(&format!("pub mod mail {{\n    pub mod {name};\n}}\n"));
    }
    fs::write(&path, src)?;
    Ok(())
}

/// `puerto-mail` for the template mailers, and its fakes for their tests.
fn add_puerto_mail(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    add_workspace_dependency(base, "puerto-mail")?;
    let path = base.join("infrastructure/Cargo.toml");
    let mut src = fs::read_to_string(&path)?;
    if src.contains("puerto-mail") {
        return Ok(());
    }
    let core = "puerto-core = { workspace = true }\n";
    if !src.contains(core) {
        return Err(format!("{}: no `{}`", path.display(), core.trim_end()).into());
    }
    src = src.replacen(
        core,
        &format!("{core}puerto-mail = {{ workspace = true }}\n"),
        1,
    );
    let dev = "puerto-mail = { workspace = true, features = [\"test-utils\"] }\n";
    src = match src.find("[dev-dependencies]\n") {
        Some(at) => {
            let at = at + "[dev-dependencies]\n".len();
            format!("{}{dev}{}", &src[..at], &src[at..])
        }
        None => format!("{src}\n[dev-dependencies]\n{dev}"),
    };
    fs::write(&path, src)?;
    Ok(())
}

fn mailpit_service(project: &str) -> String {
    format!(
        r#"  mailpit:
    image: {MAILPIT_IMAGE}
    container_name: {project}-mailpit
    ports:
      - "1025:1025"
      - "8025:8025"
    restart: unless-stopped
"#
    )
}

/// Mailpit in `docker-compose.yml`: next to Postgres in a `--db` project, alone otherwise.
fn add_mailpit_to_compose(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("docker-compose.yml");
    if !path.exists() {
        write_file(&path, &format!("services:\n{}", mailpit_service(project)))?;
        return Ok(());
    }
    let src = fs::read_to_string(&path)?;
    if src.contains("mailpit:") {
        return Ok(());
    }
    let out = match src.find("\nvolumes:\n") {
        // After the last service, before the top-level volumes.
        Some(at) => format!("{}{}{}", &src[..=at], mailpit_service(project), &src[at..]),
        None => {
            let sep = if src.ends_with('\n') { "" } else { "\n" };
            format!("{src}{sep}{}", mailpit_service(project))
        }
    };
    fs::write(&path, out)?;
    Ok(())
}

fn add_mail_env(base: &Path, project: &str) -> Result<(), Box<dyn std::error::Error>> {
    // A host name has no `_`: `MAIL_FROM` has to parse as an address.
    let host = project.replace('_', "-");
    let vars = format!(
        "\n# Mail — Mailpit (docker-compose.yml) catches every mail in development: `make mail/ui`\n\
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

docker-compose/up: ## Start the containers (Mailpit)
	@echo "${CYAN}Starting containers...${NC}"
	$(DOCKER_COMPOSE) up -d --wait

docker-compose/down: ## Stop the containers
	@echo "${CYAN}Stopping containers...${NC}"
	$(DOCKER_COMPOSE) down
"#;

const MAIL_MAKEFILE_TARGETS: &str = r#"
.PHONY: mail/ui

mail/ui: docker-compose/up ## Where the mails sent in development land (Mailpit)
	@echo "${GREEN}Mailpit: http://localhost:8025${NC}"
"#;

fn add_mail_makefile_targets(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("Makefile");
    if !path.exists() {
        return Ok(());
    }
    let mut src = fs::read_to_string(&path)?;
    if src.contains("mail/ui:") {
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
## Mail

A mail is three pieces, written once by `puerto generate mail <name> [-- field:Type …]` and yours
from then on (ADR-0020 of Puerto):

- `business/src/domain/mail/<name>.rs` — `{Name}Mail` (the recipient `to` and the values its text
  shows) and the port `{Name}Mailer`. Code that sends it depends on the port; tests use
  `mocks::Mock{Name}Mailer`.
- `infrastructure/src/mail/<name>.rs` — `Template{Name}Mailer`: renders the templates and hands
  the message to the `puerto_mail::Mailer` (SMTP). Its tests render every template, so a typo in
  one fails `make test`.
- `infrastructure/mail/<name>/` — `subject.txt`, `body.html`, `body.txt`: minijinja, strict (an
  unknown variable is an error). Wrap an optional field in `{% if x is not none %}`.

`SMTP_URL` and `MAIL_FROM` are in `.env`. In development Mailpit (`make docker-compose/up`)
catches every mail — `make mail/ui` — and nothing reaches a real inbox until `SMTP_URL` points at
a provider.
"#;

fn add_agents_section(base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("AGENTS.md");
    let Ok(mut src) = fs::read_to_string(&path) else {
        return Ok(());
    };
    if src.contains("\n## Mail\n") {
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
    add_puerto_mail(base)?;
    add_mailpit_to_compose(base, project)?;
    add_mail_env(base, project)?;
    add_mail_makefile_targets(base)?;
    add_agents_section(base)?;
    Ok(())
}

/// `puerto generate mail <name> [-- field:Type …]`. Refuses a mail already declared or already on
/// disk (its files are never overwritten), an invalid name and a field a mail cannot carry.
pub fn run_generate_mail(
    name: &str,
    field_args: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    crate::validation::validate_mail_name(name)?;
    if config.mail.iter().any(|m| m.name == name) {
        return Err(format!("mail '{name}' is already declared in puerto.toml").into());
    }
    let fields = mail_fields(field_args)?;

    let domain = base.join(format!("business/src/domain/mail/{name}.rs"));
    let infra = base.join(format!("infrastructure/src/mail/{name}.rs"));
    let templates = base.join(format!("infrastructure/mail/{name}"));
    for path in [&domain, &infra, &templates] {
        if path.exists() {
            return Err(format!(
                "{} already exists — it is never overwritten. Remove it or pick another name",
                path.display()
            )
            .into());
        }
    }
    let (port, template_mailer) = render_files(name, &fields)?;
    let (subject, html, text) = starter_templates(name, &fields);

    let first = config.mail.is_empty();
    crate::puerto_toml::add_mail(
        base,
        Mail {
            name: name.to_string(),
            on: vec![],
        },
    )?;
    write_file(&domain, &port)?;
    write_file(&infra, &template_mailer)?;
    write_file(&templates.join("subject.txt"), &subject)?;
    write_file(&templates.join("body.html"), &html)?;
    write_file(&templates.join("body.txt"), &text)?;
    patch_business_lib(base, name)?;
    patch_infra_lib(base, name)?;
    if first {
        set_up_mail(base, &config)?;
    }

    let pascal = to_pascal_case(name);
    println!("✓ Mail '{name}' added — yours from here on:");
    println!("  business/src/domain/mail/{name}.rs       {pascal}Mail + the port {pascal}Mailer");
    println!("  infrastructure/src/mail/{name}.rs        Template{pascal}Mailer, with its tests");
    println!("  infrastructure/mail/{name}/              subject.txt, body.html, body.txt");
    if first {
        println!(
            "  Mailpit added to docker-compose.yml; SMTP_URL and MAIL_FROM to .env. \
             See the mails with `make mail/ui`."
        );
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
        fs::create_dir_all(dir.join("infrastructure/src")).unwrap();
        dir
    }

    #[test]
    fn should_start_the_mail_block_on_its_own_line_when_lib_rs_has_no_final_newline() {
        let dir = scratch("infra_lib");
        let lib = dir.join("infrastructure/src/lib.rs");
        fs::write(&lib, "pub mod greeting {\n    pub mod repository;\n}").unwrap();

        patch_infra_lib(&dir, "welcome").unwrap();

        assert_eq!(
            fs::read_to_string(&lib).unwrap(),
            "pub mod greeting {\n    pub mod repository;\n}\npub mod mail {\n    pub mod welcome;\n}\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn should_keep_the_last_variable_intact_when_env_has_no_final_newline() {
        let dir = scratch("env");
        fs::write(dir.join(".env"), "RUST_LOG=info").unwrap();

        add_mail_env(&dir, "shop").unwrap();

        let env = fs::read_to_string(dir.join(".env")).unwrap();
        assert!(env.starts_with("RUST_LOG=info\n\n# Mail"), "{env}");
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
