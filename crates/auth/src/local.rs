//! Local accounts (ADR-0021 § 2): the API is the identity provider. This module hashes and checks
//! passwords — argon2id with the crate's defaults (19 MiB, 2 passes, OWASP's recommendation), a
//! random salt per hash, stored as a PHC string (`$argon2id$v=19$…`). The sessions a successful
//! login opens are `puerto_sqlx::auth::PgSessions`.

use std::sync::OnceLock;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordVerifier};
use async_trait::async_trait;
use puerto_nucleo::auth::{ErrorHash, HasherContrasena};

/// The PHC string of `password`, with a fresh salt.
pub fn hash_contrasena(password: &str) -> Result<String, ErrorHash> {
    let salt = SaltString::generate(&mut OsRng);
    argon2::PasswordHasher::hash_password(&Argon2::default(), password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| {
            tracing::error!(error = %err, "hash_password failed");
            ErrorHash
        })
}

/// Whether `password` is the one `hash` was made from. A `hash` that is not a PHC string matches
/// nothing.
pub fn verificar_contrasena(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        tracing::warn!("verify_password: the stored hash is not a PHC string");
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// What an unknown email is checked against: a hash with the same parameters as a real one, so
/// both answers take as long.
fn decoy() -> Option<&'static str> {
    static DECOY: OnceLock<Option<String>> = OnceLock::new();
    DECOY
        .get_or_init(|| hash_contrasena("puerto: no account has this password").ok())
        .as_deref()
}

/// [`PasswordHasher`] over argon2id. Each hash and check runs on tokio's blocking pool: it is tens
/// of milliseconds of CPU, which would otherwise stall every request sharing the worker thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct HasherArgon2;

impl HasherArgon2 {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HasherContrasena for HasherArgon2 {
    async fn hash(&self, password: &str) -> Result<String, ErrorHash> {
        let password = password.to_string();
        tokio::task::spawn_blocking(move || hash_contrasena(&password))
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "hash: the blocking task failed");
                ErrorHash
            })?
    }

    async fn verificar(&self, password: &str, hash: Option<String>) -> bool {
        let password = password.to_string();
        tokio::task::spawn_blocking(move || match hash {
            Some(hash) => verificar_contrasena(&password, &hash),
            None => {
                if let Some(decoy) = decoy() {
                    let _ = verificar_contrasena(&password, decoy);
                }
                false
            }
        })
        .await
        .unwrap_or(false)
    }
}
