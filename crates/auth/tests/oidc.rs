//! `OidcAuthenticator` against a fake provider: a local HTTP server publishing a discovery document
//! and a JWKS built from the keys in `fixtures/` (test-only keys), with tokens signed in the test.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use poem::listener::{Acceptor, Listener, TcpListener};
use poem::{IntoResponse, Route, Server, http::StatusCode, web::Json};
use puerto_auth::{
    AuthError, Authenticator, OidcAuthenticator, OidcConfig, OidcError, Principal, RolesClaim,
};
use serde_json::{Value, json};
use uuid::Uuid;

const AUDIENCE: &str = "api";

struct Key {
    kid: &'static str,
    algorithm: Algorithm,
    pem: &'static [u8],
}

const RSA: Key = Key {
    kid: "rsa",
    algorithm: Algorithm::RS256,
    pem: include_bytes!("fixtures/rsa.pem"),
};
const EC: Key = Key {
    kid: "ec",
    algorithm: Algorithm::ES256,
    pem: include_bytes!("fixtures/ec.pem"),
};
const ROTATED: Key = Key {
    kid: "rotated",
    algorithm: Algorithm::RS256,
    pem: include_bytes!("fixtures/rsa_rotated.pem"),
};

impl Key {
    fn encoding(&self) -> EncodingKey {
        match self.algorithm {
            Algorithm::ES256 => EncodingKey::from_ec_pem(self.pem),
            _ => EncodingKey::from_rsa_pem(self.pem),
        }
        .expect("fixture key")
    }

    fn jwk(&self) -> Jwk {
        let mut jwk = Jwk::from_encoding_key(&self.encoding(), self.algorithm).expect("a JWK");
        jwk.common.key_id = Some(self.kid.into());
        jwk
    }
}

fn jwks(keys: &[&Key]) -> Value {
    serde_json::to_value(JwkSet {
        keys: keys.iter().map(|key| key.jwk()).collect(),
    })
    .expect("JWKS as JSON")
}

/// The fake provider: what it publishes, how often its JWKS was fetched, whether it is failing.
struct Provider {
    issuer: String,
    jwks: Arc<Mutex<Value>>,
    fetches: Arc<AtomicUsize>,
    down: Arc<AtomicBool>,
}

impl Provider {
    async fn start(keys: Value) -> Self {
        Self::start_calling_itself(keys, |base| base.to_string()).await
    }

    /// A provider whose discovery document names `issuer(base)` as its issuer.
    async fn start_calling_itself(keys: Value, issuer: impl Fn(&str) -> String) -> Self {
        let acceptor = TcpListener::bind("127.0.0.1:0")
            .into_acceptor()
            .await
            .expect("a free port");
        let port = acceptor.local_addr()[0]
            .as_socket_addr()
            .expect("a TCP address")
            .port();
        let base = format!("http://127.0.0.1:{port}");
        let jwks = Arc::new(Mutex::new(keys));
        let fetches = Arc::new(AtomicUsize::new(0));
        let down = Arc::new(AtomicBool::new(false));
        let discovery = json!({ "issuer": issuer(&base), "jwks_uri": format!("{base}/jwks") });
        let app = Route::new()
            .at(
                "/.well-known/openid-configuration",
                poem::endpoint::make(move |_| {
                    let discovery = discovery.clone();
                    async move { Json(discovery) }
                }),
            )
            .at("/jwks", {
                let (jwks, fetches, down) = (jwks.clone(), fetches.clone(), down.clone());
                poem::endpoint::make(move |_| {
                    fetches.fetch_add(1, Ordering::SeqCst);
                    let keys = jwks.lock().unwrap().clone();
                    let down = down.load(Ordering::SeqCst);
                    async move {
                        if down {
                            StatusCode::INTERNAL_SERVER_ERROR.into_response()
                        } else {
                            Json(keys).into_response()
                        }
                    }
                })
            });
        tokio::spawn(Server::new_with_acceptor(acceptor).run(app));
        Self {
            issuer: base,
            jwks,
            fetches,
            down,
        }
    }

    fn config(&self) -> OidcConfig {
        OidcConfig::new(&self.issuer, AUDIENCE)
    }

    async fn authenticator(&self) -> OidcAuthenticator {
        OidcAuthenticator::discover(self.config())
            .await
            .expect("the fake provider is discovered")
    }

    fn publish(&self, keys: Value) {
        *self.jwks.lock().unwrap() = keys;
    }

    fn fetches(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }

    /// The claims of a valid token for this provider, valid for five minutes.
    fn claims(&self) -> Value {
        let now = now();
        json!({
            "iss": self.issuer,
            "aud": AUDIENCE,
            "sub": "f81d4fae-7dec",
            "iat": now,
            "exp": now + 300,
            "email": "ada@example.com",
            "name": "Ada Lovelace",
            "realm_access": { "roles": ["admin", "offline_access"] },
        })
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
}

fn sign(key: &Key, claims: &Value) -> String {
    sign_as(key, key.kid, claims)
}

/// A token signed by `key` whose header names `kid`.
fn sign_as(key: &Key, kid: &str, claims: &Value) -> String {
    let mut header = Header::new(key.algorithm);
    header.kid = Some(kid.into());
    encode(&header, claims, &key.encoding()).expect("a signed token")
}

fn with(mut claims: Value, field: &str, value: Value) -> Value {
    claims[field] = value;
    claims
}

fn without(mut claims: Value, field: &str) -> Value {
    claims.as_object_mut().unwrap().remove(field);
    claims
}

/// Base64url without padding — to hand-craft a token the library would refuse to sign.
fn b64(value: &Value) -> String {
    URL_SAFE_NO_PAD.encode(value.to_string())
}

#[tokio::test]
async fn should_describe_the_caller_when_an_rs256_token_is_valid() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA, &EC])).await;
    let authenticator = provider.authenticator().await;

    // Act
    let principal = authenticator
        .authenticate(&sign(&RSA, &provider.claims()))
        .await;

    // Assert
    let expected = Principal::new(Uuid::nil(), "f81d4fae-7dec")
        .with_email("ada@example.com")
        .with_name("Ada Lovelace")
        .with_roles(["admin", "offline_access"]);
    assert_eq!(principal, Ok(expected));
}

#[tokio::test]
async fn should_accept_the_token_when_it_is_signed_with_es256() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA, &EC])).await;
    let authenticator = provider.authenticator().await;

    // Act
    let principal = authenticator
        .authenticate(&sign(&EC, &provider.claims()))
        .await;

    // Assert
    assert_eq!(principal.map(|p| p.subject), Ok("f81d4fae-7dec".into()));
}

#[tokio::test]
async fn should_reject_the_token_when_it_was_issued_by_another_issuer() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let claims = with(provider.claims(), "iss", json!("https://evil.example.com"));

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert_eq!(result, Err(AuthError::Invalid));
}

#[tokio::test]
async fn should_reject_the_token_when_it_is_for_another_audience() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;

    for aud in [json!("account"), json!(["account", "billing"])] {
        // Act
        let result = authenticator
            .authenticate(&sign(&RSA, &with(provider.claims(), "aud", aud.clone())))
            .await;

        // Assert
        assert_eq!(result, Err(AuthError::Invalid), "{aud}");
    }
}

#[tokio::test]
async fn should_accept_the_token_when_its_audiences_include_ours() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let claims = with(provider.claims(), "aud", json!(["account", AUDIENCE]));

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn should_answer_expired_when_the_token_expired_beyond_the_leeway() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let claims = with(provider.claims(), "exp", json!(now() - 120));

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert_eq!(result, Err(AuthError::Expired));
}

#[tokio::test]
async fn should_accept_the_token_when_it_expired_within_the_leeway() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let claims = with(provider.claims(), "exp", json!(now() - 30));

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn should_reject_the_token_when_it_is_not_valid_yet() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let claims = with(provider.claims(), "nbf", json!(now() + 300));

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert_eq!(result, Err(AuthError::Invalid));
}

#[tokio::test]
async fn should_reject_the_token_when_a_required_claim_is_missing() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;

    for claim in ["exp", "iss", "aud", "sub"] {
        // Act
        let result = authenticator
            .authenticate(&sign(&RSA, &without(provider.claims(), claim)))
            .await;

        // Assert
        assert_eq!(result, Err(AuthError::Invalid), "without {claim}");
    }
}

#[tokio::test]
async fn should_reject_the_token_when_its_algorithm_is_none() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let header = json!({ "alg": "none", "typ": "JWT", "kid": "rsa" });
    let token = format!("{}.{}.", b64(&header), b64(&provider.claims()));

    // Act
    let result = authenticator.authenticate(&token).await;

    // Assert
    assert_eq!(result, Err(AuthError::Invalid));
}

#[tokio::test]
async fn should_reject_the_token_when_it_is_an_hmac_keyed_with_the_public_key() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("rsa".into());
    let public_pem = include_bytes!("fixtures/rsa_public.pem");
    let token = encode(
        &header,
        &provider.claims(),
        &EncodingKey::from_secret(public_pem),
    )
    .expect("an HMAC token");

    // Act
    let result = authenticator.authenticate(&token).await;

    // Assert
    assert_eq!(result, Err(AuthError::Invalid));
}

#[tokio::test]
async fn should_reject_the_token_when_its_signature_is_not_the_named_key() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA, &EC])).await;
    let authenticator = provider.authenticator().await;

    for (key, kid) in [(&ROTATED, "rsa"), (&RSA, "ec")] {
        // Act
        let result = authenticator
            .authenticate(&sign_as(key, kid, &provider.claims()))
            .await;

        // Assert
        assert_eq!(result, Err(AuthError::Invalid), "{} as {kid}", key.kid);
    }
}

#[tokio::test]
async fn should_reject_the_token_when_it_is_not_a_jwt_or_has_no_kid() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let no_kid = encode(
        &Header::new(Algorithm::RS256),
        &provider.claims(),
        &RSA.encoding(),
    )
    .expect("a signed token");

    for token in ["", "opaque-session-token", "a.b.c", no_kid.as_str()] {
        // Act
        let result = authenticator.authenticate(token).await;

        // Assert
        assert_eq!(result, Err(AuthError::Invalid), "{token}");
    }
}

#[tokio::test]
async fn should_accept_a_rotated_key_when_the_provider_publishes_it_after_startup() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    provider.publish(jwks(&[&RSA, &ROTATED]));

    // Act
    let result = authenticator
        .authenticate(&sign(&ROTATED, &provider.claims()))
        .await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(provider.fetches(), 2, "startup + one refetch");
}

#[tokio::test]
async fn should_refetch_the_keys_once_a_minute_at_most_when_kids_are_unknown() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;

    // Act
    let first = authenticator
        .authenticate(&sign_as(&ROTATED, "unknown-1", &provider.claims()))
        .await;
    let second = authenticator
        .authenticate(&sign_as(&ROTATED, "unknown-2", &provider.claims()))
        .await;

    // Assert
    assert_eq!(first, Err(AuthError::Invalid));
    assert_eq!(second, Err(AuthError::Invalid));
    assert_eq!(provider.fetches(), 2, "startup + one refetch");
}

#[tokio::test]
async fn should_keep_accepting_known_keys_when_unknown_kids_arrive() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    let _ = authenticator
        .authenticate(&sign_as(&ROTATED, "unknown", &provider.claims()))
        .await;

    // Act
    let result = authenticator
        .authenticate(&sign(&RSA, &provider.claims()))
        .await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn should_answer_unavailable_when_the_keys_cannot_be_refetched() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let authenticator = provider.authenticator().await;
    provider.down.store(true, Ordering::SeqCst);

    // Act
    let result = authenticator
        .authenticate(&sign(&ROTATED, &provider.claims()))
        .await;

    // Assert
    assert_eq!(result, Err(AuthError::Unavailable));
}

#[tokio::test]
async fn should_read_an_auth0_namespaced_roles_claim_when_configured() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let config = provider
        .config()
        .with_roles_claim(RolesClaim::new("https://shop.example.com/roles"));
    let authenticator = OidcAuthenticator::discover(config)
        .await
        .expect("discovered");
    let claims = with(
        provider.claims(),
        "https://shop.example.com/roles",
        json!(["editor"]),
    );

    // Act
    let principal = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert_eq!(principal.map(|p| p.roles), Ok(vec!["editor".to_string()]));
}

#[tokio::test]
async fn should_accept_an_issuer_with_a_trailing_slash_when_the_provider_publishes_one() {
    // Arrange — Auth0's issuer is `https://<tenant>.auth0.com/`
    let provider = Provider::start_calling_itself(jwks(&[&RSA]), |base| format!("{base}/")).await;
    let authenticator = OidcAuthenticator::discover(provider.config())
        .await
        .expect("discovered");
    let claims = with(
        provider.claims(),
        "iss",
        json!(format!("{}/", provider.issuer)),
    );

    // Act
    let result = authenticator.authenticate(&sign(&RSA, &claims)).await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
}

#[tokio::test]
async fn should_not_start_when_the_provider_is_unreachable() {
    // Arrange
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a free port")
        .port();

    // Act
    let result = OidcAuthenticator::discover(OidcConfig::new(
        format!("http://127.0.0.1:{port}"),
        AUDIENCE,
    ))
    .await;

    // Assert
    match result {
        Err(OidcError::Unreachable { url, .. }) => assert_eq!(
            url,
            format!("http://127.0.0.1:{port}/.well-known/openid-configuration")
        ),
        other => panic!("expected Unreachable, got {:?}", other.err()),
    }
}

#[tokio::test]
async fn should_not_start_when_the_provider_names_another_issuer() {
    // Arrange
    let provider =
        Provider::start_calling_itself(jwks(&[&RSA]), |_| "https://other.example.com".into()).await;

    // Act
    let result = OidcAuthenticator::discover(provider.config()).await;

    // Assert
    assert!(
        matches!(result, Err(OidcError::IssuerMismatch { ref discovered, .. }) if discovered == "https://other.example.com"),
        "{:?}",
        result.err()
    );
}

#[tokio::test]
async fn should_not_start_when_the_issuer_url_is_not_a_provider() {
    // Arrange
    let provider = Provider::start(jwks(&[&RSA])).await;
    let config = OidcConfig::new(format!("{}/realms/missing", provider.issuer), AUDIENCE);

    // Act
    let result = OidcAuthenticator::discover(config).await;

    // Assert
    assert!(
        matches!(result, Err(OidcError::Invalid { .. })),
        "{:?}",
        result.err()
    );
}

#[tokio::test]
async fn should_not_start_when_the_provider_publishes_no_usable_key() {
    // Arrange
    let mut encryption = RSA.jwk();
    encryption.common.public_key_use = Some(jsonwebtoken::jwk::PublicKeyUse::Encryption);
    let mut no_kid = EC.jwk();
    no_kid.common.key_id = None;
    let keys = serde_json::to_value(JwkSet {
        keys: vec![encryption, no_kid],
    })
    .unwrap();
    let provider = Provider::start(keys).await;

    // Act
    let result = OidcAuthenticator::discover(provider.config()).await;

    // Assert
    assert!(
        matches!(result, Err(OidcError::NoSigningKey { .. })),
        "{:?}",
        result.err()
    );
}
