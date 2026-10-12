use super::*;
use crate::commands::new::{NewOptions, new_project};
use crate::generators::auth::AGENTS_AUTH_SECTION;

// ── Phase 16.4: `puerto new --auth oidc` ─────────────────────────────────────

fn new_auth_project(name: &str, options: NewOptions) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            auth: Some("oidc".into()),
            ..options
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

fn new_auth_project_without_auth(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project(
        Some(name.replace('_', "-")),
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();
    (dir, output)
}

fn read(output: &Path, file: &str) -> String {
    fs::read_to_string(output.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn new_with_auth_records_the_mode_and_keeps_the_demo_public() {
    let (dir, output) = new_auth_project(
        "auth_new_manifest",
        NewOptions {
            no_db: true,
            ..Default::default()
        },
    );

    let config = puerto_toml::read(&output).unwrap();
    assert_eq!(config.project.auth.as_deref(), Some("oidc"));
    let greeting = config.entity.iter().find(|e| e.name == "Greeting").unwrap();
    assert_eq!(greeting.public, vec!["get_greeting"]);
    // What it wrote passes its own checks: the demo's handler takes no BearerAuth, and `user`
    // is not an orphan entity.
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn new_with_auth_writes_the_user_the_provisioning_and_me() {
    let (dir, output) = new_auth_project(
        "auth_new_files",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    for file in [
        "business/src/domain/user/model.rs",
        "business/src/domain/user/errors.rs",
        "business/src/domain/user/repository.rs",
        "infrastructure/src/user/repository.rs",
        "infrastructure/src/user/provisioning.rs",
        "infrastructure/keycloak/realm.json",
        "presentation/src/api/me.rs",
        "presentation/src/api/me/dto.rs",
        "presentation/src/api/me/routes.rs",
        "docker-compose.yml",
    ] {
        assert!(output.join(file).exists(), "{file}");
    }
    assert!(read(&output, "business/src/lib.rs").contains("pub mod user {"));
    assert!(
        read(&output, "infrastructure/src/user/repository.rs").contains("InMemoryUserRepository")
    );
    assert!(read(&output, "presentation/src/api.rs").contains("pub mod me;"));
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(
        bootstrap.contains("OidcAuthenticator::discover"),
        "{bootstrap}"
    );
    assert!(
        bootstrap.contains("InMemoryUserRepository::new()"),
        "{bootstrap}"
    );
    assert!(bootstrap.contains(".data(authenticator)"), "{bootstrap}");
    assert!(bootstrap.contains("me_api"), "{bootstrap}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_and_db_keeps_users_in_postgres() {
    let (dir, output) = new_auth_project(
        "auth_new_db",
        NewOptions {
            db: true,
            no_web: true,
            no_demo: true,
            ..Default::default()
        },
    );

    assert!(read(&output, "infrastructure/src/user/repository.rs").contains("PgUserRepository"));
    let migrations: Vec<String> = fs::read_dir(output.join("infrastructure/migrations"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let users = migrations
        .iter()
        .find(|m| m.ends_with("_create_users_table.sql"))
        .unwrap_or_else(|| panic!("{migrations:?}"));
    let sql = read(&output, &format!("infrastructure/migrations/{users}"));
    assert!(sql.contains("UNIQUE (issuer, subject)"), "{sql}");
    // No entity uses the database yet: the bootstrap still opens the pool, for the users.
    let bootstrap = read(&output, "presentation/src/generated/bootstrap.rs");
    assert!(bootstrap.contains("create_postgres_pool"), "{bootstrap}");
    assert!(
        bootstrap.contains("PgUserRepository::new(pool.clone())"),
        "{bootstrap}"
    );
    // Postgres keeps its service; Keycloak joins it before the volumes.
    let compose = read(&output, "docker-compose.yml");
    let keycloak = compose.find("  keycloak:").unwrap();
    assert!(compose.find("  postgres:").unwrap() < keycloak);
    assert!(keycloak < compose.find("\nvolumes:").unwrap(), "{compose}");
    cleanup(&dir);
}

#[test]
fn new_with_auth_points_env_and_make_at_the_development_provider() {
    let (dir, output) = new_auth_project(
        "auth_new_env",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    for env in [".env", ".env.example"] {
        let src = read(&output, env);
        assert!(
            src.contains("SERVICE_PORT=8080"),
            "{env}: the base vars stay"
        );
        assert!(
            src.contains("AUTH_ISSUER=http://localhost:8180/realms/auth-new-env\n"),
            "{env}: {src}"
        );
        assert!(src.contains("AUTH_AUDIENCE=api\n"), "{env}");
        assert!(
            src.contains("AUTH_ROLES_CLAIM=realm_access.roles\n"),
            "{env}"
        );
    }
    let makefile = read(&output, "Makefile");
    assert!(makefile.contains("auth/ui: docker-compose/up"));
    assert!(makefile.contains("docker-compose/up: ##"));
    assert!(makefile.contains("\nrun: docker-compose/up\n"));
    let realm = read(&output, "infrastructure/keycloak/realm.json");
    let realm: serde_json::Value = serde_json::from_str(&realm).expect("realm.json is JSON");
    assert_eq!(realm["realm"], "auth-new-env");
    assert_eq!(realm["users"][0]["email"], "admin@auth-new-env.localhost");
    assert_eq!(realm["clients"][0]["clientId"], "web");
    cleanup(&dir);
}

#[test]
fn new_with_auth_adds_the_auth_section_to_agents_md() {
    let (dir, output) = new_auth_project(
        "auth_new_agents",
        NewOptions {
            no_db: true,
            no_web: true,
            ..Default::default()
        },
    );

    let agents = read(&output, "AGENTS.md");
    assert!(agents.contains(AGENTS_AUTH_SECTION));
    assert!(agents.find("## Auth").unwrap() < agents.find("## Conventions").unwrap());
    // What the section names exists in the project it describes (ADR-0010).
    for path in [
        "business/src/domain/user",
        "infrastructure/src/user/provisioning.rs",
    ] {
        assert!(AGENTS_AUTH_SECTION.contains(path), "{path}");
        assert!(output.join(path).exists(), "{path}");
    }
    assert!(read(&output, "Makefile").contains("auth/ui:"));
    cleanup(&dir);
}

#[test]
fn new_without_auth_writes_none_of_it() {
    let (dir, output) = new_auth_project_without_auth("auth_none");

    assert!(!output.join("business/src/domain/user").exists());
    assert!(!output.join("presentation/src/api/me.rs").exists());
    assert!(!read(&output, "AGENTS.md").contains("## Auth"));
    assert!(!read(&output, "Cargo.toml").contains("puerto-auth"));
    cleanup(&dir);
}

#[test]
fn new_refuses_local_auth_until_it_ships() {
    let dir = temp_dir("auth_local");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let result = new_project(
        Some("auth-local".into()),
        NewOptions {
            db: true,
            auth: Some("local".into()),
            ..Default::default()
        },
        Some(dir.clone()),
    );

    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not available yet")
    );
    assert!(!dir.join("auth-local").exists(), "nothing is written");
    cleanup(&dir);
}
