use dominio_saludo::errores::ErrorSaludo;
use poem::http::StatusCode;
use poem_openapi::payload::Json;
use puerto_web::RespuestaError;

/// Traduce un error de saludo a su estado HTTP y su cuerpo (ADR-0007 de Puerto). Los fallos de
/// validación son `422 Unprocessable Entity`: la petición estaba bien formada (poem ya responde
/// `400` a una que no lo está), pero incumple una regla del dominio. Exhaustivo a propósito: una
/// variante de error nueva no compila hasta que tiene un estado.
pub fn a_respuesta_error(err: ErrorSaludo) -> (StatusCode, Json<RespuestaError>) {
    let status = match &err {
        ErrorSaludo::Validacion(_) => StatusCode::UNPROCESSABLE_ENTITY,
        ErrorSaludo::NoEncontrado => StatusCode::NOT_FOUND,
        ErrorSaludo::Repositorio => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(RespuestaError::from_codigo(err.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deberia_mapear_validacion_a_422_con_su_codigo() {
        // Actuar
        let (status, Json(cuerpo)) =
            a_respuesta_error(ErrorSaludo::Validacion("nombre_vacio".into()));

        // Comprobar
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(cuerpo.codigo, "saludo.validacion.nombre_vacio");
    }

    #[test]
    fn deberia_mapear_no_encontrado_a_404_con_su_codigo() {
        // Actuar
        let (status, Json(cuerpo)) = a_respuesta_error(ErrorSaludo::NoEncontrado);

        // Comprobar
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(cuerpo.codigo, "saludo.no_encontrado");
    }

    #[test]
    fn deberia_mapear_repositorio_a_500_con_su_codigo() {
        // Actuar
        let (status, Json(cuerpo)) = a_respuesta_error(ErrorSaludo::Repositorio);

        // Comprobar
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(cuerpo.codigo, "saludo.repositorio");
    }
}
