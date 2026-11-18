// Phase 14.3 (ADR-0020): `puerto generar correo <name> [-- field:Type …]` in every project — the
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

    let domain = read(&output, "contextos/correo/dominio/src/welcome.rs");
    assert!(domain.contains("pub struct CorreoWelcome {"), "{domain}");
    assert!(domain.contains("pub para: String,"), "{domain}");
    assert!(domain.contains("pub name: String,"), "{domain}");
    assert!(domain.contains("pub plan: Option<String>,"), "{domain}");
    assert!(domain.contains("pub seats: i64,"), "{domain}");
    assert!(
        domain.contains("pub trait EnviadorCorreoWelcome: Send + Sync {"),
        "{domain}"
    );
    assert!(
        domain
            .contains("async fn enviar(&self, correo: &CorreoWelcome) -> Result<(), ErrorCorreo>;"),
        "{domain}"
    );
    assert!(domain.contains("pub EnviadorCorreoWelcome {}"), "{domain}");
    assert!(domain.contains("#[cfg(any(test, feature = \"pruebas\"))]"));
    // The domain names no template engine and no SMTP.
    assert!(!domain.contains("puerto_correo"), "{domain}");
    assert!(
        domain.contains("use puerto_nucleo::correo::ErrorCorreo;"),
        "{domain}"
    );

    let infra = read(&output, "contextos/correo/infraestructura/src/welcome.rs");
    assert!(
        infra.contains("pub struct EnviadorCorreoWelcomePlantilla {"),
        "{infra}"
    );
    assert!(
        infra.contains("impl EnviadorCorreoWelcome for EnviadorCorreoWelcomePlantilla {"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../plantillas/welcome/asunto.txt\")"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../plantillas/welcome/cuerpo.html\")"),
        "{infra}"
    );
    assert!(
        infra.contains("include_str!(\"../plantillas/welcome/cuerpo.txt\")"),
        "{infra}"
    );
    assert!(infra.contains("EnviadorCorreoGrabador"), "{infra}");

    let subject = read(
        &output,
        "contextos/correo/infraestructura/plantillas/welcome/asunto.txt",
    );
    assert!(subject.contains("{{ name }}"), "{subject}");
    let html = read(
        &output,
        "contextos/correo/infraestructura/plantillas/welcome/cuerpo.html",
    );
    let text = read(
        &output,
        "contextos/correo/infraestructura/plantillas/welcome/cuerpo.txt",
    );
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

    let domain = read(&output, "contextos/correo/dominio/src/welcome.rs");
    assert!(domain.contains("pub nombre: String,"), "{domain}");
    cleanup(&dir);
}

#[test]
fn generate_mail_imports_what_its_field_types_need() {
    let (dir, output) = project("mail_imports");

    scaffold::run_generate_mail(
        "receipt",
        &args(&[
            "pedido_id:Uuid",
            "paid_at:DateTime",
            "total:f64",
            "gift:bool",
        ]),
        &output,
    )
    .unwrap();

    let domain = read(&output, "contextos/correo/dominio/src/receipt.rs");
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

    // The `correo` context: the mail data and its port are domain types, their templates and
    // mailers infrastructure.
    for layer in ["dominio", "infraestructura"] {
        let lib = read(&output, &format!("contextos/correo/{layer}/src/lib.rs"));
        assert_eq!(lib, "pub mod receipt;\npub mod welcome;\n", "{layer}");
    }
    let root = read(&output, "Cargo.toml");
    assert!(
        root.contains("dominio_correo = { path = \"contextos/correo/dominio\" }"),
        "{root}"
    );
    assert!(root.contains("infra_correo = { path = \"contextos/correo/infraestructura\" }"));
    cleanup(&dir);
}

#[test]
fn generate_mail_declares_it_in_puerto_toml() {
    let (dir, output) = project("mail_toml");

    welcome(&output);

    let toml = read(&output, "puerto.toml");
    assert!(
        toml.contains("[[correo]]\nnombre = \"welcome\"\n"),
        "{toml}"
    );
    // Wiring only: no field, no recipient — those are in the mail's files.
    assert!(!toml.contains("seats"), "{toml}");
    assert!(!toml.contains("tras = "), "{toml}");
    cleanup(&dir);
}

// ── The first mail sets the project up ───────────────────────────────────

#[test]
fn first_mail_adds_puerto_mail_to_the_workspace_and_infrastructure() {
    let (dir, output) = project("mail_deps");

    welcome(&output);

    let root = read(&output, "Cargo.toml");
    assert!(root.contains("puerto-correo = \""), "{root}");
    let infra = read(&output, "contextos/correo/infraestructura/Cargo.toml");
    let (deps, dev) = infra.split_once("[dev-dependencies]").unwrap();
    assert!(
        deps.contains("puerto-correo = { workspace = true }"),
        "{infra}"
    );
    assert!(
        dev.contains("puerto-correo = { workspace = true, features = [\"pruebas\"] }"),
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
    assert!(makefile.contains("correo/ui:"), "{makefile}");
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
    assert!(makefile.contains("correo/ui:"), "{makefile}");
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
    assert!(agents.contains("## Correo"), "{agents}");
    assert!(agents.contains("puerto generar correo"), "{agents}");
    assert!(
        agents.contains("contextos/correo/infraestructura/plantillas/<nombre>/"),
        "{agents}"
    );
    cleanup(&dir);
}

#[test]
fn second_mail_does_not_repeat_the_setup() {
    let (dir, output) = db_project("mail_setup_once");
    welcome(&output);
    let files = [
        "Cargo.toml",
        "contextos/correo/infraestructura/Cargo.toml",
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

    assert!(err.contains("ya está declarado"), "{err}");
    cleanup(&dir);
}

#[test]
fn generate_mail_refuses_a_name_that_is_not_snake_case_or_is_a_keyword() {
    let (dir, output) = project("mail_bad_name");

    for name in ["Welcome", "welcome-mail", "1welcome", "match"] {
        let err = error_of(scaffold::run_generate_mail(name, &[], &output));
        assert!(err.contains(name), "{name}: {err}");
    }
    assert!(!output.join("contextos/correo").exists());
    cleanup(&dir);
}

#[test]
fn generate_mail_refuses_fields_a_mail_cannot_carry() {
    let (dir, output) = project("mail_bad_fields");

    for (field, reason) in [
        ("para:String", "para"),
        ("name:Name:String", "objeto valor"),
        ("sku:String!", "único"),
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
    assert!(err.contains("repetido"), "{err}");
    assert!(!read(&output, "puerto.toml").contains("[[correo]]"));
    cleanup(&dir);
}

#[test]
fn an_entity_named_correo_is_refused_because_mails_own_the_context() {
    let err = crate::validation::validate_entity_name("Correo").unwrap_err();

    assert!(err.contains("correo"), "{err}");
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
    fs::remove_file(output.join("contextos/correo/infraestructura/plantillas/welcome/cuerpo.html"))
        .unwrap();
    fs::remove_file(output.join("contextos/correo/dominio/src/welcome.rs")).unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(
        err.contains("contextos/correo/infraestructura/plantillas/welcome/cuerpo.html"),
        "{err}"
    );
    assert!(
        err.contains("contextos/correo/dominio/src/welcome.rs"),
        "{err}"
    );
    cleanup(&dir);
}

#[test]
fn validate_rejects_invalid_and_duplicate_mail_names() {
    let (dir, output) = project("mail_validate_names");
    let toml = read(&output, "puerto.toml");
    fs::write(
        output.join("puerto.toml"),
        format!("{toml}\n[[correo]]\nnombre = \"Welcome\"\n\n[[correo]]\nnombre = \"Welcome\"\n"),
    )
    .unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(err.contains("nombre de correo inválido 'Welcome'"), "{err}");
    assert!(err.contains("el correo 'Welcome' está repetido"), "{err}");
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
            "nombre = \"welcome\"\n",
            "nombre = \"welcome\"\ntras = [\"saludo_creado\"]\n",
        ),
    )
    .unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(err.contains("correo 'welcome'"), "{err}");
    assert!(err.contains("proyecto.eventos no está puesto"), "{err}");
    cleanup(&dir);
}

#[test]
fn validate_rejects_a_consumer_named_like_the_consumer_of_a_mail() {
    let (dir, output) = project("mail_validate_consumer_clash");
    welcome(&output);
    let toml = read(&output, "puerto.toml");
    fs::write(
        output.join("puerto.toml"),
        format!(
            "{}\n[[consumidor]]\nnombre = \"correo_welcome\"\neventos = [\"saludo_creado\"]\n",
            // Only a mail sent on events has a worker consumer.
            toml.replace(
                "nombre = \"welcome\"\n",
                "nombre = \"welcome\"\ntras = [\"saludo_creado\"]\n"
            )
        ),
    )
    .unwrap();

    let err = error_of(validate::run_validate(&output));

    assert!(
        err.contains("el worker lo ejecuta como el consumidor 'correo_welcome'"),
        "{err}"
    );
    cleanup(&dir);
}

#[test]
fn list_shows_the_mails() {
    let (dir, output) = project("mail_list");
    welcome(&output);
    scaffold::run_generate_mail("receipt", &[], &output).unwrap();

    let report = list::list_report(&puerto_toml::read(&output).unwrap());

    assert!(
        report.contains("\nCorreos:\n  welcome\n  receipt\n"),
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

// ── Phase 14.4: `--on <event_type> --to <field>`, sent from the worker ────

/// An `events` project with a CRUD `Cliente` (no web).
fn cliente_project(name: &str, transport: &str) -> (PathBuf, PathBuf) {
    let (dir, output) = db_project(name);
    scaffold::apply_events_to_new_project(&output, transport).unwrap();
    let fields: Vec<puerto_toml::Field> = [
        "name:Name:String",
        "email:String",
        "seats:i64",
        "tags:vec:String",
    ]
    .iter()
    .map(|arg| puerto_toml::parse_field_arg(arg).unwrap())
    .collect();
    scaffold::run_scaffold("Cliente", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

fn on(events: &[&str]) -> Vec<String> {
    events.iter().map(|e| e.to_string()).collect()
}

fn welcome_on_created(output: &Path) {
    scaffold::run_generate_mail_on(
        "welcome",
        &[],
        &on(&["cliente_creado"]),
        Some("email"),
        output,
    )
    .unwrap();
}

#[test]
fn mail_on_an_event_gets_a_consumer_that_sends_it() {
    let (dir, output) = cliente_project("mail_on_consumer", "db");

    welcome_on_created(&output);

    let handler = read(
        &output,
        "presentacion/worker/src/consumidores/correo_welcome.rs",
    );
    assert!(
        handler.contains("pub struct EnviarCorreoWelcome {"),
        "{handler}"
    );
    assert!(
        handler.contains("pub enviador: Arc<dyn EnviadorCorreoWelcome>,"),
        "{handler}"
    );
    assert!(
        handler.contains("pub correos_enviados: Arc<dyn CorreosEnviados>,"),
        "{handler}"
    );
    assert!(handler.contains("CLIENTE_CREADO => match evento.decodificar::<SnapshotCliente>()"));
    assert!(
        handler.contains("para: cliente.email.clone(),"),
        "{handler}"
    );
    assert!(handler.contains("enviar_una_vez("), "{handler}");
    assert!(handler.contains("MOTIVO_PAYLOAD_INVALIDO"), "{handler}");
    assert!(handler.contains("MockEnviadorCorreoWelcome"), "{handler}");
    assert!(handler.contains("CorreosEnviadosEnMemoria"), "{handler}");

    let consumers = read(&output, "presentacion/worker/src/consumidores.rs");
    assert!(consumers.contains("pub mod correo_welcome;"), "{consumers}");
    let toml = read(&output, "puerto.toml");
    assert!(
        toml.contains("[[correo]]\nnombre = \"welcome\"\ntras = [\"cliente_creado\"]\n"),
        "{toml}"
    );
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_is_a_consumer_of_the_registry() {
    let (dir, output) = cliente_project("mail_on_registry", "db");

    welcome_on_created(&output);

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(
        registry.contains("use crate::consumidores::correo_welcome::EnviarCorreoWelcome;"),
        "{registry}"
    );
    assert!(registry.contains(".enrutar(\"correo_welcome\", &[CLIENTE_CREADO])"));
    assert!(registry.contains("ConfigCorreo::from_env()"), "{registry}");
    assert!(registry.contains("EnviadorCorreoSmtp::new("), "{registry}");
    assert!(
        registry.contains("CorreosEnviadosPostgres::new(pool.clone())"),
        "{registry}"
    );
    assert!(
        registry.contains("EnviadorCorreoWelcomePlantilla::new(enviador.clone())"),
        "{registry}"
    );
    assert!(
        registry.contains("SuscripcionPostgres::new(pool.clone(), \"correo_welcome\")"),
        "{registry}"
    );
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_is_a_kafka_consumer_on_the_kafka_transport() {
    let (dir, output) = cliente_project("mail_on_kafka", "kafka");

    welcome_on_created(&output);

    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(registry.contains("\"correo_welcome\","), "{registry}");
    assert!(registry.contains("&[\"cliente\"]"), "{registry}");
    assert!(registry.contains("EnviarCorreoWelcome {"), "{registry}");
    assert!(registry.contains("EnviadorCorreoSmtp::new("), "{registry}");
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_carries_the_snapshot_fields_a_template_prints() {
    let (dir, output) = cliente_project("mail_on_fields", "db");

    welcome_on_created(&output);

    let domain = read(&output, "contextos/correo/dominio/src/welcome.rs");
    assert!(domain.contains("pub name: String,"), "{domain}");
    assert!(domain.contains("pub seats: i64,"), "{domain}");
    // The recipient is `to`, not a field twice; a Vec is not printed.
    assert!(!domain.contains("pub email:"), "{domain}");
    assert!(!domain.contains("pub tags:"), "{domain}");
    let handler = read(
        &output,
        "presentacion/worker/src/consumidores/correo_welcome.rs",
    );
    assert!(handler.contains("name: cliente.name,"), "{handler}");
    assert!(handler.contains("seats: cliente.seats,"), "{handler}");
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_takes_only_fields_its_snapshot_has() {
    let (dir, output) = cliente_project("mail_on_named_fields", "db");

    scaffold::run_generate_mail_on(
        "welcome",
        &args(&["name:String"]),
        &on(&["cliente_creado"]),
        Some("email"),
        &output,
    )
    .unwrap();
    let domain = read(&output, "contextos/correo/dominio/src/welcome.rs");
    assert!(!domain.contains("pub seats:"), "{domain}");

    for (field, reason) in [("nickname:String", "nickname"), ("seats:String", "seats")] {
        let err = error_of(scaffold::run_generate_mail_on(
            "receipt",
            &args(&[field]),
            &on(&["cliente_creado"]),
            Some("email"),
            &output,
        ));
        assert!(err.contains(reason), "{field}: {err}");
        assert!(err.contains("cliente_creado"), "{field}: {err}");
    }
    cleanup(&dir);
}

#[test]
fn first_mail_on_an_event_adds_the_sent_log_and_puerto_mail_to_the_worker() {
    let (dir, output) = cliente_project("mail_on_setup", "db");

    welcome_on_created(&output);
    scaffold::run_generate_mail_on(
        "farewell",
        &[],
        &on(&["cliente_borrado"]),
        Some("email"),
        &output,
    )
    .unwrap();

    let migrations: Vec<String> = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|m| m.ends_with("_correos_enviados.sql"))
        .collect();
    assert_eq!(migrations.len(), 1, "{migrations:?}");
    let sql = read(&output, &format!("migraciones/{}", migrations[0]));
    assert!(sql.ends_with(include_str!("../templates/infrastructure/sent_mails.sql")));

    let worker = read(&output, "presentacion/worker/Cargo.toml");
    let (deps, dev) = worker.split_once("[dev-dependencies]").unwrap();
    assert_eq!(
        deps.matches("puerto-correo = { workspace = true }").count(),
        1
    );
    assert!(
        dev.contains("puerto-correo = { workspace = true, features = [\"pruebas\"] }"),
        "{worker}"
    );
    cleanup(&dir);
}

#[test]
fn mail_without_on_in_an_events_project_has_no_consumer() {
    let (dir, output) = cliente_project("mail_on_none", "db");

    welcome(&output);

    assert!(
        !output
            .join("presentacion/worker/src/consumidores/correo_welcome.rs")
            .exists()
    );
    let registry = read(&output, "presentacion/worker/src/generado/registro.rs");
    assert!(!registry.contains("EnviadorCorreoSmtp"), "{registry}");
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_is_refused_without_what_it_needs() {
    let (dir, output) = cliente_project("mail_on_refusals", "db");
    let try_mail = |on_events: &[&str], to: Option<&str>| {
        error_of(scaffold::run_generate_mail_on(
            "welcome",
            &[],
            &on(on_events),
            to,
            &output,
        ))
    };

    assert!(try_mail(&["cliente_creado"], None).contains("--para"));
    assert!(try_mail(&[], Some("email")).contains("--tras"));
    assert!(try_mail(&["cliente_archived"], Some("email")).contains("cliente_archived"));
    let not_string = try_mail(&["cliente_creado"], Some("seats"));
    assert!(not_string.contains("seats"), "{not_string}");
    assert!(not_string.contains("String"), "{not_string}");
    assert!(try_mail(&["cliente_creado"], Some("phone")).contains("phone"));
    assert!(!read(&output, "puerto.toml").contains("[[correo]]"));
    cleanup(&dir);
}

#[test]
fn mail_on_an_event_is_refused_in_a_project_without_events() {
    let (dir, output) = project("mail_on_no_events");

    let err = error_of(scaffold::run_generate_mail_on(
        "welcome",
        &[],
        &on(&["saludo_creado"]),
        Some("name"),
        &output,
    ));

    assert!(err.contains("--eventos"), "{err}");
    cleanup(&dir);
}

#[test]
fn mail_and_consumer_names_never_share_a_consumer() {
    let (dir, output) = cliente_project("mail_on_consumer_clash", "db");
    scaffold::run_generate_consumer("correo_welcome", &on(&["cliente_creado"]), &output).unwrap();

    let err = error_of(scaffold::run_generate_mail_on(
        "welcome",
        &[],
        &on(&["cliente_creado"]),
        Some("email"),
        &output,
    ));
    assert!(err.contains("correo_welcome"), "{err}");

    scaffold::run_generate_mail_on(
        "receipt",
        &[],
        &on(&["cliente_creado"]),
        Some("email"),
        &output,
    )
    .unwrap();
    let err = error_of(scaffold::run_generate_consumer(
        "correo_receipt",
        &on(&["cliente_creado"]),
        &output,
    ));
    assert!(err.contains("correo 'receipt'"), "{err}");
    cleanup(&dir);
}

#[test]
fn a_missing_mail_consumer_is_drift_for_validate_and_bootstrap() {
    let (dir, output) = cliente_project("mail_on_drift", "db");
    welcome_on_created(&output);
    fs::remove_file(output.join("presentacion/worker/src/consumidores/correo_welcome.rs")).unwrap();

    let validate = error_of(validate::run_validate(&output));
    let bootstrap = error_of(scaffold::run_bootstrap_command(&output, false));

    assert!(
        validate.contains("presentacion/worker/src/consumidores/correo_welcome.rs"),
        "{validate}"
    );
    assert!(
        bootstrap.contains("presentacion/worker/src/consumidores/correo_welcome.rs"),
        "{bootstrap}"
    );
    cleanup(&dir);
}
