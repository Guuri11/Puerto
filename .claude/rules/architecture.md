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
  use_cases.rs      # pub mod <action>;  (sibling to use_cases/)
  use_cases/
    <action>.rs     # Params struct + Trait
```

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
    #[error("entity.repository_error")]
    RepositoryError,
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

- `repository.rs` — implements the domain repository trait
- Start with `InMemory<Entity>Repository`; replace with SQLx adapter when adding a DB
- `from_repository()` is the **only** entry point from persistence to domain — never call `Entity::new()` inside a repository
- Conversion pattern (SQLx): `TryFrom<EntityDb> for Entity` and `From<&Entity> for EntityDb`

---

## Presentation Layer (`presentation/src/api/<entity>/`)

**Entry points only. No business logic.**

Each entity module has **exactly** these four files:

| File | Responsibility |
|------|----------------|
| `routes.rs` | OpenApi impl — endpoints, auth, param parsing |
| `dto.rs` | Request/Response structs (`#[derive(Object)]`) |
| `responses.rs` | `#[derive(ApiResponse)]` enums |
| `error_mapper.rs` | `impl IntoErrorResponse for EntityError` |

### Rules

- **API First**: update `routes.rs` + `dto.rs` before any implementation
- Domain models are **never** exposed in responses — always map via `EntityDto::from_domain(&entity)`
- Every `ApiResponse` enum has a `from_status(StatusCode, Json<ErrorResponse>) -> Self` helper
- All errors use the shared `ErrorResponse { name: String, message: String }` from `api/error.rs`
- Dependencies wired manually in `main.rs` — no DI framework

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

---

## Rust Conventions

- **`snake_case`** — files, modules, variables, functions
- **`PascalCase`** — structs, enums, traits
- **`SCREAMING_SNAKE_CASE`** — constants
- **Never** `unwrap()` or `expect()` outside of tests
- Trait naming: `*RepositoryTrait`, `*UseCaseTrait`
- `From<RepositoryError> for EntityError` to keep use cases clean
