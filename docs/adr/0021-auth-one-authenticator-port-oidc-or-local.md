# 0021 — Auth: one `Authenticator` port, two adapters (OIDC, local sessions)

**Status:** Accepted (2026-10-12). Spec: `PLAN.md` § Phase 16. Builds on ADR-0017 (runtime crates:
ports in core, adapters in their own crate, the domain part generated), ADR-0007 (the error
contract) and ADR-0013 (the web frontend).

## Context

Three ways to sign people in were asked for: Auth0, Keycloak and a project's own database. They
are not three adapters. Auth0 and Keycloak are both standard OpenID Connect providers, and for the
API the job is the same with either: it is an OAuth2 *resource server* that verifies a JWT access
token and turns its claims into a caller. The only real difference is where the roles are:

| Provider | Roles claim |
|---|---|
| Keycloak | `realm_access.roles` (realm roles) or `resource_access.<client>.roles` |
| Auth0 | `permissions` (RBAC on), or a namespaced custom claim (`https://example.com/roles`) |

A project's own database is a different thing: there the API *is* the identity provider. It
registers, hashes passwords, opens and closes sessions — later, resets passwords and verifies
addresses.

How the frameworks Puerto is compared with split it:

- **Spring Security** — one `oauth2ResourceServer().jwt()` configured by `issuer-uri`; Auth0 and
  Keycloak are configuration, the roles are mapped by a `JwtAuthenticationConverter`.
- **ASP.NET Core** — one generic `AddJwtBearer(Authority, Audience)`; per-provider differences
  are configuration. Authorization is a separate step: `[Authorize(Roles = "admin")]` or policies.
- **Laravel** — *guards* (how a request is authenticated) are separate from *providers* (where
  the user comes from). A Keycloak guard verifies the JWT and may or may not load a local user.
- **NestJS / Passport** — interchangeable *strategies* behind one `AuthGuard`, roles through a
  `@Roles()` decorator and a guard.
- **Rails 8** — authentication is a **generator**, not a gem: `User`, `Session` and the
  controllers belong to the app, the primitives (`has_secure_password`) stay in the framework.
  Sessions are rows in a `sessions` table.

## Decision

### 1. One port, two adapters

- `puerto_core::auth`: `Principal { user_id: Uuid, subject, email: Option<String>, name:
  Option<String>, roles: Vec<String> }` with `has_role(&str)`; the port `Authenticator {
  async fn authenticate(&self, token: &str) -> Result<Principal, AuthError> }`; `AuthError::
  {Invalid, Expired, Unavailable}` (`#[non_exhaustive]`).
- `puerto-auth` (new runtime crate), feature `oidc`: `OidcAuthenticator` — discovery from
  `AUTH_ISSUER/.well-known/openid-configuration`, the JWKS fetched and cached (refetched on an
  unknown `kid`), signature + `iss` + `aud` + `exp` checked, roles read from `AUTH_ROLES_CLAIM`.
  Auth0, Keycloak or any other OIDC provider is configuration.
- `puerto-auth`, feature `local`: password hashing (argon2id) and session tokens. The sessions
  themselves are `puerto-sqlx`'s `PgSessions`, which is the `Authenticator` of a local project.

A route never knows where its caller came from, and tests use `MockAuthenticator`. A project
picks one with `puerto new --auth oidc|local`; `[project] auth` records it.

### 2. Local sessions are opaque rows, not self-issued JWTs

`POST /auth/login` returns a random 256-bit token; the database keeps its SHA-256 hash, the user and
an expiry. Revoking — logout, a stolen token — is a `DELETE`. There is no signing key to keep or
rotate, and a lookup per request is the cost (Rails 8 pays the same). A self-issued JWT would
save the lookup and lose revocation, and the local mode is never the one that has to scale past
a primary key lookup.

### 3. Every authenticated caller is a local `User` row

A generated `User` entity (`UserId`, `email`, `name`, `roles`, and for OIDC `issuer` + `subject`)
is what other entities point at (`owner: UserId`). It is the same entity in both modes, so moving
from local to OIDC does not touch the domain:

- **OIDC**: `ProvisioningAuthenticator { inner, users }` (generated, infrastructure) looks the
  caller up by `(issuer, subject)` and creates the row on the first request (just-in-time
  provisioning). A short in-memory cache keyed by the subject keeps it off the database on every
  request. **Roles come from the token only**: the column is not read in OIDC projects, so one
  place decides.
- **Local**: the password lives in `credentials (user_id, password_hash)` and the sessions in
  `sessions`, so `User` carries no secret. Roles come from the `roles` column.

`GET /me` answers the user and the roles as the API resolved them, in both modes. The frontend
never reads provider-specific claims. `User` becomes a reserved entity name in auth projects.

### 4. Protected by default; roles are one attribute on the route

- A handler that takes `auth: BearerAuth` (a poem-openapi `SecurityScheme`, `puerto-web`) is
  authenticated: no or a bad token → `401 auth.unauthenticated`. The OpenAPI spec carries the
  `security` requirement, so the frontend's wire types know it.
- `BearerAuth` implements poem-openapi's `ApiExtractor` by hand; only the spec registration comes
  from a private `#[derive(SecurityScheme)]` struct. The derive's checker runs only once a bearer
  header was found: a request without one gets poem's plain-text 401, not an `ErrorResponse`, and
  the checker has no way to answer 503. By hand, every refusal is an `ErrorResponse` JSON body —
  401 `auth.unauthenticated` (no header, `Invalid`, `Expired`), 503 `auth.unavailable`, and 500
  `auth.not_configured` when the routes carry no `Arc<dyn Authenticator>` (`bootstrap.rs` did not
  register one).
- `#[require_role("admin")]` (`puerto-macros`, re-exported by `puerto-web`) on the handler adds a
  `403 auth.forbidden` check at its top. One line, visible, written by the generator and by hand
  alike. The 16.1 spike (poem-openapi 5.1.16) proved it works inside `#[OpenApi] impl`:
  poem-openapi leaves attributes it does not own on the method, so the route still registers and
  its spec entry still carries `security: [{"BearerAuth": []}]`. That holds whichever of the two
  attributes comes first, and clippy `-D warnings` is clean on the expansion. What the macro relies on:
  - the handler has a parameter whose type's last segment is `BearerAuth` (a type alias hides it;
    no parameter is a compile error naming the attribute), and
  - the return type has `from_status(StatusCode, Json<ErrorResponse>)`, which every generated
    response enum has.
  It does not document the 403: the response enum's `Forbidden` variant does. On error the macro
  must still emit the method unchanged, or rustc adds a confusing "no method named …" error.
- Scaffold protects every CRUD route of an auth project. `puerto.toml` records the exceptions:
  `public = [...]` (no `BearerAuth`) and `roles = { use_case = [...] }`.
- Rules that depend on data ("only the owner edits") are not annotations: protected use cases take
  `actor: Principal` in their params and decide, returning `{Entity}Error::Forbidden` → 403. The
  domain sees a `Principal`, never a token.

### 5. The frontend: an `AuthGateway` port, bearer tokens, two adapters

- `domain/auth/session.ts`: `CurrentUser` (from `GET /me`) and the port `AuthGateway {
  currentUser, accessToken, login, logout }`.
- `infrastructure/auth/oidc-auth-gateway.ts` — `oidc-client-ts`, Authorization Code + PKCE, works
  with Auth0 (plus the `audience` parameter) and Keycloak; `local-auth-gateway.ts` — the
  `/auth/*` endpoints.
- `infrastructure/http/client.ts`: an openapi-fetch middleware adds `Authorization: Bearer …`; a
  401 starts `login()`.
- `presentation/core/auth.tsx`: `AuthProvider` + `useAuth()` → `{ user, can(role), login, logout
  }`; the generated route table wraps every non-public route in `<RequireAuth>`; generated screens
  hide actions the user's roles do not allow. The API stays the authority: a 403 reaches the
  existing `ErrorAlert` as `auth.forbidden`.
- Where the token is kept: **in memory for OIDC** (`InMemoryWebStorage`) — a reload redirects to
  the provider, whose own session sends the user straight back; **`localStorage` for local** —
  there is no provider session to fall back on, and an opaque token that leaks is revoked
  server-side.

### 6. Development and tests run against Keycloak

An OIDC project's `docker-compose.yml` gains Keycloak with an imported realm: a public `web` client
(PKCE, redirect to the Vite dev server), an audience mapper for the API and a user with the
`admin` role. `make test/auth` runs `puerto-auth` and a generated project against it. Auth0
cannot run locally; being the same adapter, its claim mapping is covered by unit tests over
signed tokens and a static JWKS.

## Alternatives rejected

- **An adapter per provider (`puerto-auth0`, `puerto-keycloak`).** Same verification twice, a crate
  to add for every new provider, and none of the compared frameworks does it.
- **Self-issued JWTs in local mode.** See § 2.
- **The API as a backend-for-frontend with an HttpOnly cookie.** Safer against XSS, but it adds CSRF
  protection, a server-side session for OIDC too, and a proxy in front of the SPA. Bearer keeps one
  shape for the web, the Swagger UI and any other client.
- **Roles from both the token and the `User` row in OIDC.** Two places deciding; when they
  disagree nobody can say which one won.
- **A `Principal` without a local user (OIDC).** Simpler, but every entity that belongs to someone
  would hold a provider subject, and changing provider would orphan the data.

## Consequences

- Two more runtime crates, `puerto-auth` and `puerto-macros`, released in lockstep before `puerto`.
- An auth project's response enums gain 401/403 variants, its protected params an `actor`, and
  its route tests a `MockAuthenticator`. A project without `--auth` is generated exactly as today.
- Password reset and address verification need a mail sent **from a use case** — the `deliver_now`
  ADR-0020 left out. Local auth is the first feature that needs it.
- One local-user lookup per request in OIDC projects (cached) and one session lookup in local ones.
