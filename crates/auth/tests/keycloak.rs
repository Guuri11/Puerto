//! `OidcAuthenticator` against a real provider: a throwaway Keycloak started by `make test/auth`,
//! with the realms `puerto` and `other` imported from `fixtures/keycloak/` (a public client `test`
//! with password grant and an audience mapper adding `api`; the user `ada` with the role `admin`).

use puerto_auth::{AuthError, Authenticator, OidcAuthenticator, OidcConfig};
use serde_json::Value;

const SKIP: &str = "PUERTO_TEST_KEYCLOAK_URL not set — skipping (run `make test/auth`)";

/// Keycloak's base URL, when `make test/auth` started one.
fn keycloak() -> Option<String> {
    std::env::var("PUERTO_TEST_KEYCLOAK_URL").ok()
}

/// An access token for `ada` from `realm`, by password grant.
async fn token(keycloak: &str, realm: &str) -> String {
    let response: Value = reqwest::Client::new()
        .post(format!(
            "{keycloak}/realms/{realm}/protocol/openid-connect/token"
        ))
        .form(&[
            ("grant_type", "password"),
            ("client_id", "test"),
            ("username", "ada"),
            ("password", "ada-test-password"),
            ("scope", "openid"),
        ])
        .send()
        .await
        .expect("Keycloak answers")
        .json()
        .await
        .expect("a token response");
    response["access_token"]
        .as_str()
        .unwrap_or_else(|| panic!("no access_token in {response}"))
        .to_string()
}

async fn authenticator(keycloak: &str) -> OidcAuthenticator {
    OidcAuthenticator::discover(OidcConfig::new(format!("{keycloak}/realms/puerto"), "api"))
        .await
        .expect("Keycloak's realm is discovered")
}

#[tokio::test]
#[ignore = "needs Keycloak: make test/auth"]
async fn should_describe_the_caller_when_keycloak_issued_the_token() {
    let Some(keycloak) = keycloak() else {
        return eprintln!("{SKIP}");
    };
    // Arrange
    let authenticator = authenticator(&keycloak).await;
    let token = token(&keycloak, "puerto").await;

    // Act
    let principal = authenticator.authenticate(&token).await.expect("accepted");

    // Assert
    assert!(!principal.subject.is_empty());
    assert_eq!(principal.email.as_deref(), Some("ada@puerto.localhost"));
    assert_eq!(principal.name.as_deref(), Some("Ada Lovelace"));
    assert!(principal.has_role("admin"), "{:?}", principal.roles);
}

#[tokio::test]
#[ignore = "needs Keycloak: make test/auth"]
async fn should_reject_the_token_when_another_realm_issued_it() {
    let Some(keycloak) = keycloak() else {
        return eprintln!("{SKIP}");
    };
    // Arrange
    let authenticator = authenticator(&keycloak).await;
    let token = token(&keycloak, "other").await;

    // Act
    let result = authenticator.authenticate(&token).await;

    // Assert
    assert_eq!(result.err(), Some(AuthError::Invalid));
}
