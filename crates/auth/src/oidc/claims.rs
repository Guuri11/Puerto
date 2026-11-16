//! From a verified token's claims to a [`Principal`].

use puerto_nucleo::auth::{ErrorAuth, Principal};
use serde_json::{Map, Value};
use uuid::Uuid;

/// Where the provider puts the caller's roles (`AUTH_ROLES_CLAIM`).
///
/// A name that exists at the top level of the claims is read as is — Auth0's namespaced
/// `https://example.com/roles`, or `permissions`. Otherwise it is a dotted path — Keycloak's
/// `realm_access.roles`, `resource_access.web.roles`. A claim that is absent, or not an array, is
/// no role; elements that are not strings are skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRoles(String);

impl ClaimRoles {
    /// Keycloak's realm roles.
    pub const POR_DEFECTO: &'static str = "realm_access.roles";

    pub fn new(claim: impl Into<String>) -> Self {
        Self(claim.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn leer(&self, claims: &Map<String, Value>) -> Vec<String> {
        let value = match claims.get(&self.0) {
            Some(value) => Some(value),
            None => self
                .0
                .split('.')
                .try_fold(None::<&Value>, |current, segment| {
                    let object = match current {
                        None => claims,
                        Some(value) => value.as_object()?,
                    };
                    object.get(segment).map(Some)
                })
                .flatten(),
        };
        value
            .and_then(Value::as_array)
            .map(|roles| {
                roles
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for ClaimRoles {
    fn default() -> Self {
        Self::new(Self::POR_DEFECTO)
    }
}

/// The caller the claims describe. `id_usuario` is nil: the project's provisioning (ADR-0021 § 3)
/// fills it with the local user's id.
pub(crate) fn principal(
    claims: &Map<String, Value>,
    roles: &ClaimRoles,
) -> Result<Principal, ErrorAuth> {
    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .filter(|sub| !sub.is_empty())
        .ok_or(ErrorAuth::Invalido)?;
    let mut principal = Principal::new(Uuid::nil(), subject).con_roles(roles.leer(claims));
    if let Some(email) = claims.get("email").and_then(Value::as_str) {
        principal = principal.con_email(email);
    }
    if let Some(name) = claims.get("name").and_then(Value::as_str) {
        principal = principal.con_nombre(name);
    }
    Ok(principal)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn claims(value: Value) -> Map<String, Value> {
        value.as_object().cloned().expect("an object")
    }

    #[test]
    fn should_read_keycloak_realm_roles_when_the_claim_is_a_dotted_path() {
        // Arrange
        let claims = claims(json!({ "realm_access": { "roles": ["admin", "offline_access"] } }));

        // Act
        let roles = ClaimRoles::default().leer(&claims);

        // Assert
        assert_eq!(roles, vec!["admin", "offline_access"]);
    }

    #[test]
    fn should_read_keycloak_client_roles_when_the_path_is_deeper() {
        // Arrange
        let claims = claims(json!({ "resource_access": { "web": { "roles": ["editor"] } } }));

        // Act
        let roles = ClaimRoles::new("resource_access.web.roles").leer(&claims);

        // Assert
        assert_eq!(roles, vec!["editor"]);
    }

    #[test]
    fn should_read_an_auth0_namespaced_claim_as_is_when_it_exists_at_the_top_level() {
        // Arrange
        let claims = claims(json!({ "https://shop.example.com/roles": ["admin"] }));

        // Act
        let roles = ClaimRoles::new("https://shop.example.com/roles").leer(&claims);

        // Assert
        assert_eq!(roles, vec!["admin"]);
    }

    #[test]
    fn should_prefer_the_top_level_name_when_a_dotted_path_would_also_match() {
        // Arrange
        let claims = claims(json!({
            "realm_access.roles": ["top"],
            "realm_access": { "roles": ["nested"] },
        }));

        // Act
        let roles = ClaimRoles::default().leer(&claims);

        // Assert
        assert_eq!(roles, vec!["top"]);
    }

    #[test]
    fn should_hold_no_role_when_the_claim_is_absent_or_not_an_array() {
        for value in [
            json!({}),
            json!({ "realm_access": {} }),
            json!({ "realm_access": "admin" }),
            json!({ "realm_access": { "roles": "admin" } }),
        ] {
            assert!(
                ClaimRoles::default()
                    .leer(&claims(value.clone()))
                    .is_empty(),
                "{value}"
            );
        }
    }

    #[test]
    fn should_skip_roles_that_are_not_strings() {
        // Arrange
        let claims = claims(json!({ "permissions": ["read:products", 7, null, "write:products"] }));

        // Act
        let roles = ClaimRoles::new("permissions").leer(&claims);

        // Assert
        assert_eq!(roles, vec!["read:products", "write:products"]);
    }

    #[test]
    fn should_describe_the_caller_when_the_claims_carry_a_profile() {
        // Arrange
        let claims = claims(json!({
            "sub": "f81d4fae",
            "email": "ada@example.com",
            "name": "Ada Lovelace",
            "realm_access": { "roles": ["admin"] },
        }));

        // Act
        let principal = principal(&claims, &ClaimRoles::default());

        // Assert
        let expected = Principal::new(Uuid::nil(), "f81d4fae")
            .con_email("ada@example.com")
            .con_nombre("Ada Lovelace")
            .con_roles(["admin"]);
        assert_eq!(principal, Ok(expected));
    }

    #[test]
    fn should_reject_the_claims_when_there_is_no_subject() {
        for value in [json!({}), json!({ "sub": "" }), json!({ "sub": 7 })] {
            assert_eq!(
                principal(&claims(value.clone()), &ClaimRoles::default()),
                Err(ErrorAuth::Invalido),
                "{value}"
            );
        }
    }
}
