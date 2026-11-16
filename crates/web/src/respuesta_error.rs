use poem::http::StatusCode;
use poem_openapi::{Object, payload::Json};

/// Every error body the API returns (ADR-0007).
///
/// `codigo` is the machine-readable code clients branch on — `producto.no_encontrado`,
/// `producto.validacion.titulo_vacio`. `mensaje` is for people and logs; for domain errors,
/// whose `Display` *is* the code, the two are equal.
#[derive(Object, Debug)]
pub struct RespuestaError {
    pub codigo: String,
    pub mensaje: String,
}

impl RespuestaError {
    /// An error body for a machine-readable code.
    pub fn from_codigo(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            mensaje: code.clone(),
            codigo: code,
        }
    }
}

/// Maps a domain error to its HTTP status and body. Validation failures are `422 Unprocessable
/// Entity`: the request was well-formed (poem already answers `400` for one that is not) but
/// breaks a domain rule.
pub trait IntoRespuestaError {
    fn into_respuesta_error(self) -> (StatusCode, Json<RespuestaError>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_carry_the_code_in_name_and_message_when_built_from_a_code() {
        // Act
        let body = RespuestaError::from_codigo("product.not_found");

        // Assert
        assert_eq!(body.codigo, "product.not_found");
        assert_eq!(body.mensaje, "product.not_found");
    }
}
