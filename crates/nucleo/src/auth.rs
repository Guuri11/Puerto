//! Auth (ADR-0021): who is calling and the port that says so. A route sees a [`Principal`], never a
//! token; where the token was checked — an OIDC provider, a local session — is the
//! [`Autenticador`]'s business. The adapters are `puerto-auth`'s and `puerto-sqlx`'s, the HTTP
//! side (`BearerAuth`, `#[requiere_rol]`) `puerto-web`'s.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::ErrorRepositorio;

/// The authenticated caller.
///
/// `id_usuario` is the local `User` row every caller has (ADR-0021 § 3); `sujeto` is who the
/// provider says it is. `roles` decide `#[requiere_rol]`; rules that depend on data take the
/// principal in their params and decide in the use case.
///
/// `#[non_exhaustive]`: built with [`Principal::new`] and the `with_*` methods, so a new field
/// later breaks no one.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub id_usuario: Uuid,
    pub sujeto: String,
    pub email: Option<String>,
    pub nombre: Option<String>,
    pub roles: Vec<String>,
}

impl Principal {
    pub fn new(id_usuario: Uuid, sujeto: impl Into<String>) -> Self {
        Self {
            id_usuario,
            sujeto: sujeto.into(),
            email: None,
            nombre: None,
            roles: Vec::new(),
        }
    }

    pub fn con_id_usuario(mut self, id_usuario: Uuid) -> Self {
        self.id_usuario = id_usuario;
        self
    }

    pub fn con_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn con_nombre(mut self, nombre: impl Into<String>) -> Self {
        self.nombre = Some(nombre.into());
        self
    }

    pub fn con_roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }

    /// Whether the caller holds `role` — compared exactly, as the provider or the `roles` column
    /// spells it.
    pub fn tiene_rol(&self, role: &str) -> bool {
        self.roles.iter().any(|held| held == role)
    }
}

/// Why a token did not become a [`Principal`]. The first two are the caller's (401), the last is
/// ours (503): the adapter logs the cause, the API answers the code.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErrorAuth {
    /// Malformed, badly signed, for another issuer or audience, or an unknown session.
    #[error("auth.invalido")]
    Invalido,
    /// Was valid once: the token or the session expired.
    #[error("auth.expirado")]
    Expirado,
    /// Could not be checked: the provider or the database is unreachable.
    #[error("auth.no_disponible")]
    NoDisponible,
}

/// Turns a bearer token into the caller. `bootstrap.rs` registers one on the routes;
/// `puerto_web::auth::BearerAuth` asks it on every protected request.
#[async_trait]
pub trait Autenticador: Send + Sync {
    async fn autenticar(&self, token: &str) -> Result<Principal, ErrorAuth>;
}

// ── Local mode (ADR-0021 § 2): the API is the identity provider ─────────────────

/// Hashing a password failed — ours, not the caller's (500).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("auth.hash_fallido")]
pub struct ErrorHash;

/// Turns passwords into stored hashes and checks them. `puerto_auth::HasherArgon2` in a project;
/// the generated `register_user` and `login` use cases see only this port.
#[async_trait]
pub trait HasherContrasena: Send + Sync {
    async fn hash(&self, password: &str) -> Result<String, ErrorHash>;

    /// Whether `password` matches `hash`. `None` — an unknown email — still does the work against
    /// a fixed hash and answers `false`: the caller cannot tell the two cases apart by the time
    /// the answer takes.
    async fn verificar(&self, password: &str, hash: Option<String>) -> bool;
}

/// Where each user's password hash is kept, apart from the user (`User` carries no secret).
#[async_trait]
pub trait AlmacenCredenciales: Send + Sync {
    async fn hash_contrasena(&self, id_usuario: Uuid) -> Result<Option<String>, ErrorRepositorio>;
    /// Sets or replaces the user's hash.
    async fn guardar_hash_contrasena(
        &self,
        id_usuario: Uuid,
        hash: &str,
    ) -> Result<(), ErrorRepositorio>;
}

/// A session `login` opened: the opaque token the client sends back as a bearer token, and when
/// it stops being accepted.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sesion {
    pub token: String,
    pub expira_en: DateTime<Utc>,
}

impl Sesion {
    pub fn new(token: impl Into<String>, expira_en: DateTime<Utc>) -> Self {
        Self {
            token: token.into(),
            expira_en,
        }
    }
}

/// Opens and revokes sessions. `puerto_sqlx::auth::PgSessions` keeps them, and is the
/// [`Autenticador`] of a local project too.
#[async_trait]
pub trait AlmacenSesiones: Send + Sync {
    async fn abrir(&self, id_usuario: Uuid) -> Result<Sesion, ErrorRepositorio>;
    /// Revokes the session `token` opened; an unknown token is already revoked.
    async fn revocar(&self, token: &str) -> Result<(), ErrorRepositorio>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use super::*;
    use mockall::mock;

    /// Quien llama, para los tests (ADR-0021): un usuario con sesión y un id nuevo, con o sin
    /// roles.
    #[derive(Debug, Default)]
    pub struct MadrePrincipal {
        roles: Vec<String>,
    }

    impl MadrePrincipal {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn con_roles<I, S>(mut self, roles: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.roles = roles.into_iter().map(Into::into).collect();
            self
        }

        pub fn construir(self) -> Principal {
            let id_usuario = Uuid::now_v7();
            Principal::new(id_usuario, format!("sujeto-{id_usuario}"))
                .con_email(format!("{}@example.com", id_usuario.simple()))
                .con_nombre("Ada Lovelace")
                .con_roles(self.roles)
        }

        /// Alguien con sesión y sin ningún rol.
        pub fn aleatorio() -> Principal {
            Self::new().construir()
        }
    }

    mock! {
        pub Autenticador {}

        #[async_trait]
        impl Autenticador for Autenticador {
            async fn autenticar(&self, token: &str) -> Result<Principal, ErrorAuth>;
        }
    }

    mock! {
        pub HasherContrasena {}

        #[async_trait]
        impl HasherContrasena for HasherContrasena {
            async fn hash(&self, password: &str) -> Result<String, ErrorHash>;
            async fn verificar(&self, password: &str, hash: Option<String>) -> bool;
        }
    }

    mock! {
        pub AlmacenCredenciales {}

        #[async_trait]
        impl AlmacenCredenciales for AlmacenCredenciales {
            async fn hash_contrasena(&self, id_usuario: Uuid) -> Result<Option<String>, ErrorRepositorio>;
            async fn guardar_hash_contrasena(&self, id_usuario: Uuid, hash: &str) -> Result<(), ErrorRepositorio>;
        }
    }

    mock! {
        pub AlmacenSesiones {}

        #[async_trait]
        impl AlmacenSesiones for AlmacenSesiones {
            async fn abrir(&self, id_usuario: Uuid) -> Result<Sesion, ErrorRepositorio>;
            async fn revocar(&self, token: &str) -> Result<(), ErrorRepositorio>;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::dobles::{
        MockAlmacenCredenciales, MockAlmacenSesiones, MockAutenticador, MockHasherContrasena,
    };
    use super::*;

    #[test]
    fn should_build_a_fresh_caller_with_the_roles_it_is_given() {
        // Act
        let admin = dobles::MadrePrincipal::new()
            .con_roles(["admin"])
            .construir();
        let otro = dobles::MadrePrincipal::aleatorio();

        // Assert
        assert!(admin.tiene_rol("admin"));
        assert!(otro.roles.is_empty());
        assert_ne!(admin.id_usuario, otro.id_usuario);
    }

    #[test]
    fn should_hold_no_role_or_profile_when_built_from_an_id_and_a_subject() {
        // Arrange
        let id_usuario = Uuid::now_v7();

        // Act
        let principal = Principal::new(id_usuario, "auth0|ada");

        // Assert
        assert_eq!(principal.id_usuario, id_usuario);
        assert_eq!(principal.sujeto, "auth0|ada");
        assert_eq!(principal.email, None);
        assert_eq!(principal.nombre, None);
        assert!(principal.roles.is_empty());
    }

    #[test]
    fn should_carry_the_profile_when_given_one() {
        // Arrange
        let id_usuario = Uuid::now_v7();

        // Act
        let principal = Principal::new(Uuid::nil(), "ada")
            .con_id_usuario(id_usuario)
            .con_email("ada@example.com")
            .con_nombre("Ada Lovelace")
            .con_roles(["admin", "editor"]);

        // Assert
        assert_eq!(principal.id_usuario, id_usuario);
        assert_eq!(principal.email.as_deref(), Some("ada@example.com"));
        assert_eq!(principal.nombre.as_deref(), Some("Ada Lovelace"));
        assert_eq!(principal.roles, vec!["admin", "editor"]);
    }

    #[test]
    fn should_have_a_role_only_when_it_holds_exactly_that_role() {
        // Arrange
        let principal = Principal::new(Uuid::nil(), "ada").con_roles(["admin"]);

        // Act / Assert
        assert!(principal.tiene_rol("admin"));
        assert!(!principal.tiene_rol("Admin"));
        assert!(!principal.tiene_rol("adm"));
        assert!(!principal.tiene_rol("editor"));
    }

    #[test]
    fn should_keep_error_codes_stable_when_logged() {
        assert_eq!(ErrorAuth::Invalido.to_string(), "auth.invalido");
        assert_eq!(ErrorAuth::Expirado.to_string(), "auth.expirado");
        assert_eq!(ErrorAuth::NoDisponible.to_string(), "auth.no_disponible");
    }

    #[test]
    fn should_keep_the_token_and_its_expiry_when_a_session_is_built() {
        // Arrange
        let expira_en = chrono::Utc::now();

        // Act
        let session = Sesion::new("opaque", expira_en);

        // Assert
        assert_eq!(session.token, "opaque");
        assert_eq!(session.expira_en, expira_en);
        assert_eq!(ErrorHash.to_string(), "auth.hash_fallido");
    }

    #[tokio::test]
    async fn should_drive_the_local_ports_when_mocked() {
        // Arrange
        let id_usuario = Uuid::now_v7();
        let mut hasher = MockHasherContrasena::new();
        hasher
            .expect_verificar()
            .returning(|password, hash| hash.as_deref() == Some("h") && password == "secret");
        let mut credentials = MockAlmacenCredenciales::new();
        credentials
            .expect_hash_contrasena()
            .returning(|_| Ok(Some("h".to_string())));
        let mut sessions = MockAlmacenSesiones::new();
        sessions
            .expect_abrir()
            .returning(|_| Ok(Sesion::new("t", chrono::Utc::now())));
        let (hasher, credentials, sessions): (
            &dyn HasherContrasena,
            &dyn AlmacenCredenciales,
            &dyn AlmacenSesiones,
        ) = (&hasher, &credentials, &sessions);

        // Act
        let hash = credentials.hash_contrasena(id_usuario).await;
        let accepted = hasher.verificar("secret", Some("h".to_string())).await;
        let unknown = hasher.verificar("secret", None).await;
        let session = sessions.abrir(id_usuario).await;

        // Assert
        assert_eq!(hash, Ok(Some("h".to_string())));
        assert!(accepted);
        assert!(!unknown);
        assert_eq!(session.map(|s| s.token), Ok("t".to_string()));
    }

    #[tokio::test]
    async fn should_answer_what_the_test_programs_when_mocked() {
        // Arrange
        let mut authenticator = MockAutenticador::new();
        authenticator
            .expect_autenticar()
            .returning(|token| match token {
                "good" => Ok(Principal::new(Uuid::nil(), "ada")),
                _ => Err(ErrorAuth::Expirado),
            });
        let authenticator: &dyn Autenticador = &authenticator;

        // Act
        let accepted = authenticator.autenticar("good").await;
        let rejected = authenticator.autenticar("old").await;

        // Assert
        assert_eq!(accepted.map(|p| p.sujeto), Ok("ada".to_string()));
        assert_eq!(rejected, Err(ErrorAuth::Expirado));
    }
}
