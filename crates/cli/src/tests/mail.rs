// Phase 14.3 (ADR-0020): `puerto generate mail <name> [-- field:Type …]` in every project — the
// domain port, the template implementation, the templates, and the mail setup on the first one.
use super::*;
use crate::commands::{list, validate};

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A fresh project without a database (no web: nothing here concerns the frontend).
fn project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::apply_no_web(&output).unwrap();
    (dir, output)
}

fn db_project(name: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = project(name);
    scaffold::apply_db_to_new_project(&output).unwrap();
    (dir, output)
}

fn args(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|f| f.to_string()).collect()
}

fn error_of(result: Result<(), Box<dyn std::error::Error>>) -> String {
    result.err().map(|e| e.to_string()).unwrap_or_default()
}

fn welcome(output: &Path) {
    scaffold::run_generate_mail(
        "welcome",
        &args(&["name:String", "plan:opt:String", "seats:i64"]),
        output,
    )
    .unwrap();
}

// ── The mail's files ─────────────────────────────────────────────────────

#[test]
fn generate_mail_writes_the_port_its_template_implementation_and_the_templates() {
    let (dir, output) = project("mail_files");

    welcome(&output);

    let domain = read(&output, "business/src/domain/mail/welcome.rs");
    assert!(domain.contains("pub struct WelcomeMail {"), "{domain}");
    assert!(domain.contains("pub to: String,"), "{domain}");
    assert!(domain.contains("pub name: String,"), "{domain}");
    assert!(domain.contains("pub plan: Option<String>,"), "{domain}");
    assert!(domain.contains("pub seats: i64,"), "{domain}");
    assert!(
        domain.contains("pub trait WelcomeMailer: Send + Sync {"),
        "{domain}"
    );
    assert!(
        domain.contains("async fn send(&self, mail: &WelcomeMail) -> Result<(), MailError>;"),
        "{domain}"
    );
    assert!(domain.contains("pub WelcomeMailer {}"), "{domain}");
    assert!(domain.contains("#[cfg(any(test, feature = \"test-utils\"))]"));
    // The domain names no template engine and no SMTP.
    assert!(!domain.contains("puerto_mail"), "{domain}");
    assert!(
        domain.contains("use puerto_core::mail::MailError;"),
        "{domain}"
    );

    let infra = read(&output, "infrastructure/src/mail/welcome.rs");
    assert!(
        infra.contains("pub struct TemplateWelcomeMailer {"),
        "{infra}"
    );
    assert!(
        infra.contains("impl WelcomeMailer for TemplateWelcomeMailer {"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../../mail/welcome/subject.txt\")"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../../mail/welcome/body.html\")"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../../mail/welcome/body.txt\")"),
        "{infra}"
    );
    assert!(infra.contains("RecordingMailer"), "{infra}");

    let subject = read(&output, "infrastructure/mail/welcome/subject.txt");
    assert!(subject.contains("{{ name }}"), "{subject}");
    let html = read(&output, "infrastructure/mail/welcome/body.html");
    let text = read(&output, "infrastructure/mail/welcome/body.txt");
    for body in [&html, &text] {
        assert!(body.contains("{{ name }}"), "{body}");
        assert!(body.contains("{{ seats }}"), "{body}");
        // An Option prints only when it is set: `none` is not a word a reader should see.
        assert!(body.contains("{% if plan is not none %}"), "{body}");
    }
    cleanup(&dir);
}

#[test]
fn generate_mail_without_fields_carries_a_name() {
    let (dir, output) = project("mail_default_field");

    scaffold::run_generate_mail("welcome", &[], &output).unwrap();

    let domain = read(&output, "business/src/domain/mail/welcome.rs");
    assert!(domain.contains("pub name: String,"), "{domain}");
    cleanup(&dir);
}

#[test]
fn generate_mail_imports_what_its_field_types_need() {
    let (dir, output) = project("mail_imports");

    scaffold::run_generate_mail(
        "receipt",
        &args(&[
            "order_id:Uuid",
            "paid_at:DateTime",
            "total:f64",
            "gift:bool",
        ]),
        &output,
    )
    .unwrap();

    let domain = read(&output, "business/src/domain/mail/receipt.rs");
    assert!(domain.contains("use uuid::Uuid;"), "{domain}");
    assert!(domain.contains("use chrono::{DateTime, Utc};"), "{domain}");
    assert!(domain.contains("pub paid_at: DateTime<Utc>,"), "{domain}");
    cleanup(&dir);
}

#[test]
fn generate_mail_declares_its_modules_in_both_crates() {
    let (dir, output) = project("mail_modules");

    welcome(&output);
    scaffold::run_generate_mail("receipt", &[], &output).unwrap();

    let business = read(&output, "business/src/lib.rs");
    assert_eq!(business.matches("pub mod mail {").count(), 1, "{business}");
    assert!(business.contains("pub mod welcome;"), "{business}");
    assert!(business.contains("pub mod receipt;"), "{business}");
    // Inside `domain`: the mail data and its port are domain types.
    let domain_block = &business[business.find("pub mod domain {").unwrap()..];
    let domain_end = domain_block.find("\n}\n").unwrap();
    assert!(
        domain_block[..domain_end].contains("pub mod mail {"),
        "{business}"
    );

    let infra = read(&output, "infrastructure/src/lib.rs");
    assert_eq!(infra.matches("pub mod mail {").count(), 1, "{infra}");
    assert!(infra.contains("pub mod welcome;"), "{infra}");
    assert!(infra.contains("pub mod receipt;"), "{infra}");
    cleanup(&dir);
}

#[test]
fn generate_mail_declares_it_in_puerto_toml() {
    let (dir, output) = project("mail_toml");

    welcome(&output);

    let toml = read(&output, "puerto.toml");
    assert!(toml.contains("[[mail]]\nname = \"welcome\"\n"), "{toml}");
    // Wiring only: no field, no recipient — those are in the mail's files.
    assert!(!toml.contains("seats"), "{toml}");
    assert!(!toml.contains("on = "), "{toml}");
    cleanup(&dir);
}

// ── The first mail sets the project up ───────────────────────────────────

#[test]
fn first_mail_adds_puerto_mail_to_the_workspace_and_infrastructure() {
    let (dir, output) = project("mail_deps");

    welcome(&output);

    let root = read(&output, "Cargo.toml");
    assert!(root.contains("puerto-mail = \""), "{root}");
    let infra = read(&output, "infrastructure/Cargo.toml");
    let (deps, dev) = infra.split_once("[dev-dependencies]").unwrap();
    assert!(
        deps.contains("puerto-mail = { workspace = true }"),
        "{infra}"
    );
    assert!(
        dev.contains("puerto-mail = { workspace = true, features = [\"test-utils\"] }"),
        "{infra}"
    );
    cleanup(&dir);
}

#[test]
fn first_mail_of_a_project_without_database_creates_compose_with_mailpit_alone() {
    let (dir, output) = project("mail_compose_new");
    assert!(!output.join("docker-compose.yml").exists());

    welcome(&output);

    let compose = read(&output, "docker-compose.yml");
    assert!(compose.starts_with("services:\n  mailpit:\n"), "{compose}");
    assert!(
        compose.contains("image: axllent/mailpit:v1.31.1"),
        "{compose}"
    );
    assert!(compose.contains("\"1025:1025\""), "{compose}");
    assert!(compose.contains("\"8025:8025\""), "{compose}");
    assert!(!compose.contains("postgres"), "{compose}");

    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("docker-compose/up:"), "{makefile}");
    assert!(makefile.contains("docker-compose/down:"), "{makefile}");
    assert!(makefile.contains("mail/ui:"), "{makefile}");
    assert!(makefile.contains("http://localhost:8025"), "{makefile}");
    cleanup(&dir);
}

#[test]
fn first_mail_of_a_database_project_adds_mailpit_next_to_postgres() {
    let (dir, output) = db_project("mail_compose_db");

    welcome(&output);

    let compose = read(&output, "docker-compose.yml");
    assert!(compose.contains("  postgres:\n"), "{compose}");
    assert!(compose.contains("  mailpit:\n"), "{compose}");
    assert!(
        compose.contains("\nvolumes:\n  postgres-data:\n"),
        "{compose}"
    );
    // Before the top-level `volumes:`: a service, not a volume.
    assert!(
        compose.find("  mailpit:").unwrap() < compose.find("\nvolumes:").unwrap(),
        "{compose}"
    );
    let makefile = read(&output, "Makefile");
    assert_eq!(
        makefile.matches("\ndocker-compose/up:").count(),
        1,
        "{makefile}"
    );
    assert!(makefile.contains("mail/ui:"), "{makefile}");
    cleanup(&dir);
}

#[test]
fn first_mail_points_env_at_mailpit() {
    let (dir, output) = project("mail_env");

    welcome(&output);

    for file in [".env", ".env.example"] {
        let env = read(&output, file);
        assert!(
            env.contains("SMTP_URL=smtp://localhost:1025\n"),
            "{file}: {env}"
        );
        assert!(
            env.contains("MAIL_FROM=no-reply@mail-env.localhost\n"),
            "{file}: {env}"
        );
    }
    cleanup(&dir);
}

#[test]
fn first_mail_documents_mail_in_the_root_agents_md() {
    let (dir, output) = project("mail_agents");

    welcome(&output);

    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains("## Mail"), "{agents}");
    assert!(agents.contains("puerto generate mail"), "{agents}");
    assert!(agents.contains("infrastructure/mail/<name>/"), "{agents}");
    cleanup(&dir);
}

#[test]
fn second_mail_does_not_repeat_the_setup() {
    let (dir, output) = db_project("mail_setup_once");
    welcome(&output);
    let files = [
        "Cargo.toml",
        "infrastructure/Cargo.toml",
        "docker-compose.yml",
        ".env",
        ".env.example",
        "Makefile",
        "AGENTS.md",
    ];
    let before: Vec<String> = files.iter().map(|f| read(&output, f)).collect();

    scaffold::run_generate_mail("receipt", &[], &output).unwrap();

    let after: Vec<String> = files.iter().map(|f| read(&output, f)).collect();
    assert_eq!(before, after);
    cleanup(&dir);
}

// ── Refusals ─────────────────────────────────────────────────────────────

#[test]
fn generate_mail_refuses_a_mail_already_declared() {
    let (dir, output) = project("mail_duplicate");
    welcome(&output);

    let err = error_of(scaffold::run_generate_mail("welcome", &[], &output));

    assert!(err.contains("already"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_mail_refuses_a_name_that_is_not_snake_case_or_is_a_keyword() {
    let (dir, output) = project("mail_bad_name");

    for name in ["Welcome", "welcome-mail", "1welcome", "match"] {
        let err = error_of(scaffold::run_generate_mail(name, &[], &output));
        assert!(err.contains(name), "{name}: {err}");
    }
    assert!(!output.join("business/src/domain/mail").exists());
    cleanup(&dir);
}

#[test]
fn generate_mail_refuses_fields_a_mail_cannot_carry() {
    let (dir, output) = project("mail_bad_fields");

    for (field, reason) in [
        ("to:String", "to"),
        ("name:Name:String", "value object"),
        ("sku:String!", "unique"),
        ("tags:vec:String", "Vec<String>"),
        ("meta:map", "HashMap<String, String>"),
        ("price:u32", "u32"),
    ] {
        let err = error_of(scaffold::run_generate_mail(
            "welcome",
            &args(&[field]),
            &output,
        ));
        assert!(err.contains(reason), "{field}: {err}");
    }
    let err = error_of(scaffold::run_generate_mail(
        "welcome",
        &args(&["name:String", "name:i64"]),
        &output,
    ));
    assert!(err.contains("duplicate"), "{err}");
    assert!(!read(&output, "puerto.toml").contains("[[mail]]"));
    cleanup(&dir);
}

#[test]
fn an_entity_named_mail_is_refused_because_mails_own_the_module() {
    let err = crate::validation::validate_entity_name("Mail").unwrap_err();

    assert!(err.contains("mail"), "{err}");
}

// ── validate / list ──────────────────────────────────────────────────────

#[test]
fn validate_accepts_a_project_with_mails() {
    let (dir, output) = project("mail_validate_ok");
    welcome(&output);

    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn validate_reports_a_mail_whose_files_are_missing() {
    let (dir, output) = project("mail_validate_drift");
    welcome(&output);
    fs::remove_file(output.join("infrastructure/mail/welcome/body.html")).unwrap();
    fs::remove_file(output.join("business/src/domain/mail/welcome.rs")).unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(
        err.contains("infrastructure/mail/welcome/body.html"),
        "{err}"
    );
    assert!(err.contains("business/src/domain/mail/welcome.rs"), "{err}");
    cleanup(&dir);
}

#[test]
fn validate_rejects_invalid_and_duplicate_mail_names() {
    let (dir, output) = project("mail_validate_names");
    let toml = read(&output, "puerto.toml");
    fs::write(
        output.join("puerto.toml"),
        format!("{toml}\n[[mail]]\nname = \"Welcome\"\n\n[[mail]]\nname = \"Welcome\"\n"),
    )
    .unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(err.contains("invalid mail name 'Welcome'"), "{err}");
    assert!(err.contains("duplicate mail name 'Welcome'"), "{err}");
    cleanup(&dir);
}

#[test]
fn validate_rejects_a_mail_sent_on_events_in_a_project_without_events() {
    let (dir, output) = project("mail_validate_on");
    welcome(&output);
    let toml = read(&output, "puerto.toml");
    fs::write(
        output.join("puerto.toml"),
        toml.replace(
            "name = \"welcome\"\n",
            "name = \"welcome\"\non = [\"greeting_created\"]\n",
        ),
    )
    .unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(err.contains("mail 'welcome'"), "{err}");
    assert!(err.contains("project.events is not set"), "{err}");
    cleanup(&dir);
}

#[test]
fn list_shows_the_mails() {
    let (dir, output) = project("mail_list");
    welcome(&output);
    scaffold::run_generate_mail("receipt", &[], &output).unwrap();

    let report = list::list_report(&puerto_toml::read(&output).unwrap());

    assert!(
        report.contains("\nMails:\n  welcome\n  receipt\n"),
        "{report}"
    );
    cleanup(&dir);
}

#[test]
fn validate_does_not_take_the_mail_directory_for_an_orphan_entity() {
    let (dir, output) = project("mail_not_orphan");
    welcome(&output);

    let orphans = crate::generators::consistency::orphan_entity_dirs(
        &output,
        &puerto_toml::read(&output).unwrap(),
    );

    assert!(orphans.is_empty(), "{orphans:?}");
    cleanup(&dir);
}
