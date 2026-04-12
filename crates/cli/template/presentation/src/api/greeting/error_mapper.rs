use business::domain::greeting::errors::GreetingError;
use poem::http::StatusCode;
use poem_openapi::payload::Json;

use crate::api::error::{ErrorResponse, IntoErrorResponse};

impl IntoErrorResponse for GreetingError {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>) {
        let (status, message) = match &self {
            GreetingError::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            GreetingError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            GreetingError::RepositoryError => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
        };

        (
            status,
            Json(ErrorResponse {
                name: "GreetingError".into(),
                message,
            }),
        )
    }
}
