use poem::http::StatusCode;
use poem_openapi::{Object, payload::Json};

#[derive(Object, Debug)]
pub struct ErrorResponse {
    pub name: String,
    pub message: String,
}

pub trait IntoErrorResponse {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>);
}
