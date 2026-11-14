//! [`AutenticadorOidc`]: verifies a provider's access tokens against its published keys.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use jsonwebtoken::jwk::{
    AlgorithmParameters, EllipticCurve, Jwk, JwkSet, KeyAlgorithm, PublicKeyUse,
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, errors::ErrorKind};
use puerto_nucleo::auth::{Autenticador, ErrorAuth, Principal};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::{ClaimRoles, ConfigOidc, claims};

/// How far `exp` and `nbf` may be off: the provider's clock and ours are never exactly in step.
pub const MARGEN: Duration = Duration::from_secs(60);
/// A token signed with a key we do not know refetches the keys at most this often: a key rotated
/// in is picked up, a flood of made-up `kid`s does not turn into a flood of requests to the
/// provider.
pub const INTERVALO_RECARGA: Duration = Duration::from_secs(60);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Why [`AutenticadorOidc::descubrir`] did not start: the provider is down, or it is not the
/// provider the configuration names. Its message is the whole report.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorOidc {
    #[error("no se pudo contactar con el proveedor de auth en {url}: {cause}")]
    Unreachable { url: String, cause: String },
    #[error("{url} no es la respuesta de un proveedor OpenID Connect: {cause}")]
    Invalido { url: String, cause: String },
    #[error(
        "AUTH_ISSUER is {configured} but the provider calls itself {discovered} — set AUTH_ISSUER to the provider's issuer"
    )]
    IssuerMismatch {
        configured: String,
        discovered: String,
    },
    #[error("{url} no publica ninguna clave de firma RS256 ni ES256")]
    NoSigningKey { url: String },
}

/// The [`Authenticator`] of an OIDC project — an OAuth2 resource server.
///
/// Accepts a JWT access token signed with RS256 or ES256 by a key in the provider's JWKS (never
/// `none`, never an HMAC), issued by the configured issuer for the configured audience, within
/// `exp` / `nbf` give or take [`MARGEN`]. The [`Principal`] has the token's `sub`, `email`, `name`
/// and the roles at [`ClaimRoles`]; its `user_id` is nil until the project provisions the caller.
pub struct AutenticadorOidc {
    http: reqwest::Client,
    issuer: String,
    audience: String,
    jwks_uri: String,
    claim_roles: ClaimRoles,
    keys: RwLock<HashMap<String, Arc<SigningKey>>>,
    /// When the keys were last refetched for an unknown `kid`; held while refetching, so
    /// concurrent requests with the same new key wait for one refetch instead of each making one.
    last_refetch: tokio::sync::Mutex<Option<Instant>>,
}

struct SigningKey {
    key: DecodingKey,
    algorithm: Algorithm,
}

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
}

enum FetchError {
    Unreachable(String),
    Invalido(String),
}

impl AutenticadorOidc {
    /// Reads `<emisor>/.well-known/openid-configuration` and the keys it points to. Fails when
    /// either is unreachable, when the provider names another issuer, or when it publishes no key
    /// this authenticator can verify with.
    pub async fn descubrir(config: ConfigOidc) -> Result<Self, ErrorOidc> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            config.emisor.trim_end_matches('/')
        );
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|err| ErrorOidc::Unreachable {
                url: url.clone(),
                cause: err.to_string(),
            })?;
        let discovery: Discovery = get_json(&http, &url).await.map_err(|err| err.at(&url))?;
        if discovery.issuer.trim_end_matches('/') != config.emisor.trim_end_matches('/') {
            return Err(ErrorOidc::IssuerMismatch {
                configured: config.emisor,
                discovered: discovery.issuer,
            });
        }
        let keys = fetch_keys(&http, &discovery.jwks_uri)
            .await
            .map_err(|err| err.at(&discovery.jwks_uri))?;
        if keys.is_empty() {
            return Err(ErrorOidc::NoSigningKey {
                url: discovery.jwks_uri,
            });
        }
        tracing::info!(issuer = %discovery.issuer, keys = keys.len(), "oidc: proveedor descubierto");
        Ok(Self {
            http,
            issuer: discovery.issuer,
            audience: config.audiencia,
            jwks_uri: discovery.jwks_uri,
            claim_roles: config.claim_roles,
            keys: RwLock::new(keys),
            last_refetch: tokio::sync::Mutex::new(None),
        })
    }

    fn cached(&self, kid: &str) -> Option<Arc<SigningKey>> {
        self.keys
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(kid)
            .cloned()
    }

    /// The key `kid` names, refetching the JWKS once if it is new (see [`INTERVALO_RECARGA`]).
    async fn key(&self, kid: &str) -> Result<Arc<SigningKey>, ErrorAuth> {
        if let Some(key) = self.cached(kid) {
            return Ok(key);
        }
        let mut last_refetch = self.last_refetch.lock().await;
        // Another request may have refetched while this one waited.
        if let Some(key) = self.cached(kid) {
            return Ok(key);
        }
        if last_refetch.is_some_and(|at| at.elapsed() < INTERVALO_RECARGA) {
            tracing::debug!(
                kid,
                "oidc: clave desconocida, recargadas hace menos de un minuto"
            );
            return Err(ErrorAuth::Invalido);
        }
        // Counted before the fetch, so a failed one counts too: made-up `kid`s while the provider
        // is down stay one request a minute.
        *last_refetch = Some(Instant::now());
        let keys = fetch_keys(&self.http, &self.jwks_uri)
            .await
            .map_err(|err| {
                tracing::warn!(error = %err.at(&self.jwks_uri), "oidc: no se pudieron recargar las claves");
                ErrorAuth::NoDisponible
            })?;
        *self.keys.write().unwrap_or_else(PoisonError::into_inner) = keys;
        self.cached(kid).ok_or_else(|| {
            tracing::debug!(
                kid,
                "oidc: clave desconocida, tampoco está en las claves recargadas"
            );
            ErrorAuth::Invalido
        })
    }

    fn validation(&self, algorithm: Algorithm) -> Validation {
        let mut validation = Validation::new(algorithm);
        validation.leeway = MARGEN.as_secs();
        validation.validate_nbf = true;
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation
    }
}

#[async_trait]
impl Autenticador for AutenticadorOidc {
    async fn autenticar(&self, token: &str) -> Result<Principal, ErrorAuth> {
        let header = decode_header(token).map_err(|err| rejected(&err))?;
        // Only what a provider signs with its private key. An HMAC would be verified with the
        // public key as the secret — which anyone can read from the JWKS.
        if !matches!(header.alg, Algorithm::RS256 | Algorithm::ES256) {
            tracing::debug!(alg = ?header.alg, "oidc: token rechazado, algoritmo no permitido");
            return Err(ErrorAuth::Invalido);
        }
        let Some(kid) = header.kid else {
            tracing::debug!("oidc: token rechazado, sin kid");
            return Err(ErrorAuth::Invalido);
        };
        let key = self.key(&kid).await?;
        if key.algorithm != header.alg {
            tracing::debug!(kid, alg = ?header.alg, "oidc: token rechazado, no es el algoritmo de la clave");
            return Err(ErrorAuth::Invalido);
        }
        let claims = decode::<Map<String, Value>>(token, &key.key, &self.validation(header.alg))
            .map_err(|err| rejected(&err))?
            .claims;
        claims::principal(&claims, &self.claim_roles)
    }
}

fn rejected(err: &jsonwebtoken::errors::Error) -> ErrorAuth {
    tracing::debug!(error = %err, "oidc: token rechazado");
    match err.kind() {
        ErrorKind::ExpiredSignature => ErrorAuth::Expirado,
        _ => ErrorAuth::Invalido,
    }
}

impl FetchError {
    fn at(self, url: &str) -> ErrorOidc {
        let url = url.to_string();
        match self {
            Self::Unreachable(cause) => ErrorOidc::Unreachable { url, cause },
            Self::Invalido(cause) => ErrorOidc::Invalido { url, cause },
        }
    }
}

async fn get_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T, FetchError> {
    let response = http
        .get(url)
        .send()
        .await
        .map_err(|err| FetchError::Unreachable(err.to_string()))?;
    let status = response.status();
    if status.is_server_error() {
        return Err(FetchError::Unreachable(format!("respondió {status}")));
    }
    if !status.is_success() {
        return Err(FetchError::Invalido(format!("respondió {status}")));
    }
    response
        .json()
        .await
        .map_err(|err| FetchError::Invalido(err.to_string()))
}

/// The provider's signing keys this authenticator can verify with, by `kid`.
async fn fetch_keys(
    http: &reqwest::Client,
    url: &str,
) -> Result<HashMap<String, Arc<SigningKey>>, FetchError> {
    let set: JwkSet = get_json(http, url).await?;
    Ok(set
        .keys
        .iter()
        .filter_map(|jwk| Some((jwk.common.key_id.clone()?, Arc::new(signing_key(jwk)?))))
        .collect())
}

/// A JWK as a key for RS256 or ES256 — `None` for an encryption key, another algorithm or curve.
fn signing_key(jwk: &Jwk) -> Option<SigningKey> {
    if matches!(jwk.common.public_key_use, Some(PublicKeyUse::Encryption)) {
        return None;
    }
    let algorithm = match (&jwk.algorithm, jwk.common.key_algorithm) {
        (AlgorithmParameters::RSA(_), None | Some(KeyAlgorithm::RS256)) => Algorithm::RS256,
        (AlgorithmParameters::EllipticCurve(ec), None | Some(KeyAlgorithm::ES256))
            if ec.curve == EllipticCurve::P256 =>
        {
            Algorithm::ES256
        }
        _ => return None,
    };
    let key = DecodingKey::from_jwk(jwk).ok()?;
    Some(SigningKey { key, algorithm })
}
