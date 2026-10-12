# Presentation Layer — AI Agent Notes

This crate exposes the application via HTTP using `poem-openapi`. It is **just another adapter** —
no business logic lives here.

## Layer Structure

```
presentation/src/
├── main.rs                  # `puerto_web::Config::from_env()` + `App::new(config).serve(…)` (never changes)
├── app.rs                   # Yours: `configure(routes)` — extra middleware and routes; no generator touches it
├── generated.rs             # `pub mod bootstrap;`
├── generated/bootstrap.rs   # AUTO-GENERATED DI wiring + routes — never hand-edit
├── api.rs                   # `pub mod error; pub mod tags; pub mod <entity>;` — no `mod.rs`
└── api/
    ├── error.rs             # The IntoErrorResponse trait (the body, ErrorResponse, is puerto-web's)
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
- **`<Entity>Api` holds ports** — `Arc<dyn CreateProductUseCaseTrait>`, never the `…UseCaseImpl`.
  `routes.rs` imports nothing from `business::application`; only `bootstrap.rs` does.
- Log failures with `tracing::warn!(code = %error.0.name, "<use_case> failed")`.

## Health and shutdown

Served by the Puerto runtime, `puerto-web`; `bootstrap.rs` hands it the readiness check.

- `GET /health` — liveness: the process answers.
- `GET /health/ready` — readiness: `503` until every dependency answers (the database, in a
  `--db` project). Point orchestrator readiness checks here, liveness checks at `/health`.
- On SIGTERM / Ctrl-C the server stops accepting and drains in-flight requests for up to
  `puerto_web::app::SHUTDOWN_DEADLINE` (8s).

## Configuration

`puerto_web::Config::from_env()` reads the environment, after `.env`: `SERVICE_PORT` (default
`8080`), `CORS_ALLOWED_ORIGINS` (comma-separated; none set means any origin), `PUERTO_BANNER`
(`false` hides the banner). A value that is set but invalid stops the server at startup.

## Route Handler Pattern

```rust
#[oai(path = "/products/:id", method = "get", tag = "ApiTags::Product")]
async fn get_by_id(&self, id: Path<Uuid>) -> GetProductResponse {
    match self.get_product.execute(GetProductParams { id: ProductId::from_uuid(id.0) }).await {
        Ok(entity) => GetProductResponse::Ok(Json(ProductDto::from_domain(&entity))),
        Err(err) => {
            let (status, error) = err.into_error_response();
            tracing::warn!(code = %error.0.name, "get_product failed");
            GetProductResponse::from_status(status, error)
        }
    }
}
```

## Route Tests

`routes.rs` ends with a `#[cfg(test)]` module that drives the real `OpenApiService` through
`poem::test::TestClient`, with every use case mocked (`Mock<Uc>UseCase`, from `business` under the
`test-utils` dev-dependency feature) and entities built by the Object Mothers. One test per status
path. A new endpoint gets its tests here, the same way:

```rust
#[tokio::test]
async fn should_respond_404_when_product_does_not_exist() {
    // Arrange
    let mut mocks = Mocks::default();
    mocks.get.expect_execute().returning(|_| Err(ProductError::NotFound));

    // Act
    let response = mocks.client().get(item_path()).send().await;

    // Assert
    response.assert_status(StatusCode::NOT_FOUND);
}
```

## Adding a New Entity Endpoint

Run `puerto generate presentation <Name>` (the entity must already be in `puerto.toml`). It
writes the four files, declares the module in `api.rs` and regenerates `bootstrap.rs`. A use case
added later with `puerto generate use-case` gets a field on `<Entity>Api`; expose it by adding an
endpoint to `routes.rs`.
