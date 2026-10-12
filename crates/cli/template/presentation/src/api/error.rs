use poem::http::StatusCode;
use poem_openapi::payload::Json;
use puerto_web::ErrorResponse;

/// Maps a domain error to its HTTP status and body. Validation failures are `422 Unprocessable
/// Entity`: the request was well-formed (poem already answers `400` for one that is not) but
/// breaks a domain rule.
///
/// The body, `ErrorResponse`, is the Puerto runtime's (`puerto-web`); the trait stays here because
/// each `error_mapper.rs` implements it for an error of `business` — Rust only allows that for a
/// trait of this crate.
pub trait IntoErrorResponse {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>);
}
