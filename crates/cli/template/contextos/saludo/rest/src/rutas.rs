use std::sync::Arc;

use aplicacion_saludo::obtener_saludo::{ParamsObtenerSaludo, ObtenerSaludo};
use poem_openapi::{OpenApi, Tags, param::Path};

use crate::{dto::DtoSaludo, errores::a_respuesta_error, respuestas::RespuestaObtenerSaludo};

#[derive(Tags)]
enum ApiTags {
    Saludo,
}

/// Guarda el puerto del caso de uso; `generado/bootstrap.rs` conecta la implementación.
pub struct ApiSaludo {
    pub obtener_saludo: Arc<dyn ObtenerSaludo>,
}

#[OpenApi]
impl ApiSaludo {
    /// Obtiene un saludo para el nombre dado
    #[oai(path = "/saludos/:nombre", method = "get", tag = "ApiTags::Saludo")]
    async fn obtener_saludo(&self, nombre: Path<String>) -> RespuestaObtenerSaludo {
        match self
            .obtener_saludo
            .ejecutar(ParamsObtenerSaludo { nombre: nombre.0 })
            .await
        {
            Ok(saludo) => RespuestaObtenerSaludo::Ok(poem_openapi::payload::Json(
                DtoSaludo::from_dominio(&saludo),
            )),
            Err(err) => {
                let (status, error) = a_respuesta_error(err);
                tracing::warn!(codigo = %error.0.codigo, "obtener_saludo falló");
                RespuestaObtenerSaludo::from_status(status, error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use aplicacion_saludo::obtener_saludo::dobles::MockObtenerSaludo;
    use dominio_saludo::{errores::ErrorSaludo, modelo::Saludo};
    use poem::{Route, http::StatusCode, test::TestClient};
    use poem_openapi::OpenApiService;

    use super::*;

    fn cliente(caso_de_uso: MockObtenerSaludo) -> TestClient<Route> {
        let api = ApiSaludo {
            obtener_saludo: Arc::new(caso_de_uso),
        };
        TestClient::new(Route::new().nest("/", OpenApiService::new(api, "test", "1.0")))
    }

    #[tokio::test]
    async fn deberia_responder_200_con_el_saludo_cuando_el_nombre_es_valido() {
        // Preparar
        let mut caso_de_uso = MockObtenerSaludo::new();
        caso_de_uso
            .expect_ejecutar()
            .returning(|params| Saludo::new(&params.nombre));

        // Actuar
        let respuesta = cliente(caso_de_uso).get("/saludos/Ada").send().await;

        // Comprobar
        respuesta.assert_status(StatusCode::OK);
        respuesta
            .json()
            .await
            .value()
            .object()
            .get("nombre")
            .assert_string("Ada");
    }

    #[tokio::test]
    async fn deberia_responder_422_cuando_se_rechaza_el_saludo() {
        // Preparar
        let mut caso_de_uso = MockObtenerSaludo::new();
        caso_de_uso
            .expect_ejecutar()
            .returning(|_| Err(ErrorSaludo::Validacion("nombre_vacio".into())));

        // Actuar
        let respuesta = cliente(caso_de_uso).get("/saludos/Ada").send().await;

        // Comprobar
        respuesta.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
}
