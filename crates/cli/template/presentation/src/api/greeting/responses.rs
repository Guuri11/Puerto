use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};
use puerto_web::ErrorResponse;

use crate::api::greeting::dto::GreetingDto;

#[derive(ApiResponse)]
pub enum GetGreetingResponse {
    #[oai(status = 200)]
    Ok(Json<GreetingDto>),
    #[oai(status = 422)]
    UnprocessableEntity(Json<ErrorResponse>),
    #[oai(status = 401)]
    Unauthorized(Json<ErrorResponse>),
    #[oai(status = 403)]
    Forbidden(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl GetGreetingResponse {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::UNPROCESSABLE_ENTITY => Self::UnprocessableEntity(error),
            StatusCode::UNAUTHORIZED => Self::Unauthorized(error),
            StatusCode::FORBIDDEN => Self::Forbidden(error),
            _ => Self::InternalError(error),
        }
    }
}
