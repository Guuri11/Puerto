//! `BearerAuth` and `#[require_role]` as a generated project uses them: inside an `#[OpenApi] impl`,
//! with the `Authenticator` registered on the routes by `.data(…)`, as `bootstrap.rs` does.

use std::sync::Arc;

use poem::{EndpointExt, Route, http::StatusCode, test::TestClient};
use poem_openapi::{ApiResponse, OpenApi, OpenApiService, payload::Json};
use puerto_core::auth::mocks::MockAuthenticator;
use puerto_core::{AuthError, Authenticator, Principal};
use puerto_web::{ErrorResponse, auth::BearerAuth, require_role};
use uuid::Uuid;

#[derive(ApiResponse)]
enum WhoResponse {
    #[oai(status = 200)]
    Ok(Json<String>),
    #[oai(status = 403)]
    Forbidden(Json<ErrorResponse>),
}

impl WhoResponse {
    fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::FORBIDDEN => Self::Forbidden(error),
            _ => unreachable!("only 403 in these tests"),
        }
    }
}

struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/who", method = "get")]
    async fn who(&self, auth: BearerAuth) -> WhoResponse {
        WhoResponse::Ok(Json(auth.0.subject))
    }

    #[oai(path = "/admin", method = "get")]
    #[require_role("admin")]
    async fn admin(&self, auth: BearerAuth) -> WhoResponse {
        WhoResponse::Ok(Json(auth.0.subject))
    }

    #[require_role("admin")]
    #[oai(path = "/admin-first", method = "get")]
    async fn admin_first(&self, auth: BearerAuth) -> WhoResponse {
        WhoResponse::Ok(Json(auth.0.subject))
    }

    #[oai(path = "/open", method = "get")]
    async fn open(&self) -> Json<String> {
        Json("anyone".into())
    }
}

/// The mock accepts `ada` (an admin) and `bob` (no role), and answers `expired` / `down` with the
/// matching error; any other token is invalid.
fn authenticator() -> Arc<dyn Authenticator> {
    let mut mock = MockAuthenticator::new();
    mock.expect_authenticate().returning(|token| match token {
        "ada" => Ok(Principal::new(Uuid::nil(), "ada").with_roles(["admin"])),
        "bob" => Ok(Principal::new(Uuid::nil(), "bob")),
        "expired" => Err(AuthError::Expired),
        "down" => Err(AuthError::Unavailable),
        _ => Err(AuthError::Invalid),
    });
    Arc::new(mock)
}

fn service() -> OpenApiService<Api, ()> {
    OpenApiService::new(Api, "auth", "1.0")
}

fn client() -> TestClient<impl poem::Endpoint> {
    TestClient::new(Route::new().nest("/", service()).data(authenticator()))
}

async fn assert_error(response: poem::test::TestResponse, status: StatusCode, code: &str) {
    response.assert_status(status);
    let body = response.json().await;
    body.value().object().get("name").assert_string(code);
    body.value().object().get("message").assert_string(code);
}

#[tokio::test]
async fn should_answer_the_caller_when_the_token_is_accepted() {
    // Act
    let response = client()
        .get("/who")
        .header("Authorization", "Bearer bob")
        .send()
        .await;

    // Assert
    response.assert_status_is_ok();
    response.assert_json("bob").await;
}

#[tokio::test]
async fn should_answer_401_unauthenticated_when_there_is_no_token() {
    // Act
    let response = client().get("/who").send().await;

    // Assert
    assert_error(response, StatusCode::UNAUTHORIZED, "auth.unauthenticated").await;
}

#[tokio::test]
async fn should_answer_401_unauthenticated_when_the_header_is_not_bearer() {
    // Act
    let response = client()
        .get("/who")
        .header("Authorization", "Basic YWRhOmFkYQ==")
        .send()
        .await;

    // Assert
    assert_error(response, StatusCode::UNAUTHORIZED, "auth.unauthenticated").await;
}

#[tokio::test]
async fn should_answer_401_unauthenticated_when_the_authenticator_rejects_the_token() {
    for token in ["forged", "expired"] {
        // Act
        let response = client()
            .get("/who")
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await;

        // Assert
        assert_error(response, StatusCode::UNAUTHORIZED, "auth.unauthenticated").await;
    }
}

#[tokio::test]
async fn should_answer_503_unavailable_when_the_token_cannot_be_checked() {
    // Act
    let response = client()
        .get("/who")
        .header("Authorization", "Bearer down")
        .send()
        .await;

    // Assert
    assert_error(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "auth.unavailable",
    )
    .await;
}

#[tokio::test]
async fn should_answer_500_not_configured_when_no_authenticator_is_registered() {
    // Arrange
    let client = TestClient::new(Route::new().nest("/", service()));

    // Act
    let response = client
        .get("/who")
        .header("Authorization", "Bearer ada")
        .send()
        .await;

    // Assert
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.json().await;
    body.value()
        .object()
        .get("name")
        .assert_string("auth.not_configured");
    let message = body.value().object().get("message").string().to_string();
    assert!(message.contains("bootstrap.rs"), "{message}");
}

#[tokio::test]
async fn should_answer_403_forbidden_when_the_caller_lacks_the_role() {
    for path in ["/admin", "/admin-first"] {
        // Act
        let response = client()
            .get(path)
            .header("Authorization", "Bearer bob")
            .send()
            .await;

        // Assert
        assert_error(response, StatusCode::FORBIDDEN, "auth.forbidden").await;
    }
}

#[tokio::test]
async fn should_run_the_handler_when_the_caller_holds_the_role() {
    for path in ["/admin", "/admin-first"] {
        // Act
        let response = client()
            .get(path)
            .header("Authorization", "Bearer ada")
            .send()
            .await;

        // Assert
        response.assert_status_is_ok();
        response.assert_json("ada").await;
    }
}

#[tokio::test]
async fn should_answer_401_before_the_role_check_when_there_is_no_token() {
    // Act
    let response = client().get("/admin").send().await;

    // Assert
    assert_error(response, StatusCode::UNAUTHORIZED, "auth.unauthenticated").await;
}

#[tokio::test]
async fn should_leave_a_route_without_bearer_auth_open() {
    // Act
    let response = client().get("/open").send().await;

    // Assert
    response.assert_status_is_ok();
}

#[test]
fn should_list_the_bearer_scheme_on_protected_routes_only_when_the_spec_is_built() {
    // Act
    let spec: serde_json::Value = serde_json::from_str(&service().spec()).expect("spec is JSON");

    // Assert
    let bearer = serde_json::json!([{ "BearerAuth": [] }]);
    for path in ["/who", "/admin", "/admin-first"] {
        assert_eq!(spec["paths"][path]["get"]["security"], bearer, "{path}");
    }
    assert!(spec["paths"]["/open"]["get"].get("security").is_none());
    let scheme = &spec["components"]["securitySchemes"]["BearerAuth"];
    assert_eq!(scheme["type"], "http");
    assert_eq!(scheme["scheme"], "bearer");
    assert_eq!(scheme["bearerFormat"], "opaque or JWT");
}
