//! Auth (ADR-0021): who is calling and the port that says so. A route sees a [`Principal`], never a
//! token; where the token was checked — an OIDC provider, a local session — is the
//! [`Authenticator`]'s business. The adapters are `puerto-auth`'s and `puerto-sqlx`'s, the HTTP
//! side (`BearerAuth`, `#[require_role]`) `puerto-web`'s.

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

/// The authenticated caller.
///
/// `user_id` is the local `User` row every caller has (ADR-0021 § 3); `subject` is who the
/// provider says it is. `roles` decide `#[require_role]`; rules that depend on data take the
/// principal in their params and decide in the use case.
///
/// `#[non_exhaustive]`: built with [`Principal::new`] and the `with_*` methods, so a new field
/// later breaks no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub user_id: Uuid,
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

impl Principal {
    pub fn new(user_id: Uuid, subject: impl Into<String>) -> Self {
        Self {
            user_id,
            subject: subject.into(),
            email: None,
            name: None,
            roles: Vec::new(),
        }
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }

    /// Whether the caller holds `role` — compared exactly, as the provider or the `roles` column
    /// spells it.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|held| held == role)
    }
}

/// Why a token did not become a [`Principal`]. The first two are the caller's (401), the last is
/// ours (503): the adapter logs the cause, the API answers the code.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AuthError {
    /// Malformed, badly signed, for another issuer or audience, or an unknown session.
    #[error("auth.invalid")]
    Invalid,
    /// Was valid once: the token or the session expired.
    #[error("auth.expired")]
    Expired,
    /// Could not be checked: the provider or the database is unreachable.
    #[error("auth.unavailable")]
    Unavailable,
}

/// Turns a bearer token into the caller. `bootstrap.rs` registers one on the routes;
/// `puerto_web::auth::BearerAuth` asks it on every protected request.
#[async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(&self, token: &str) -> Result<Principal, AuthError>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use super::*;
    use mockall::mock;

    mock! {
        pub Authenticator {}

        #[async_trait]
        impl Authenticator for Authenticator {
            async fn authenticate(&self, token: &str) -> Result<Principal, AuthError>;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mocks::MockAuthenticator;
    use super::*;

    #[test]
    fn should_hold_no_role_or_profile_when_built_from_an_id_and_a_subject() {
        // Arrange
        let user_id = Uuid::now_v7();

        // Act
        let principal = Principal::new(user_id, "auth0|ada");

        // Assert
        assert_eq!(principal.user_id, user_id);
        assert_eq!(principal.subject, "auth0|ada");
        assert_eq!(principal.email, None);
        assert_eq!(principal.name, None);
        assert!(principal.roles.is_empty());
    }

    #[test]
    fn should_carry_the_profile_when_given_one() {
        // Arrange
        let user_id = Uuid::now_v7();

        // Act
        let principal = Principal::new(Uuid::nil(), "ada")
            .with_user_id(user_id)
            .with_email("ada@example.com")
            .with_name("Ada Lovelace")
            .with_roles(["admin", "editor"]);

        // Assert
        assert_eq!(principal.user_id, user_id);
        assert_eq!(principal.email.as_deref(), Some("ada@example.com"));
        assert_eq!(principal.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(principal.roles, vec!["admin", "editor"]);
    }

    #[test]
    fn should_have_a_role_only_when_it_holds_exactly_that_role() {
        // Arrange
        let principal = Principal::new(Uuid::nil(), "ada").with_roles(["admin"]);

        // Act / Assert
        assert!(principal.has_role("admin"));
        assert!(!principal.has_role("Admin"));
        assert!(!principal.has_role("adm"));
        assert!(!principal.has_role("editor"));
    }

    #[test]
    fn should_keep_error_codes_stable_when_logged() {
        assert_eq!(AuthError::Invalid.to_string(), "auth.invalid");
        assert_eq!(AuthError::Expired.to_string(), "auth.expired");
        assert_eq!(AuthError::Unavailable.to_string(), "auth.unavailable");
    }

    #[tokio::test]
    async fn should_answer_what_the_test_programs_when_mocked() {
        // Arrange
        let mut authenticator = MockAuthenticator::new();
        authenticator
            .expect_authenticate()
            .returning(|token| match token {
                "good" => Ok(Principal::new(Uuid::nil(), "ada")),
                _ => Err(AuthError::Expired),
            });
        let authenticator: &dyn Authenticator = &authenticator;

        // Act
        let accepted = authenticator.authenticate("good").await;
        let rejected = authenticator.authenticate("old").await;

        // Assert
        assert_eq!(accepted.map(|p| p.subject), Ok("ada".to_string()));
        assert_eq!(rejected, Err(AuthError::Expired));
    }
}
