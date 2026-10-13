// Phase 16.8 end to end: a generated auth project's API as a real process. OIDC (`make test/auth`):
// the project's own realm imported into a throwaway Keycloak, tokens from `make auth/token` (its
// `dev-cli` client), `admin@` the caller with the role and `user@` the one without; the first call
// provisions one `users` row, later ones (a restart included) add none. Local (`make test/db`):
// register → login → `/me` → logout, and the revoked token is refused.
use super::*;

use std::process::Command;

use super::compile_matrix::{AUTH_LOCAL, AUTH_OIDC_DB};
use super::worker_e2e::{Project, build_scenario_project, free_port, http, http_as, wait_until};

const PRODUCT: &str = r#"{"name":"Desk lamp","sku":"LAMP-1","price":2500,"status":"Draft"}"#;

/// Whether `body` is the error `code` (`ErrorResponse { name, message }`).
fn is_error(body: &str, code: &str) -> bool {
    body.contains(&format!("\"name\":\"{code}\""))
}

fn database_url(project: &Project) -> String {
    project
        .env
        .iter()
        .find(|(k, _)| k == "DATABASE_URL")
        .map(|(_, v)| v.clone())
        .expect("no DATABASE_URL")
}

/// How many `users` rows the project's database holds.
fn users(project: &Project) -> i64 {
    let url = database_url(project);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let pool = sqlx::PgPool::connect(&url).await.unwrap();
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap();
            pool.close().await;
            count
        })
}

/// Starts the project's API on a free port and waits until it is ready.
fn start_api(
    project: &Project,
    binary: &str,
    log: &str,
    env: &[(&str, String)],
) -> (super::worker_e2e::Running, u16) {
    let port = free_port();
    let mut env = env.to_vec();
    env.push(("SERVICE_PORT", port.to_string()));
    let api = project.spawn(binary, &project.dir.join(log), &env);
    wait_until("the API to be ready", || {
        http(port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });
    (api, port)
}

// ── OIDC ──────────────────────────────────────────────────────────────────

const OIDC_PROJECT: &str = "e2e-auth-oidc";

/// `curl` with `args`; its stdout.
fn curl(args: &[&str]) -> String {
    let out = Command::new("curl")
        .args(["-sS", "--fail-with-body"])
        .args(args)
        .output()
        .expect("curl not found");
    let body = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "curl {args:?}: {body}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    body
}

/// Imports the generated `infrastructure/keycloak/realm.json` into the test Keycloak, as the
/// project's `docker-compose.yml` does on its first start — replacing one an earlier run left.
fn import_realm(keycloak: &str, output: &Path) {
    let token = curl(&[
        &format!("{keycloak}/realms/master/protocol/openid-connect/token"),
        "-d",
        "grant_type=password",
        "-d",
        "client_id=admin-cli",
        "-d",
        "username=admin",
        "-d",
        "password=admin",
    ]);
    let token: serde_json::Value = serde_json::from_str(&token).unwrap();
    let bearer = format!(
        "Authorization: Bearer {}",
        token["access_token"].as_str().unwrap()
    );
    let _ = Command::new("curl")
        .args(["-s", "-X", "DELETE", "-H", &bearer])
        .arg(format!("{keycloak}/admin/realms/{OIDC_PROJECT}"))
        .output();
    let realm = output.join("infrastructure/keycloak/realm.json");
    curl(&[
        "-X",
        "POST",
        "-H",
        &bearer,
        "-H",
        "Content-Type: application/json",
        "--data-binary",
        &format!("@{}", realm.display()),
        &format!("{keycloak}/admin/realms"),
    ]);
}

/// `make -s auth/token EMAIL=<email>@…` in the project: what a developer runs.
fn token(project: &Project, issuer: &str, email: &str) -> String {
    let out = Command::new("make")
        .args(["-s", "auth/token"])
        .arg(format!("EMAIL={email}@{OIDC_PROJECT}.localhost"))
        .arg(format!("AUTH_ISSUER={issuer}"))
        .current_dir(&project.output)
        .output()
        .expect("make not found");
    assert!(
        out.status.success(),
        "make auth/token EMAIL={email}@…: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let token = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(token.split('.').count(), 3, "not a JWT: {token}");
    token
}

#[test]
#[ignore = "slow: needs Keycloak and Postgres — run via `make test/auth`"]
fn keycloak_oidc_project_refuses_without_token_or_role_and_provisions_each_caller_once() {
    let Some(keycloak) = std::env::var("PUERTO_TEST_KEYCLOAK_URL").ok() else {
        eprintln!("PUERTO_TEST_KEYCLOAK_URL not set — skipping (run `make test/auth`)");
        return;
    };
    let keycloak = keycloak.trim_end_matches('/').to_string();
    let issuer = format!("{keycloak}/realms/{OIDC_PROJECT}");
    let env = [
        ("AUTH_ISSUER", issuer.clone()),
        ("AUTH_AUDIENCE", "api".to_string()),
        ("AUTH_ROLES_CLAIM", "realm_access.roles".to_string()),
    ];
    let Some(project) = build_scenario_project(
        &AUTH_OIDC_DB,
        OIDC_PROJECT,
        "puerto_matrix_auth_oidc_e2e",
        &env,
        |_| {},
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/auth`)");
        return;
    };
    import_realm(&keycloak, &project.output);
    let admin = token(&project, &issuer, "admin");
    let user = token(&project, &issuer, "user");

    let (api, port) = start_api(&project, OIDC_PROJECT, "api.log", &[]);

    // No token: a public route answers, a protected one is 401.
    let (status, _) = http(port, "GET", "/api/products", "").unwrap();
    assert_eq!(status, 200, "a public route needs no token");
    let (status, body) = http(port, "GET", "/api/orders", "").unwrap();
    assert_eq!(status, 401, "{body}");
    assert!(is_error(&body, "auth.unauthenticated"), "{body}");
    let (status, body) = http_as(port, "GET", "/api/orders", "", Some("not.a.token")).unwrap();
    assert_eq!(status, 401, "{body}");
    assert_eq!(users(&project), 0, "nobody signed in yet");

    // The first call provisions the caller; a second one does not add another row.
    let (status, body) = http_as(port, "GET", "/api/me", "", Some(&admin)).unwrap();
    assert_eq!(status, 200, "{body}");
    assert!(
        body.contains(&format!("admin@{OIDC_PROJECT}.localhost")),
        "{body}"
    );
    assert!(
        body.contains("\"admin\""),
        "the role from the token: {body}"
    );
    assert_eq!(users(&project), 1);
    let (status, body) = http_as(port, "POST", "/api/products", PRODUCT, Some(&admin)).unwrap();
    assert_eq!(status, 201, "admin holds `admin`: {body}");
    assert_eq!(users(&project), 1);

    // The role: `user@` holds none — 403 on a role-guarded route, 200 on a public one and on a
    // protected one that asks no role.
    let (status, body) = http_as(port, "POST", "/api/products", PRODUCT, Some(&user)).unwrap();
    assert_eq!(status, 403, "{body}");
    assert!(is_error(&body, "auth.forbidden"), "{body}");
    let (status, _) = http_as(port, "GET", "/api/products", "", Some(&user)).unwrap();
    assert_eq!(status, 200);
    let (status, body) = http_as(port, "GET", "/api/orders", "", Some(&user)).unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(users(&project), 2, "user@ provisioned on its first call");

    // A restarted API has no cache: it finds the rows instead of adding them.
    drop(api);
    let (api, port) = start_api(&project, OIDC_PROJECT, "api-restarted.log", &[]);
    let (status, body) = http_as(port, "GET", "/api/me", "", Some(&admin)).unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(users(&project), 2);

    drop(api);
    cleanup(&project.dir);
}

// ── Local ─────────────────────────────────────────────────────────────────

const LOCAL_PROJECT: &str = "e2e-auth-local";

/// The `"<field>":"<value>"` of a JSON body.
fn field(body: &str, name: &str) -> String {
    let json: serde_json::Value =
        serde_json::from_str(body).unwrap_or_else(|_| panic!("not JSON: {body}"));
    json[name]
        .as_str()
        .unwrap_or_else(|| panic!("no {name}: {body}"))
        .to_string()
}

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_local_auth_project_registers_signs_in_and_revokes_on_logout() {
    let Some(project) = build_scenario_project(
        &AUTH_LOCAL,
        LOCAL_PROJECT,
        "puerto_matrix_auth_local_e2e",
        &[],
        |_| {},
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let (api, port) = start_api(&project, LOCAL_PROJECT, "api.log", &[]);

    // No token: a public route answers, a protected one is 401.
    let (status, _) = http(port, "GET", "/api/products", "").unwrap();
    assert_eq!(status, 200);
    let (status, body) = http(port, "GET", "/api/orders", "").unwrap();
    assert_eq!(status, 401, "{body}");
    assert!(is_error(&body, "auth.unauthenticated"), "{body}");

    // Register, then sign in.
    let account = r#"{"email":"Ada@Example.com","password":"correct horse battery","name":"Ada"}"#;
    let (status, body) = http(port, "POST", "/api/auth/register", account).unwrap();
    assert_eq!(status, 201, "{body}");
    let (status, body) = http(port, "POST", "/api/auth/register", account).unwrap();
    assert_eq!(status, 409, "the email is taken: {body}");
    let wrong = r#"{"email":"ada@example.com","password":"not the password"}"#;
    let (status, body) = http(port, "POST", "/api/auth/login", wrong).unwrap();
    assert_eq!(status, 401, "{body}");
    assert!(is_error(&body, "auth.invalid_credentials"), "{body}");
    let login = r#"{"email":"ada@example.com","password":"correct horse battery"}"#;
    let (status, body) = http(port, "POST", "/api/auth/login", login).unwrap();
    assert_eq!(status, 200, "{body}");
    let token = field(&body, "token");

    // The session: `/me`, a protected route, and no role for a role-guarded one.
    let (status, body) = http_as(port, "GET", "/api/me", "", Some(&token)).unwrap();
    assert_eq!(status, 200, "{body}");
    assert_eq!(field(&body, "email"), "ada@example.com");
    let (status, body) = http_as(port, "GET", "/api/orders", "", Some(&token)).unwrap();
    assert_eq!(status, 200, "{body}");
    let (status, body) = http_as(port, "POST", "/api/products", PRODUCT, Some(&token)).unwrap();
    assert_eq!(status, 403, "{body}");
    assert!(is_error(&body, "auth.forbidden"), "{body}");
    assert_eq!(users(&project), 1);

    // Logout revokes the token it was called with.
    let (status, body) = http_as(port, "POST", "/api/auth/logout", "", Some(&token)).unwrap();
    assert_eq!(status, 204, "{body}");
    let (status, body) = http_as(port, "GET", "/api/me", "", Some(&token)).unwrap();
    assert_eq!(status, 401, "a revoked session: {body}");
    assert!(is_error(&body, "auth.unauthenticated"), "{body}");

    drop(api);
    cleanup(&project.dir);
}
