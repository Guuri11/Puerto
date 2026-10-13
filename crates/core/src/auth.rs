//! Auth (ADR-0021): who is calling and the port that says so. A route sees a [`Principal`], never a
//! token; where the token was checked — an OIDC provider, a local session — is the
//! [`Authenticator`]'s business. The adapters are `puerto-auth`'s and `puerto-sqlx`'s, the HTTP
//! side (`BearerAuth`, `#[require_role]`) `puerto-web`'s.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::RepositoryError;

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

// ── Local mode (ADR-0021 § 2): the API is the identity provider ─────────────────

/// Hashing a password failed — ours, not the caller's (500).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("auth.hash_failed")]
pub struct HashError;

/// Turns passwords into stored hashes and checks them. `puerto_auth::Argon2Hasher` in a project;
/// the generated `register_user` and `login` use cases see only this port.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<String, HashError>;

    /// Whether `password` matches `hash`. `None` — an unknown email — still does the work against
    /// a fixed hash and answers `false`: the caller cannot tell the two cases apart by the time
    /// the answer takes.
    async fn verify(&self, password: &str, hash: Option<String>) -> bool;
}

/// Where each user's password hash is kept, apart from the user (`User` carries no secret).
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn password_hash(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError>;
    /// Sets or replaces the user's hash.
    async fn set_password_hash(&self, user_id: Uuid, hash: &str) -> Result<(), RepositoryError>;
}

/// A session `login` opened: the opaque token the client sends back as a bearer token, and when
/// it stops being accepted.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn new(token: impl Into<String>, expires_at: DateTime<Utc>) -> Self {
        Self {
            token: token.into(),
            expires_at,
        }
    }
}

/// Opens and revokes sessions. `puerto_sqlx::auth::PgSessions` keeps them, and is the
/// [`Authenticator`] of a local project too.
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn open(&self, user_id: Uuid) -> Result<Session, RepositoryError>;
    /// Revokes the session `token` opened; an unknown token is already revoked.
    async fn revoke(&self, token: &str) -> Result<(), RepositoryError>;
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

    mock! {
        pub PasswordHasher {}

        #[async_trait]
        impl PasswordHasher for PasswordHasher {
            async fn hash(&self, password: &str) -> Result<String, HashError>;
            async fn verify(&self, password: &str, hash: Option<String>) -> bool;
        }
    }

    mock! {
        pub CredentialStore {}

        #[async_trait]
        impl CredentialStore for CredentialStore {
            async fn password_hash(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError>;
            async fn set_password_hash(&self, user_id: Uuid, hash: &str) -> Result<(), RepositoryError>;
        }
    }

    mock! {
        pub SessionStore {}

        #[async_trait]
        impl SessionStore for SessionStore {
            async fn open(&self, user_id: Uuid) -> Result<Session, RepositoryError>;
            async fn revoke(&self, token: &str) -> Result<(), RepositoryError>;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mocks::{
        MockAuthenticator, MockCredentialStore, MockPasswordHasher, MockSessionStore,
    };
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

    #[test]
    fn should_keep_the_token_and_its_expiry_when_a_session_is_built() {
        // Arrange
        let expires_at = chrono::Utc::now();

        // Act
        let session = Session::new("opaque", expires_at);

        // Assert
        assert_eq!(session.token, "opaque");
        assert_eq!(session.expires_at, expires_at);
        assert_eq!(HashError.to_string(), "auth.hash_failed");
    }

    #[tokio::test]
    async fn should_drive_the_local_ports_when_mocked() {
        // Arrange
        let user_id = Uuid::now_v7();
        let mut hasher = MockPasswordHasher::new();
        hasher
            .expect_verify()
            .returning(|password, hash| hash.as_deref() == Some("h") && password == "secret");
        let mut credentials = MockCredentialStore::new();
        credentials
            .expect_password_hash()
            .returning(|_| Ok(Some("h".to_string())));
        let mut sessions = MockSessionStore::new();
        sessions
            .expect_open()
            .returning(|_| Ok(Session::new("t", chrono::Utc::now())));
        let (hasher, credentials, sessions): (
            &dyn PasswordHasher,
            &dyn CredentialStore,
            &dyn SessionStore,
        ) = (&hasher, &credentials, &sessions);

        // Act
        let hash = credentials.password_hash(user_id).await;
        let accepted = hasher.verify("secret", Some("h".to_string())).await;
        let unknown = hasher.verify("secret", None).await;
        let session = sessions.open(user_id).await;

        // Assert
        assert_eq!(hash, Ok(Some("h".to_string())));
        assert!(accepted);
        assert!(!unknown);
        assert_eq!(session.map(|s| s.token), Ok("t".to_string()));
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
