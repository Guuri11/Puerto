//! Local accounts (ADR-0021 § 2): the sessions `login` opens and the password hashes it checks.
//! Their tables are [`crate::MIGRACION_USUARIOS`] and [`crate::MIGRACION_AUTH`], which
//! `puerto new --auth local` writes into the project.
//!
//! A session is an opaque token — 32 random bytes, base64url — and its row keeps only the token's
//! SHA-256. Revoking is a `DELETE`; there is no signing key to keep or rotate, and a lookup per
//! request is the cost.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, TimeDelta, Utc};
use puerto_nucleo::ErrorRepositorio;
use puerto_nucleo::auth::{
    AlmacenCredenciales, AlmacenSesiones, Autenticador, ErrorAuth, Principal, Sesion,
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::mapear_error_sqlx;

/// How long a session lasts when `AUTH_SESSION_TTL` is not set.
pub const TTL_SESION_POR_DEFECTO: Duration = Duration::from_secs(30 * 24 * 3600);

/// A new session token: 256 random bits, base64url without padding (43 characters).
pub fn generar_token() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// What a session row keeps of its token.
pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// `AUTH_SESSION_TTL` — `30d`, `12h`, `90m` — or [`TTL_SESION_POR_DEFECTO`] when it is not set. A set
/// but invalid value is an error: a startup error, not a silent default.
pub fn ttl_sesion_from_env() -> Result<Duration, String> {
    let _ = dotenvy::dotenv();
    match std::env::var("AUTH_SESSION_TTL") {
        Err(_) => Ok(TTL_SESION_POR_DEFECTO),
        Ok(value) => parsear_ttl(&value).ok_or_else(|| {
            format!("AUTH_SESSION_TTL={value} is not a duration: use days, hours or minutes (30d, 12h, 90m)")
        }),
    }
}

/// `<n>d`, `<n>h` or `<n>m`, `n` > 0.
pub fn parsear_ttl(value: &str) -> Option<Duration> {
    let value = value.trim();
    let unit = match value.chars().last()? {
        'd' => 24 * 3600,
        'h' => 3600,
        'm' => 60,
        _ => return None,
    };
    let n: u64 = value[..value.len() - 1].parse().ok()?;
    if n == 0 {
        return None;
    }
    n.checked_mul(unit).map(Duration::from_secs)
}

/// `sessions`: a [`SessionStore`] for `login` and `logout`, and the [`Authenticator`] of a local
/// project — the presented token's hash, joined with its user. The roles are the `users.roles`
/// column (`make auth/grant`).
pub struct SesionesPostgres {
    pool: PgPool,
    ttl: TimeDelta,
}

impl SesionesPostgres {
    pub fn new(pool: PgPool, ttl: Duration) -> Self {
        Self {
            pool,
            ttl: TimeDelta::from_std(ttl).unwrap_or(TimeDelta::MAX),
        }
    }
}

/// A session's user: id, subject, email, name, roles, and the session's expiry.
type SessionUser = (
    Uuid,
    String,
    Option<String>,
    Option<String>,
    Vec<String>,
    DateTime<Utc>,
);

#[async_trait]
impl AlmacenSesiones for SesionesPostgres {
    async fn abrir(&self, user_id: Uuid) -> Result<Sesion, ErrorRepositorio> {
        let token = generar_token().map_err(|err| {
            tracing::error!(error = %err, "sessions: no randomness for a token");
            ErrorRepositorio::NoDisponible
        })?;
        let expires_at = Utc::now() + self.ttl;
        sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(&hash_token(&token)[..])
            .bind(user_id)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, %user_id, "sessions: open failed");
                mapear_error_sqlx(&err)
            })?;
        Ok(Sesion::new(token, expires_at))
    }

    async fn revocar(&self, token: &str) -> Result<(), ErrorRepositorio> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(&hash_token(token)[..])
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|err| {
                tracing::error!(error = %err, "sessions: revoke failed");
                mapear_error_sqlx(&err)
            })
    }
}

#[async_trait]
impl Autenticador for SesionesPostgres {
    async fn autenticar(&self, token: &str) -> Result<Principal, ErrorAuth> {
        let row = sqlx::query_as::<_, SessionUser>(
            "SELECT u.id, u.subject, u.email, u.name, u.roles, s.expires_at \
             FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = $1",
        )
        .bind(&hash_token(token)[..])
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "sessions: lookup failed");
            ErrorAuth::NoDisponible
        })?
        .ok_or(ErrorAuth::Invalido)?;
        let (id, subject, email, name, roles, expires_at) = row;
        if expires_at <= Utc::now() {
            return Err(ErrorAuth::Expirado);
        }
        let mut principal = Principal::new(id, subject).con_roles(roles);
        if let Some(email) = email {
            principal = principal.con_email(email);
        }
        if let Some(name) = name {
            principal = principal.con_nombre(name);
        }
        Ok(principal)
    }
}

/// `credentials`: one password hash per user.
pub struct CredencialesPostgres {
    pool: PgPool,
}

impl CredencialesPostgres {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlmacenCredenciales for CredencialesPostgres {
    async fn hash_contrasena(&self, user_id: Uuid) -> Result<Option<String>, ErrorRepositorio> {
        sqlx::query_scalar("SELECT password_hash FROM credentials WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, %user_id, "credentials: read failed");
                mapear_error_sqlx(&err)
            })
    }

    async fn guardar_hash_contrasena(
        &self,
        user_id: Uuid,
        hash: &str,
    ) -> Result<(), ErrorRepositorio> {
        sqlx::query(
            "INSERT INTO credentials (user_id, password_hash) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET password_hash = EXCLUDED.password_hash, \
             updated_at = now()",
        )
        .bind(user_id)
        .bind(hash)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|err| {
            tracing::error!(error = %err, %user_id, "credentials: write failed");
            mapear_error_sqlx(&err)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_make_a_new_url_safe_token_of_256_bits_each_time() {
        // Act
        let first = generar_token().unwrap();
        let second = generar_token().unwrap();

        // Assert
        assert_eq!(first.len(), 43);
        assert!(
            first
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_ne!(first, second);
    }

    #[test]
    fn should_hash_a_token_the_same_way_every_time() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }

    #[test]
    fn should_read_days_hours_and_minutes() {
        assert_eq!(
            parsear_ttl("30d"),
            Some(Duration::from_secs(30 * 24 * 3600))
        );
        assert_eq!(parsear_ttl("12h"), Some(Duration::from_secs(12 * 3600)));
        assert_eq!(parsear_ttl(" 90m "), Some(Duration::from_secs(90 * 60)));
    }

    #[test]
    fn should_refuse_a_ttl_without_a_unit_a_zero_or_an_overflow() {
        for value in [
            "30",
            "0d",
            "-1d",
            "d",
            "",
            "1w",
            "1.5d",
            "99999999999999999999d",
        ] {
            assert_eq!(parsear_ttl(value), None, "{value}");
        }
    }
}
