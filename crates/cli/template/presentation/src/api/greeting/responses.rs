use crate::api::{error::ErrorResponse, greeting::dto::GreetingDto};
use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};

#[derive(ApiResponse)]
pub enum GetGreetingResponse {
    #[oai(status = 200)]
    Ok(Json<GreetingDto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl GetGreetingResponse {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            _ => Self::InternalError(error),
        }
    }
}
