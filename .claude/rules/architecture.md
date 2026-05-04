# Architecture Guidelines

## Dependency Rule (NEVER break this)

```
Presentation → Infrastructure → Application → Domain
```

- **Domain** depends on NOTHING — pure Rust, only `std`, `async-trait`, `thiserror`
- **Application** depends on Domain only — use case implementations
- **Infrastructure** depends on Domain — adapters (DB, HTTP, etc.)
- **Presentation** depends on all layers via manual dependency injection

---

## Domain Layer (`business/src/domain/<entity>/`)

**Pure business logic. No infrastructure or presentation dependencies.**

### Module declaration pattern

No `mod.rs` files. Module hierarchy is declared **inline in `lib.rs`**:

```rust
// business/src/lib.rs
pub mod domain {
    pub mod greeting {
        pub mod errors;
        pub mod model;
        pub mod repository;
        pub mod use_cases;
    }
}
pub mod application {
    pub mod greeting {
        pub mod get_greeting;
    }
}
```

Only leaf files and directories with their own subdirectory need a sibling `.rs` file. `use_cases.rs` exists as a sibling to `use_cases/` and declares its children:

```rust
// business/src/domain/greeting/use_cases.rs
pub mod get_greeting;
```

### File structure per entity

```
domain/<entity>/
  model.rs          # Entity struct + Props struct + constructors
  errors.rs         # Domain error enum (thiserror)
  repository.rs     # Repository trait (port) + mockall mock
  use_cases/
    <action>.rs     # Params struct + Trait
```

No `use_cases.rs` sibling file. Use case modules are declared inline in `business/src/lib.rs`:

```rust
pub mod domain {
    pub mod product {
        pub mod use_cases {
            pub mod create_product;
            pub mod delete_product;
        }
    }
}
```

`puerto generate use-case` patches lib.rs automatically.

### Model constructor pattern

Every domain model has two constructors:

```rust
impl Entity {
    /// Creates a new entity — validates all fields. Use for new entities.
    pub fn new(props: EntityProps) -> Result<Self, EntityError> { ... }

    /// Reconstructs from persistence — bypasses validation. ONLY infrastructure uses this.
    pub fn from_repository(data: Entity) -> Self { ... }
}
```

### Use case file structure

Each file in `domain/<entity>/use_cases/` contains **exactly** three things:

```rust
// 1. Input params
pub struct CreateEntityParams {
    pub name: String,
}

// 2. Use case trait (contract)
#[async_trait]
pub trait CreateEntityUseCaseTrait: Send + Sync {
    async fn execute(&self, params: CreateEntityParams) -> Result<Entity, EntityError>;
}
```

The `Impl` struct lives in the **application layer**, not here.

### Repository trait (port)

```rust
#[async_trait]
pub trait EntityRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Entity>, EntityError>;
    async fn save(&self, entity: &Entity) -> Result<(), EntityError>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use super::*;

    mock! {
        pub EntityRepository {}
        #[async_trait]
        impl EntityRepositoryTrait for EntityRepository {
            async fn find_by_id(&self, id: Uuid) -> Result<Option<Entity>, EntityError>;
            async fn save(&self, entity: &Entity) -> Result<(), EntityError>;
        }
    }
}
```

### Error pattern

```rust
#[derive(Debug, thiserror::Error)]
pub enum EntityError {
    #[error("entity.validation_error.{0}")]
    ValidationError(String),   // "name_empty", "email_invalid", etc.
    #[error("entity.not_found")]
    NotFound,
    #[error("entity.duplicate")]
    Duplicate,
    #[error("entity.repository_error")]
    RepositoryError,
    #[error("entity.unknown")]
    Unknown,
}
```

Error codes are **machine-readable identifiers**, never human-readable messages.

---

## Application Layer (`business/src/application/<entity>/`)

**Orchestrates domain logic. Implements use case traits.**

- One file per use case: `application/<entity>/<action>.rs`
- Implements the trait defined in `domain/<entity>/use_cases/<action>.rs`
- Dependencies injected as `Arc<dyn Trait>` — never concrete types

```rust
pub struct CreateEntityUseCaseImpl {
    pub repository: Arc<dyn EntityRepositoryTrait>,
}

#[async_trait]
impl CreateEntityUseCaseTrait for CreateEntityUseCaseImpl {
    async fn execute(&self, params: CreateEntityParams) -> Result<Entity, EntityError> {
        let entity = Entity::new(EntityProps { name: params.name })?;
        self.repository.save(&entity).await?;
        Ok(entity)
    }
}
```

Tests live inside each file in `#[cfg(test)] mod tests { ... }`.

---

## Infrastructure Layer (`infrastructure/src/<entity>/`)

**Adapters for external systems. No business logic.**

### InMemory (default — `puerto generate scaffold <Name>`)

```
infrastructure/src/<entity>/
  repository.rs    # InMemory<Entity>Repository — implements domain repository trait
```

### SQLx/Postgres (`puerto generate scaffold <Name> --db`)

```
infrastructure/src/<entity>/
  entity.rs        # <Entity>Db struct (#[derive(FromRow)]) + TryFrom + From conversions
  repository.rs    # Pg<Entity>Repository { pool: PgPool } — implements domain repository trait
infrastructure/src/
  db.rs            # create_postgres_pool() — created once by puerto new --db
infrastructure/migrations/
  <ts>_<name>.sql  # created by puerto generate migration <name>
```

**Rules:**

- `from_repository()` is the **only** entry point from persistence to domain — never call `Entity::new()` inside a repository
- `TryFrom<EntityDb> for Entity` handles row → domain conversion (can fail)
- `From<&Entity> for EntityDb` handles domain → row conversion (infallible)
- `db.rs` is shared across all entities — never duplicated per entity
- `entity.rs` and `db.rs` are only present when `db = true` in puerto.toml

---

## Presentation Layer

**Entry points only. No business logic.**

### Module layout

```
presentation/src/
  main.rs                  # 5-line entry point — calls generated::bootstrap::build_app()
  generated.rs             # pub mod bootstrap; — static, never changes
  generated/
    bootstrap.rs           # AUTO-GENERATED from puerto.toml — never hand-edit
  api.rs                   # pub mod error; pub mod <entity>; ...
  api/
    error.rs               # Shared ErrorResponse + IntoErrorResponse trait
    <entity>.rs            # pub mod dto; pub mod routes; pub mod responses; pub mod error_mapper;
    <entity>/
      routes.rs            # OpenApi impl — endpoints, auth, param parsing
      dto.rs               # Request/Response structs (#[derive(Object)])
      responses.rs         # #[derive(ApiResponse)] enums
      error_mapper.rs      # impl IntoErrorResponse for EntityError
```

### `presentation/src/api/<entity>.rs`

Every entity API module **must** have this sibling declaration file:

```rust
// presentation/src/api/product.rs
pub mod dto;
pub mod error_mapper;
pub mod responses;
pub mod routes;
```

`puerto generate scaffold` creates this file automatically. Do not omit it — without it Rust cannot resolve the sub-modules.

### `presentation/src/generated/bootstrap.rs`

**Never edit by hand.** Regenerated from `puerto.toml` by:

- `puerto generate scaffold <Name>` (automatic)
- `puerto generate bootstrap` (manual)

Contains all DI wiring: repo instantiation, use case wiring, `OpenApiService` setup, route registration.

### Rules

- **API First**: update `routes.rs` + `dto.rs` before any implementation
- Domain models are **never** exposed in responses — always map via `EntityDto::from_domain(&entity)`
- Every `ApiResponse` enum has a `from_status(StatusCode, Json<ErrorResponse>) -> Self` helper
- All errors use the shared `ErrorResponse { name: String, message: String }` from `api/error.rs`
- DI wiring lives in `generated/bootstrap.rs` — never in `main.rs`

```rust
pub struct EntityApi {
    pub create_entity: Arc<CreateEntityUseCaseImpl>,
}

#[OpenApi]
impl EntityApi {
    #[oai(path = "/entities", method = "post")]
    async fn create(&self, body: Json<CreateEntityRequest>) -> CreateEntityResponse {
        match self.create_entity.execute(CreateEntityParams { name: body.name.clone() }).await {
            Ok(entity) => CreateEntityResponse::Ok(Json(EntityDto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                CreateEntityResponse::from_status(status, error)
            }
        }
    }
}
```

`EntityApi` fields are wired automatically by `puerto generate scaffold` into `generated/bootstrap.rs`.

---

## Rust Conventions

- **`snake_case`** — files, modules, variables, functions
- **`PascalCase`** — structs, enums, traits
- **`SCREAMING_SNAKE_CASE`** — constants
- **Never** `unwrap()` or `expect()` outside of tests
- Trait naming: `*RepositoryTrait`, `*UseCaseTrait`
