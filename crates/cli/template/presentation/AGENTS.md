# Presentation Layer — AI Agent Notes

This crate exposes the application via HTTP using `poem-openapi`. It is **just another adapter** — no business logic lives here.

## Layer Structure

```
presentation/src/
├── main.rs                       # Server bootstrap + manual DI
└── api/
    ├── mod.rs
    ├── error.rs                  # Shared ErrorResponse struct + IntoErrorResponse trait
    └── <entity>/
        ├── mod.rs
        ├── routes.rs             # #[OpenApi] impl with endpoints
        ├── dto.rs                # Request/Response structs (#[derive(Object)])
        ├── responses.rs          # #[derive(ApiResponse)] enums
        └── error_mapper.rs       # impl IntoErrorResponse for EntityError
```

## Critical Rules

- **API First**: update `routes.rs` + `dto.rs` **before** any implementation. Never change an endpoint without updating the contract first.
- **DTOs never expose domain models** — always map via `EntityDto::from_domain(&entity)`.
- **Every `ApiResponse` enum** needs a `from_status(StatusCode, Json<ErrorResponse>) -> Self` helper.
- All error responses use `ErrorResponse { name: String, message: String }` from `api/error.rs`. The `name` field is the machine-readable code for i18n.
- **Dependencies wired manually in `main.rs`** — no DI framework.

## Route Handler Pattern

```rust
#[oai(path = "/entities/:id", method = "get")]
async fn get_entity(&self, id: Path<Uuid>) -> GetEntityResponse {
    match self.get_entity.execute(GetEntityParams { id: id.0 }).await {
        Ok(entity) => GetEntityResponse::Ok(Json(EntityDto::from_domain(&entity))),
        Err(err) => {
            let (status, error) = err.into_error_response();
            GetEntityResponse::from_status(status, error)
        }
    }
}
```

## ApiResponse Pattern

```rust
#[derive(ApiResponse)]
pub enum GetEntityResponse {
    #[oai(status = 200)]
    Ok(Json<EntityDto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl GetEntityResponse {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}
```

## Adding a New Entity Endpoint

Run `puerto generate presentation <Name>` to scaffold this layer and automatically regenerate `bootstrap.rs`. It requires the entity to already exist in `puerto.toml` (added by `puerto generate domain`).

Manual steps if not using the CLI:
1. Create `presentation/src/api/<entity>/` with all 4 files + `<entity>.rs` module declaration
2. Add `pub mod <entity>;` to `presentation/src/api.rs`
3. Run `puerto generate bootstrap` to re-wire DI
