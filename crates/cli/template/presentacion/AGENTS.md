# Presentacion — AI Agent Notes

The binaries and their wiring (ADR-0023 § 7 of Puerto). No business logic lives here: each crate
is a composition root.

## Layout

```
presentacion/
├── cableado/                 # AUTO-GENERATED crate, `Cargo.toml` included — never hand-edit
│   └── src/lib.rs            # per context: its repository, unit of work and use cases, built
├── api/                      # the HTTP binary (named after the project)
│   └── src/
│       ├── main.rs           # `puerto_web::Config::from_env()` + `App::new(config).intentar_servir(…)` (never changes)
│       ├── lib.rs            # `mod app; pub mod generado { pub mod bootstrap; }`
│       ├── app.rs            # Yours: `configure(routes)` — extra middleware and routes
│       └── generado/bootstrap.rs   # AUTO-GENERATED: each context's `<Entity>Api` on `cableado` — never hand-edit
└── worker/                   # `events` projects: relays the outbox, runs the consumers (its own AGENTS.md)
```

## Critical Rules

- **The wiring is generated**: `cableado` and `generado/bootstrap.rs` are rewritten from
  `puerto.toml` by `puerto generar bootstrap` (and every scaffold) — never wire by hand. A new
  context's REST crate is added to `api/Cargo.toml` by the same commands.
- **`cableado` knows no HTTP**: it builds the use cases; `api` mounts the routes on them.
- **`<Entity>Api` holds ports** — `Arc<dyn CreateProductUseCaseTrait>`, never the `…UseCaseImpl`.
- **Error contract**: every error body is `RespuestaError { codigo, mensaje }` built with
  `RespuestaError::from_codigo(…)`. `codigo` is the machine-readable code (`product.not_found`).
  Domain validation (including invalid value objects) → `422`; not found → `404`; duplicate →
  `409`; repository / unknown → `500`. Poem itself answers `400` for a malformed body. Each
  context's `rest_<c>::errores::a_respuesta_error` does the mapping.
- Your own middleware and routes go in `api/src/app.rs`, which no generator touches.

## Health and shutdown

Served by the Puerto runtime, `puerto-web`; `generado/bootstrap.rs` hands it the readiness check.

- `GET /health` — liveness: the process answers.
- `GET /health/ready` — readiness: `503` until every dependency answers (the database, in a
  `--db` project). Point orchestrator readiness checks here, liveness checks at `/health`.
- On SIGTERM / Ctrl-C the server stops accepting and drains in-flight requests for up to
  `puerto_web::app::PLAZO_APAGADO` (8s).

## Configuration

`puerto_web::Config::from_env()` reads the environment, after `.env`: `SERVICE_PORT` (default
`8080`), `CORS_ALLOWED_ORIGINS` (comma-separated; none set means any origin), `PUERTO_BANNER`
(`false` hides the banner). A value that is set but invalid stops the server at startup.

## Route Handler Pattern

In `contextos/<c>/rest/src/rutas.rs`:

```rust
#[oai(path = "/products/:id", method = "get", tag = "ApiTags::Product")]
async fn get_by_id(&self, id: Path<Uuid>) -> GetProductResponse {
    match self.get_product.execute(GetProductParams { id: ProductId::from_uuid(id.0) }).await {
        Ok(entity) => GetProductResponse::Ok(Json(ProductDto::from_domain(&entity))),
        Err(err) => {
            let (status, error) = a_respuesta_error(err);
            tracing::warn!(code = %error.0.codigo, "get_product failed");
            GetProductResponse::from_status(status, error)
        }
    }
}
```

A use case added with `puerto generar caso-de-uso` gets a field on `<Entity>Api`; expose it by
adding an endpoint to `rutas.rs`, with one route test per status.
