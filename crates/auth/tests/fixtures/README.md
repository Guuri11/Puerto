Test-only fixtures. The PEM keys sign tokens in `tests/oidc.rs` and protect nothing; the Keycloak
realms (`keycloak/`) are imported by `make test/auth` into a throwaway container.
