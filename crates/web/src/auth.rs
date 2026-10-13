//! The HTTP side of auth (ADR-0021 § 4): [`BearerAuth`], the parameter that makes a handler
//! authenticated, and [`forbidden`], the answer `#[require_role]` gives a caller without the role.
//!
//! `BearerAuth` asks the [`Authenticator`] the routes carry as data — `bootstrap.rs` registers it:
//!
//! ```ignore
//! let authenticator: Arc<dyn Authenticator> = Arc::new(…);
//! puerto_web::routes(openapi_service(…), &config, ready).data(authenticator)
//! ```

use std::sync::Arc;

use poem::{IntoResponse, Request, RequestBody, http::StatusCode};
use poem_openapi::{
    ApiExtractor, ApiExtractorType, ExtractParamOptions, SecurityScheme, auth::Bearer,
    payload::Json, registry::Registry,
};
use puerto_core::{AuthError, Authenticator, Principal};

use crate::ErrorResponse;

/// No token, a token that is not `Bearer`, or one the [`Authenticator`] rejected.
pub const UNAUTHENTICATED: &str = "auth.unauthenticated";
/// An authenticated caller without the role the route requires.
pub const FORBIDDEN: &str = "auth.forbidden";
/// The token could not be checked: the provider or the database is down — not the caller's fault.
pub const UNAVAILABLE: &str = "auth.unavailable";
/// The routes carry no [`Authenticator`]: `bootstrap.rs` did not register one.
pub const NOT_CONFIGURED: &str = "auth.not_configured";

/// The authenticated caller of a handler that takes it: `auth: BearerAuth`.
///
/// A request without a bearer token, or with one the [`Authenticator`] rejects, never reaches the
/// handler: it is answered `401 auth.unauthenticated` (`503 auth.unavailable` when the token could
/// not be checked). The route's spec entry carries `security: [{"BearerAuth": []}]`.
pub struct BearerAuth(pub Principal);

/// What poem-openapi registers in the spec for [`BearerAuth`]. The extraction is `BearerAuth`'s
/// own: the derive's checker cannot answer a missing header with an [`ErrorResponse`].
#[derive(SecurityScheme)]
#[oai(rename = "BearerAuth", ty = "bearer", bearer_format = "opaque or JWT")]
struct BearerScheme(Bearer);

impl<'a> ApiExtractor<'a> for BearerAuth {
    const TYPES: &'static [ApiExtractorType] = &[ApiExtractorType::SecurityScheme];

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut Registry) {
        BearerScheme::register(registry);
    }

    fn security_schemes() -> Vec<&'static str> {
        BearerScheme::security_schemes()
    }

    async fn from_request(
        req: &'a Request,
        body: &mut RequestBody,
        param_opts: ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        let token = BearerScheme::from_request(req, body, param_opts)
            .await
            .map_err(|_| {
                error(
                    StatusCode::UNAUTHORIZED,
                    ErrorResponse::from_code(UNAUTHENTICATED),
                )
            })?
            .0
            .token;
        let Some(authenticator) = req.data::<Arc<dyn Authenticator>>() else {
            tracing::error!("BearerAuth: no Authenticator registered on the routes");
            return Err(error(
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse {
                    name: NOT_CONFIGURED.into(),
                    message: "no Authenticator is registered on the routes: bootstrap.rs must add \
                              `.data(authenticator)` with an `Arc<dyn Authenticator>`"
                        .into(),
                },
            ));
        };
        authenticator
            .authenticate(&token)
            .await
            .map(Self)
            .map_err(|err| {
                let (status, body) = rejection(err);
                error(status, body.0)
            })
    }
}

/// The status and body for a token the [`Authenticator`] did not accept.
pub fn rejection(err: AuthError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        AuthError::Unavailable => {
            tracing::warn!(error = %err, "BearerAuth: the token could not be checked");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::from_code(UNAVAILABLE)),
            )
        }
        // `Invalid`, `Expired`, and whatever `AuthError` gains: the caller is not authenticated.
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::from_code(UNAUTHENTICATED)),
        ),
    }
}

/// What `#[require_role]` answers a caller without the role: `403 auth.forbidden`.
pub fn forbidden() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::from_code(FORBIDDEN)),
    )
}

/// The bearer token of `req` as sent — what a local project's `POST /auth/logout` revokes. The
/// scheme is matched case-insensitively, as `BearerAuth` matches it.
pub fn bearer_token(req: &poem::Request) -> Option<&str> {
    let (scheme, token) = req.header("Authorization")?.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

/// A `poem::Error` whose response is `body` as JSON — what the client reads, unlike the plain-text
/// body poem gives an extractor's error.
fn error(status: StatusCode, body: ErrorResponse) -> poem::Error {
    let mut response = Json(body).into_response();
    response.set_status(status);
    poem::Error::from_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(authorization: Option<&str>) -> poem::Request {
        let builder = poem::Request::builder();
        match authorization {
            Some(value) => builder.header("Authorization", value).finish(),
            None => builder.finish(),
        }
    }

    #[test]
    fn should_read_the_bearer_token_whatever_the_scheme_case() {
        assert_eq!(bearer_token(&request(Some("Bearer abc"))), Some("abc"));
        assert_eq!(bearer_token(&request(Some("bearer  abc "))), Some("abc"));
    }

    #[test]
    fn should_read_no_token_when_there_is_no_bearer_one() {
        assert_eq!(bearer_token(&request(None)), None);
        assert_eq!(bearer_token(&request(Some("Basic abc"))), None);
        assert_eq!(bearer_token(&request(Some("Bearer "))), None);
        assert_eq!(bearer_token(&request(Some("Bearer"))), None);
    }

    #[test]
    fn should_answer_401_unauthenticated_when_the_token_is_invalid() {
        // Act
        let (status, body) = rejection(AuthError::Invalid);

        // Assert
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.name, "auth.unauthenticated");
    }

    #[test]
    fn should_answer_401_unauthenticated_when_the_token_expired() {
        // Act
        let (status, body) = rejection(AuthError::Expired);

        // Assert
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.name, "auth.unauthenticated");
    }

    #[test]
    fn should_answer_503_unavailable_when_the_token_cannot_be_checked() {
        // Act
        let (status, body) = rejection(AuthError::Unavailable);

        // Assert
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.0.name, "auth.unavailable");
    }

    #[test]
    fn should_answer_403_forbidden_when_a_role_is_missing() {
        // Act
        let (status, body) = forbidden();

        // Assert
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.0.name, "auth.forbidden");
        assert_eq!(body.0.message, "auth.forbidden");
    }
}
