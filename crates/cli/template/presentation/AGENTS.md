# Presentation Layer — AI Agent Notes

This crate exposes the application via HTTP using `poem-openapi`. It is **just another adapter** —
no business logic lives here.

## Layer Structure

```
presentation/src/
├── main.rs                  # Tracing subscriber, server, graceful shutdown (never changes)
├── health.rs                # GET /health (liveness) and /health/ready (readiness)
├── generated.rs             # `pub mod bootstrap;`
├── generated/bootstrap.rs   # AUTO-GENERATED DI wiring + routes — never hand-edit
├── api.rs                   # `pub mod error; pub mod tags; pub mod <entity>;` — no `mod.rs`
└── api/
    ├── error.rs             # Shared ErrorResponse + IntoErrorResponse trait
    ├── <entity>.rs          # `pub mod dto; pub mod error_mapper; pub mod responses; pub mod routes;`
    └── <entity>/
        ├── routes.rs        # #[OpenApi] impl with endpoints
        ├── dto.rs           # Request/Response structs (#[derive(Object)], camelCase JSON)
        ├── responses.rs     # #[derive(ApiResponse)] enums
        └── error_mapper.rs  # impl IntoErrorResponse for EntityError (+ one test per arm)
```

## Critical Rules

- **API First**: update `routes.rs` + `dto.rs` **before** the implementation.
- **DTOs never expose domain models** — always map via `EntityDto::from_domain(&entity)`. The JSON
  contract is camelCase (`published_at` → `publishedAt`).
- **Error contract**: every error body is `ErrorResponse { name, message }` built with
  `ErrorResponse::from_code(…)`. `name` is the machine-readable code (`product.not_found`) clients
  branch on. Domain validation (including invalid value objects) → `422`; not found → `404`;
  duplicate → `409`; repository / unknown → `500`. Poem itself answers `400` for a malformed body.
- **Error mappers are exhaustive** (no `_` arm) and every arm has a test, so a status cannot drift.
- **Every `ApiResponse` enum** has a `from_status(StatusCode, Json<ErrorResponse>) -> Self` helper.
- **DI wiring lives in `generated/bootstrap.rs`**, rewritten from `puerto.toml` by
  `puerto generate bootstrap` — never wire by hand.
- Log failures with `tracing::warn!(code = %error.0.name, "<use_case> failed")`.

## Health and shutdown

- `GET /health` — liveness: the process answers.
- `GET /health/ready` — readiness: `503` until every dependency answers (the database, in a
  `--db` project). Point orchestrator readiness checks here, liveness checks at `/health`.
- On SIGTERM / Ctrl-C the server stops accepting and drains in-flight requests for up to
  `SHUTDOWN_DEADLINE` (`main.rs`).

## Route Handler Pattern

```rust
#[oai(path = "/products/:id", method = "get", tag = "ApiTags::Product")]
async fn get_by_id(&self, id: Path<Uuid>) -> GetProductResponse {
    match self.get_product.execute(GetProductParams { id: id.0 }).await {
        Ok(entity) => GetProductResponse::Ok(Json(ProductDto::from_domain(&entity))),
        Err(err) => {
            let (status, error) = err.into_error_response();
            tracing::warn!(code = %error.0.name, "get_product failed");
            GetProductResponse::from_status(status, error)
        }
    }
}
```

## Adding a New Entity Endpoint

Run `puerto generate presentation <Name>` (the entity must already be in `puerto.toml`). It
writes the four files, declares the module in `api.rs` and regenerates `bootstrap.rs`. A use case
added later with `puerto generate use-case` gets a field on `<Entity>Api`; expose it by adding an
endpoint to `routes.rs`.
