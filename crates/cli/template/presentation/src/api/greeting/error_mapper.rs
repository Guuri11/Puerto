use business::domain::greeting::errors::GreetingError;
use poem::http::StatusCode;
use poem_openapi::payload::Json;
use puerto_web::ErrorResponse;

use crate::api::error::IntoErrorResponse;

impl IntoErrorResponse for GreetingError {
    /// Exhaustive on purpose: a new error variant does not compile until it has a status.
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>) {
        let status = match &self {
            GreetingError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            GreetingError::NotFound => StatusCode::NOT_FOUND,
            GreetingError::RepositoryError => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(ErrorResponse::from_code(self.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_validation_error_to_422_with_its_code() {
        // Act
        let (status, Json(body)) =
            GreetingError::ValidationError("name_empty".into()).into_error_response();

        // Assert
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.name, "greeting.validation_error.name_empty");
    }

    #[test]
    fn should_map_not_found_to_404_with_its_code() {
        // Act
        let (status, Json(body)) = GreetingError::NotFound.into_error_response();

        // Assert
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.name, "greeting.not_found");
    }

    #[test]
    fn should_map_repository_error_to_500_with_its_code() {
        // Act
        let (status, Json(body)) = GreetingError::RepositoryError.into_error_response();

        // Assert
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.name, "greeting.repository_error");
    }
}
