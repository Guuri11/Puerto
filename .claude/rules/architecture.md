# Architecture Guidelines

## Dependency Rule (NEVER break this)

```
Presentation → Infrastructure → Application → Domain
```

- **Domain** depends on NOTHING — pure Rust, only `std`, `async-trait`, `thiserror` and the
  Puerto runtime core `puerto-core` (no driver, no runtime, no I/O: `thiserror`, `async-trait`,
  `serde_json`, `uuid`, `chrono` — ADR-0017, ADR-0018)
- **Application** depends on Domain only — use case implementations (plus `tracing` for logs)
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

When `entity.fields` is defined in `puerto.toml`, the generated structs include custom fields:

```rust
#[derive(Debug, Clone)]
pub struct ProductProps {
    pub name: String,
    pub price: i64,
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: ProductId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    // Custom fields from puerto.toml
    pub name: String,
    pub price: i64,
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
```

When `entity.fields` is empty or absent (backward compatible), entities default to a single `name: String` field.

### Typed ids

Every entity's id is its own newtype, declared in `model.rs` above the entity — passing a
`CustomerId` where a `ProductId` is expected does not compile:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProductId(Uuid);

impl ProductId {
    pub fn generate() -> Self { Self(Uuid::new_v4()) }   // Product::new
    pub fn from_uuid(value: Uuid) -> Self { Self(value) } // a path parameter, a row
    pub fn value(&self) -> Uuid { self.0 }                 // a DTO, a bind
}
// + Display, so `tracing::info!(id = %entity.id, …)` works
```

The id is a `Uuid` only at the edges: `Path<Uuid>` in routes (wrapped with `from_uuid`), `id: Uuid`
in DTOs and `{Entity}Db` rows (`.value()`). Params, the repository port and tests use the newtype.

### Validation in `new()`

- `String` fields are validated with `.trim().is_empty()` → `ValidationError("{field}_empty")`
- `Option<String>` fields are not validated (may be `None`)
- Numeric and other types pass through without domain validation

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

When `entity.fields` is defined in `puerto.toml`, `CreateEntityParams` includes all custom fields (minus system fields like `id`, `created_at`, etc.):

```rust
pub struct CreateProductParams {
    pub name: String,
    pub price: i64,
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

pub struct UpdateProductParams {
    pub id: ProductId,
    pub name: String,
    pub price: i64,
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
```

Custom use cases (added via `puerto generate use-case`) also support typed fields — their Params are defined manually when creating the use case.

### Repository trait (port)

```rust
#[async_trait]
pub trait EntityRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: EntityId) -> Result<Option<Entity>, RepositoryError>;
    async fn save(&self, entity: &Entity) -> Result<(), RepositoryError>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use super::*;

    mock! {
        pub EntityRepository {}
        #[async_trait]
        impl EntityRepositoryTrait for EntityRepository {
            async fn find_by_id(&self, id: EntityId) -> Result<Option<Entity>, RepositoryError>;
            async fn save(&self, entity: &Entity) -> Result<(), RepositoryError>;
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

### Repository error

Repository ports return one shared error, whatever the entity and the adapter (ADR-0015):

```rust
// puerto_core::RepositoryError — crates/core in Puerto's workspace (ADR-0017), #[non_exhaustive]
pub enum RepositoryError {
    Duplicate,    // "repository.duplicate"    — a unique constraint rejected the write
    InvalidData,  // "repository.invalid_data" — a stored row fails the domain's rules
    Unavailable,  // "repository.unavailable"  — the store failed or is unreachable
}
```

Each entity error converts it — `impl From<RepositoryError> for EntityError` (`Duplicate →
Duplicate`, the rest → `RepositoryError`, plus a `_` arm because the enum is `#[non_exhaustive]`)
— so use cases propagate it with `?` and the API keeps answering the entity's codes.
`domain/shared.rs` exists only once a shared value object does: it declares `errors` /
`value_objects`; there is no `shared/mod.rs`.

### Paged lists

`puerto_core::{PageRequest, Page}` (ADR-0016, moved to the runtime by ADR-0017): `PageRequest::new(limit, offset)` — private
fields, limit defaults to 20 and is clamped to `1..=100` — and `Page<T> { items, total, limit,
offset }`. The CRUD port reads `find_all(&self, page: PageRequest) -> Result<Page<Entity>,
RepositoryError>`, `ListEntitiesParams { page }` returns a `Page`, and `GET /entities?limit=&offset=`
answers `EntityPageDto`. Pg: `ORDER BY created_at DESC, id LIMIT $1 OFFSET $2` + `COUNT(*)`.

### Domain events (`[project] events` — ADR-0018)

A project created with `puerto new --db --events db|kafka`. Each **CRUD** entity gets two more
domain files; simple entities (the Greeting demo) are unchanged:

```
domain/<entity>/
  events.rs         # AUTO-GENERATED from puerto.toml, never hand-edited: {Entity}Snapshot
                    #   (DTO-shaped primitives, serde camelCase, from_entity), {Entity}Event
                    #   (Created/Updated/Deleted + custom), impl puerto_core::DomainEvent,
                    #   pub const {ENTITY}_CREATED…
  unit_of_work.rs   # {Entity}UnitOfWork (save / record / commit(self: Box<Self>)) and
                    #   {Entity}UnitOfWorkFactory (begin); mocks::RecordingUnitOfWorkFactory
```

- The repository port **has no `save`**: it only reads. Writes go `uow_factory.begin()` → `save`
  → `record({Entity}Event::X({Entity}Snapshot::from_entity(&e)))` → `commit`. Dropping the UoW
  without `commit` rolls back — an early `?` is the rollback
- `create` holds `uow_factory` only; `get`/`list` `repository` only; `update`/`delete` and every
  custom use case both. `bootstrap.rs` builds `{snake}_repo` and `{snake}_uow` once per entity
- Infrastructure: `unit_of_work.rs` — `Pg{Entity}UnitOfWorkFactory { pool }` over
  `puerto_sqlx::uow::PgTransaction`; the upsert lives there, not in the Pg repository. Its
  `#[sqlx::test]`s check the row and its outbox event commit (and roll back) together
- `events.rs` is rewritten by scaffold, `generate bootstrap` and `generate event`; domain models
  and value objects stay serde-free. The snapshot is a public contract: consumers decode it
- Tests use `RecordingUnitOfWorkFactory` (`failing_at(FailAt::Save, RepositoryError::Duplicate)`,
  `recorded()` → `saved`, `events`, `committed`)

### The worker (`[project] events` — ADR-0018 § 5)

A second binary next to `presentation`, and a composition root like it:

```
worker/src/
  main.rs                 # WorkerConfig::from_env() + Worker::new(config).run(build_worker) — never changes
  generated/registry.rs   # AUTO-GENERATED from [[consumer]]: relay + one consumer each — never hand-edit
  consumers.rs            # pub mod <consumer>; (sorted)
  consumers/<consumer>.rs # the user's EventHandler, written once by `puerto generate consumer`
```

- A consumer matches `event.event_type` against the entity's `{ENTITY}_{VERB}` constants,
  decodes `{Entity}Snapshot` (a decode error → `dead_letter(REASON_PAYLOAD_INVALID, …)`) and
  calls a use case. It must be idempotent: delivery is at-least-once, keyed on `event.id`
- `Ack` / `Retry` (redelivered with the policy's backoff until `MAX_DELIVERIES`) /
  `DeadLetter` — nothing is retired without a `dead_letters` row
- The loops, `/health` + `/health/ready` on `WORKER_PORT` and the SIGTERM drain are
  `puerto-events`'; the `db` transport adapters are `puerto_sqlx::events`, reached through
  `infrastructure::db`
- `kafka` (`puerto-kafka`): the relay still reads `PgOutbox` and dead letters stay in
  `PgDeadLetters`; events go to `<KAFKA_TOPIC_PREFIX>.<aggregate>` keyed by the aggregate id, and
  a consumer is the group `<prefix>.<consumer>`. It hands out one record at a time
  (`EventSubscription::max_batch`): a `Retry` is retried in process and holds back its partition,
  never reordered. A record that is not an envelope is dead-lettered (`event_id` NULL)

---

## Application Layer (`business/src/application/<entity>/`)

**Orchestrates domain logic. Implements use case traits.**

- One file per use case: `application/<entity>/<action>.rs`
- Implements the trait defined in `domain/<entity>/use_cases/<action>.rs`
- Dependencies injected as `Arc<dyn Trait>` — never concrete types
- **Logging with `tracing` directly** — no injected logger, no `Logger` port (ADR-0006).
  `tracing` is already a facade: the subscriber is set up once, by `puerto_web::App::serve`. A use case logs
  `<use_case>: start` and `<use_case>: done` (plus the id when there is one); it never logs the
  request payload, which is user data

```rust
pub struct CreateEntityUseCaseImpl {
    pub repository: Arc<dyn EntityRepositoryTrait>,
}

#[async_trait]
impl CreateEntityUseCaseTrait for CreateEntityUseCaseImpl {
    async fn execute(&self, params: CreateEntityParams) -> Result<Entity, EntityError> {
        tracing::info!("create_entity: start");
        let entity = Entity::new(EntityProps { name: params.name })
            .inspect_err(|e| tracing::warn!(error = %e, "create_entity: rejected"))?;
        self.repository
            .save(&entity)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "create_entity: repository failed"))?;
        tracing::info!(id = %entity.id, "create_entity: done");
        Ok(entity)
    }
}
```

Tests live inside each file in `#[cfg(test)] mod tests { ... }`.

---

## Infrastructure Layer (`infrastructure/src/<entity>/`)

**Adapters for external systems. No business logic.**

Which of the two is generated is inferred from `[project] db` in puerto.toml — set once by
`puerto new --db`. `generate scaffold` has no `--db` flag of its own.

### InMemory (default — `db` absent or false)

```
infrastructure/src/<entity>/
  repository.rs    # InMemory<Entity>Repository — implements domain repository trait
```

### SQLx/Postgres (`db = true`)

```
infrastructure/src/<entity>/
  entity.rs        # <Entity>Db struct (#[derive(FromRow)]) + TryFrom + From conversions
  repository.rs    # Pg<Entity>Repository { pool: PgPool } — implements domain repository trait
infrastructure/src/
  db.rs            # pub use puerto_sqlx::* + the project's MIGRATOR — created once by puerto new --db
infrastructure/migrations/
  <ts>_<name>.sql  # created by puerto generate migration <name>
```

**Rules:**

- `from_repository()` is the **only** entry point from persistence to domain — never call `Entity::new()` inside a repository
- `TryFrom<EntityDb> for Entity` handles row → domain conversion (can fail)
- `From<&Entity> for EntityDb` handles domain → row conversion (infallible)
- `db.rs` is shared across all entities — never duplicated per entity. The pool, `run_migrations`,
  `ping` and `map_sqlx_error` are `puerto-sqlx`'s (ADR-0017), re-exported there; the project keeps
  only `MIGRATOR` (`sqlx::migrate!` embeds the project's own `migrations/` at compile time)
- `entity.rs` and `db.rs` are only present when `db = true` in puerto.toml
- A `HashMap<String, String>` field is a `JSONB` column: the row field is
  `sqlx::types::Json<HashMap<String, String>>`, the `SELECT` overrides the column type
  (`meta as "meta: Json<HashMap<String, String>>"`) and the bind uses `db.meta as _`
- `unique` fields get a partial unique index (`WHERE deleted = false`) — rows are soft-deleted
- Repositories return `RepositoryError`: they log the driver error with their context and convert
  it with `infrastructure::db::map_sqlx_error` (the only place that knows Postgres codes —
  `23505` → `Duplicate`); `to_domain(row)` turns a row `TryFrom` rejects into `InvalidData`
- Never a `map_err` that returns its input unchanged (clippy `manual_inspect` under the generated
  project's `make lint`)

**When `entity.fields` is defined**, `<Entity>Db` includes all custom columns alongside system columns:

```rust
#[derive(Debug, sqlx::FromRow)]
pub struct ProductDb {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    // Custom columns from puerto.toml
    pub name: String,
    pub price: i64,
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}
```

SQL type mappings come from the type registry (see `puerto-toml.md`).

---

## Presentation Layer

**Entry points only. No business logic.**

### Module layout

```
presentation/src/
  main.rs                  # puerto_web::Config::from_env() + App::new(config).serve(…) — never changes
  app.rs                   # the user's hook: configure(routes) — written once by `new`, never regenerated
  generated.rs             # pub mod bootstrap; — static, never changes
  generated/
    bootstrap.rs           # AUTO-GENERATED from puerto.toml — never hand-edit
  api.rs                   # pub mod error; pub mod <entity>; ...
  api/
    error.rs               # IntoErrorResponse trait (local: the orphan rule) — the body is puerto_web::ErrorResponse
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

### DTOs with typed fields

When `entity.fields` is defined, `dto.rs` generates typed request/response structs that map to the entity's custom fields:

```rust
// CreateProductRequest maps to CreateProductParams
#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct CreateProductRequest {
    pub name: String,
    pub unit_price: i64,          // JSON: "unitPrice"
    pub sku: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

// ProductDto maps from the domain model for responses
#[derive(Debug, Object)]
#[oai(rename_all = "camelCase")]
pub struct ProductDto {
    pub id: Uuid,
    pub name: String,
    pub unit_price: i64,
    // ...
}
```

DTOs derive poem-openapi's `Object` (not serde) and the JSON contract is **camelCase**: a
`published_at` field is `publishedAt` on the wire.

Each OpenAPI type derives from the type registry mapping (e.g., `i64` → `integer(int64)`, `Uuid` → `string(uuid)`).

### Rules

- **API First**: update `routes.rs` + `dto.rs` before any implementation
- Domain models are **never** exposed in responses — always map via `EntityDto::from_domain(&entity)`
- Every `ApiResponse` enum has a `from_status(StatusCode, Json<ErrorResponse>) -> Self` helper
- All errors use the shared `puerto_web::ErrorResponse { name, message }` (ADR-0017), built with
  `ErrorResponse::from_code(err.to_string())`: **`name` is the machine-readable code**
  (`product.not_found`) — ADR-0007
- Status mapping: domain validation and invalid value objects → **422**; not found → 404;
  duplicate → 409; repository / unknown → 500. `400` is left to poem for malformed bodies
- Error mappers are exhaustive (no `_` arm) and generated with **one unit test per arm**
- `/health` is liveness only; `/health/ready` probes the dependencies (`infrastructure::db::ping`
  in `--db` projects, handed to `puerto_web::health::probe`) and answers 503 when one is down.
  SIGTERM drains in-flight requests for `SHUTDOWN_DEADLINE` (ADR-0008) — all served by `puerto-web`
- `Config::from_env()` (`puerto-web`) reads `SERVICE_PORT`, `CORS_ALLOWED_ORIGINS`, `PUERTO_BANNER`
  after `.env`; a set but invalid value is a startup error
- DI wiring lives in `generated/bootstrap.rs` — never in `main.rs`; it ends with
  `crate::app::configure(puerto_web::routes(openapi_service(…), &config, ready))`. `app.rs` is the
  user's: no generator writes it
- `{Entity}Api` holds **ports**: `Arc<dyn {Uc}UseCaseTrait>`. Only `bootstrap.rs` (the
  composition root) names `business::application::*`; its `Arc<{Uc}UseCaseImpl>` coerces at the
  struct literal
- Every use case trait ships `mocks::Mock{Uc}UseCase` (same gate as the repository mocks), and the
  CRUD `routes.rs` is generated with route tests: `poem::test::TestClient` over the real
  `OpenApiService`, one mocked use case per test, one test per status path (ADR-0012)

```rust
pub struct EntityApi {
    pub create_entity: Arc<dyn CreateEntityUseCaseTrait>,
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
