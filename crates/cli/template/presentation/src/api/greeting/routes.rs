use std::sync::Arc;

use business::domain::greeting::use_cases::get_greeting::{
    GetGreetingParams, GetGreetingUseCaseTrait,
};
use poem_openapi::{OpenApi, param::Path};

use crate::api::{
    error::IntoErrorResponse,
    greeting::{dto::GreetingDto, responses::GetGreetingResponse},
    tags::ApiTags,
};

/// Holds the use case port; `generated/bootstrap.rs` plugs in the implementation.
pub struct GreetingApi {
    pub get_greeting: Arc<dyn GetGreetingUseCaseTrait>,
}

#[OpenApi]
impl GreetingApi {
    /// Get a greeting for the given name
    #[oai(path = "/greetings/:name", method = "get", tag = "ApiTags::Greeting")]
    async fn get_greeting(&self, name: Path<String>) -> GetGreetingResponse {
        match self
            .get_greeting
            .execute(GetGreetingParams { name: name.0 })
            .await
        {
            Ok(greeting) => GetGreetingResponse::Ok(poem_openapi::payload::Json(
                GreetingDto::from_greeting(&greeting),
            )),
            Err(err) => {
                let (status, error) = err.into_error_response();
                tracing::warn!(code = %error.0.name, "get_greeting failed");
                GetGreetingResponse::from_status(status, error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use business::domain::greeting::{
        errors::GreetingError, model::Greeting,
        use_cases::get_greeting::mocks::MockGetGreetingUseCase,
    };
    use poem::{Route, http::StatusCode, test::TestClient};
    use poem_openapi::OpenApiService;

    use super::*;

    fn client(use_case: MockGetGreetingUseCase) -> TestClient<Route> {
        let api = GreetingApi {
            get_greeting: Arc::new(use_case),
        };
        TestClient::new(Route::new().nest("/", OpenApiService::new(api, "test", "1.0")))
    }

    #[tokio::test]
    async fn should_respond_200_with_the_greeting_when_name_is_valid() {
        // Arrange
        let mut use_case = MockGetGreetingUseCase::new();
        use_case
            .expect_execute()
            .returning(|params| Greeting::new(&params.name));

        // Act
        let response = client(use_case).get("/greetings/Alice").send().await;

        // Assert
        response.assert_status(StatusCode::OK);
        response
            .json()
            .await
            .value()
            .object()
            .get("name")
            .assert_string("Alice");
    }

    #[tokio::test]
    async fn should_respond_422_when_the_greeting_is_rejected() {
        // Arrange
        let mut use_case = MockGetGreetingUseCase::new();
        use_case
            .expect_execute()
            .returning(|_| Err(GreetingError::ValidationError("name_empty".into())));

        // Act
        let response = client(use_case).get("/greetings/Alice").send().await;

        // Assert
        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
}
