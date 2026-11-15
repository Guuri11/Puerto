use dominio_greeting::errores::GreetingError;
use poem::http::StatusCode;
use poem_openapi::payload::Json;
use puerto_web::RespuestaError;

/// Maps a greeting error to its HTTP status and body (ADR-0007 of Puerto). Validation failures are
/// `422 Unprocessable Entity`: the request was well-formed (poem already answers `400` for one that
/// is not) but breaks a domain rule. Exhaustive on purpose: a new error variant does not compile
/// until it has a status.
pub fn a_respuesta_error(err: GreetingError) -> (StatusCode, Json<RespuestaError>) {
    let status = match &err {
        GreetingError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
        GreetingError::NotFound => StatusCode::NOT_FOUND,
        GreetingError::RepositoryError => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(RespuestaError::from_codigo(err.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_validation_error_to_422_with_its_code() {
        // Act
        let (status, Json(body)) =
            a_respuesta_error(GreetingError::ValidationError("name_empty".into()));

        // Assert
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body.codigo, "greeting.validation_error.name_empty");
    }

    #[test]
    fn should_map_not_found_to_404_with_its_code() {
        // Act
        let (status, Json(body)) = a_respuesta_error(GreetingError::NotFound);

        // Assert
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.codigo, "greeting.not_found");
    }

    #[test]
    fn should_map_repository_error_to_500_with_its_code() {
        // Act
        let (status, Json(body)) = a_respuesta_error(GreetingError::RepositoryError);

        // Assert
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.codigo, "greeting.repository_error");
    }
}
