use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};
use puerto_web::RespuestaError;

use crate::dto::GreetingDto;

#[derive(ApiResponse)]
pub enum GetGreetingResponse {
    #[oai(status = 200)]
    Ok(Json<GreetingDto>),
    #[oai(status = 422)]
    UnprocessableEntity(Json<RespuestaError>),
    #[oai(status = 401)]
    Unauthorized(Json<RespuestaError>),
    #[oai(status = 403)]
    Forbidden(Json<RespuestaError>),
    #[oai(status = 500)]
    InternalError(Json<RespuestaError>),
}

impl GetGreetingResponse {
    pub fn from_status(status: StatusCode, error: Json<RespuestaError>) -> Self {
        match status {
            StatusCode::UNPROCESSABLE_ENTITY => Self::UnprocessableEntity(error),
            StatusCode::UNAUTHORIZED => Self::Unauthorized(error),
            StatusCode::FORBIDDEN => Self::Forbidden(error),
            _ => Self::InternalError(error),
        }
    }
}
