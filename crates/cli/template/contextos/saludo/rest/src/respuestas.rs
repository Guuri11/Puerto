use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};
use puerto_web::RespuestaError;

use crate::dto::DtoSaludo;

#[derive(ApiResponse)]
pub enum RespuestaObtenerSaludo {
    #[oai(status = 200)]
    Ok(Json<DtoSaludo>),
    #[oai(status = 422)]
    NoProcesable(Json<RespuestaError>),
    #[oai(status = 401)]
    NoAutorizado(Json<RespuestaError>),
    #[oai(status = 403)]
    Prohibido(Json<RespuestaError>),
    #[oai(status = 500)]
    ErrorInterno(Json<RespuestaError>),
}

impl RespuestaObtenerSaludo {
    pub fn from_status(status: StatusCode, error: Json<RespuestaError>) -> Self {
        match status {
            StatusCode::UNPROCESSABLE_ENTITY => Self::NoProcesable(error),
            StatusCode::UNAUTHORIZED => Self::NoAutorizado(error),
            StatusCode::FORBIDDEN => Self::Prohibido(error),
            _ => Self::ErrorInterno(error),
        }
    }
}
