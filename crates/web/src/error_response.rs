use poem::http::StatusCode;
use poem_openapi::{Object, payload::Json};

/// Every error body the API returns (ADR-0007).
///
/// `name` is the machine-readable code clients branch on — `product.not_found`,
/// `product.validation_error.title_empty`. `message` is for people and logs; for domain errors,
/// whose `Display` *is* the code, the two are equal.
#[derive(Object, Debug)]
pub struct ErrorResponse {
    pub name: String,
    pub message: String,
}

impl ErrorResponse {
    /// An error body for a machine-readable code.
    pub fn from_code(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            message: code.clone(),
            name: code,
        }
    }
}

/// Maps a domain error to its HTTP status and body. Validation failures are `422 Unprocessable
/// Entity`: the request was well-formed (poem already answers `400` for one that is not) but
/// breaks a domain rule.
pub trait IntoErrorResponse {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_carry_the_code_in_name_and_message_when_built_from_a_code() {
        // Act
        let body = ErrorResponse::from_code("product.not_found");

        // Assert
        assert_eq!(body.name, "product.not_found");
        assert_eq!(body.message, "product.not_found");
    }
}
