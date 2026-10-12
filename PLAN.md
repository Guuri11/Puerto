# Puerto — Roadmap

Puerto is a Rust full-stack framework built around DDD + Clean Architecture (Hexagonal / Ports & Adapters), inspired by the developer experience of Laravel and Ruby on Rails.

**Core principles:**

1. Delightful coding experience
2. AI-ready — Rust compiler's tight feedback loop makes LLM-generated code immediately verifiable
3. Convention over configuration

---

## Current state ✅

`puerto new <name>` scaffolds a workspace with three crates mirroring DDD layers, plus a `puerto.toml` and auto-generated DI bootstrap:

```
<name>/
├── puerto.toml                    # Source of truth for entities + use cases
├── business/
│   └── src/
│       ├── domain/greeting/
│       │   ├── model.rs
│       │   ├── errors.rs
│       │   ├── repository.rs      # Trait (port) + mockall mock
│       │   └── use_cases/
│       │       └── get_greeting.rs
│       └── application/greeting/
│           └── get_greeting.rs    # Use case impl + unit tests
├── infrastructure/
│   └── src/greeting/
│       └── repository.rs          # InMemory implementation
└── presentation/
    └── src/
        ├── main.rs                # Config::from_env() + App::new(config).serve(…) — puerto-web
        ├── app.rs                 # The user's hook (middleware, routes) — never regenerated
        ├── generated.rs           # pub mod bootstrap; — never changes
        ├── generated/
        │   └── bootstrap.rs       # AUTO-GENERATED from puerto.toml
        └── api/greeting/
            ├── greeting.rs        # pub mod dto/routes/responses/error_mapper
            ├── routes.rs
            ├── dto.rs
            ├── responses.rs
            └── error_mapper.rs
```

`puerto generate scaffold <Name>` creates all DDD files, patches `lib.rs` files, updates `puerto.toml`, and regenerates `bootstrap.rs` — zero manual wiring.

Since Phase 12 the code identical in every project lives in runtime crates instead of the template
(ADR-0017): `puerto-core` (repository error, paged lists), `puerto-web` (server, config, health,
error body) and, in `--db` projects, `puerto-sqlx` (pool, migrations, ping, driver-error mapping).

Since Phase 13 `puerto new --db --events db|kafka` makes every CRUD write go through
a per-entity unit of work that saves the entity and records its event in an `outbox` table in one
transaction (ADR-0018): `events.rs` is generated from puerto.toml, the repository only reads, and
`puerto generate event` adds custom events. `puerto-core` holds the envelope, the delivery policy
and the transport ports; `puerto-sqlx` the Pg unit of work, outbox and `db` transport;
`puerto-events` the relay, the consume loops and the worker shell. `--events db` projects get a
`worker/` binary that relays the outbox and runs the consumers `puerto generate consumer` adds;
on `--events kafka` it publishes through `puerto-kafka` (one topic per aggregate, one consumer
group per consumer) instead of the `db` transport.

Since Phase 9 a project also gets `web/` — a React + daisyUI frontend on Bun (`--no-web` skips it)
— and, once `cargo test` runs, `docs/api/openapi.json`, the API contract the frontend is typed from
(`make web/sync`). `scaffold` writes each CRUD entity's screens (list, detail, form) for every field
type; `generate web <Name>` writes them alone.

---

## Phase 2 — Generators

### 2.0 `puerto new` — interactive project creation ✅ DONE

**Command signature:**

```
puerto new [--name <name>] [--db]
```

**Behavior:**

- If `--name` is not provided → prompt: `Project name:`
- If `--db` is not provided → prompt: `Include database support (SQLx + Postgres)? [y/N]`
- If both flags are provided → no prompts, fully scriptable (CI-friendly)
- Flags and prompts are independent: any combination works

**Examples:**

```bash
puerto new                          # prompts for both name and db
puerto new --name my-app            # prompts only for db
puerto new --db                     # prompts only for name
puerto new --name my-app --db       # no prompts, creates db project
```

**Test scenarios:**

- Non-interactive: `--name` flag passed to cargo-generate, project name appears in Cargo.toml
- Non-interactive: `--db` flag skips db prompt, db files created
- Non-interactive: both flags, fully silent (no stdin required)
- Interactive path is validated manually (`puerto new` with TTY)

---

### 2.1 `puerto generate scaffold <Name>` ✅ DONE

Creates all files for a new DDD entity across every layer:

```
business/src/domain/<name>/
  model.rs           # Entity struct + Props struct + new() + business rules
  errors.rs          # <Name>Error enum with thiserror
  repository.rs      # <Name>RepositoryTrait + mockall mock
  use_cases.rs       # pub mod <action>;
  use_cases/
    create_<name>.rs # Create use case trait + Params

business/src/application/<name>/
  create_<name>.rs   # UseCaseImpl + unit tests

infrastructure/src/<name>/
  repository.rs      # InMemory<Name>Repository

presentation/src/api/<name>.rs        # pub mod dto/routes/responses/error_mapper
presentation/src/api/<name>/
  routes.rs
  dto.rs
  responses.rs
  error_mapper.rs
```

Auto-patches: `business/src/lib.rs`, `infrastructure/src/lib.rs`, `presentation/src/api.rs`, `puerto.toml`, `presentation/src/generated/bootstrap.rs`.

---

### 2.2 `puerto generate use-case <Entity> <action>` ✅ DONE

Adds a single use case (trait + impl + unit tests) for an existing entity.

**Spec:**

**Files created:**

```
business/src/domain/<entity>/use_cases/<action>.rs   # Params struct + UseCaseTrait
business/src/application/<entity>/<action>.rs         # UseCaseImpl + unit tests
```

**Auto-patches:**

- `business/src/domain/<entity>/use_cases.rs` — append `pub mod <action>;`
- `puerto.toml` — append `<action>` to the entity's `use_cases` array
- `presentation/src/generated/bootstrap.rs` — regenerated

**Behavior:**

- `<Entity>` must be PascalCase; error if not found in `puerto.toml`
- `<action>` must be snake_case
- Generated `UseCaseImpl` struct has one field: `repository: Arc<dyn <Entity>RepositoryTrait>`
- Tests follow AAA pattern with `Mock<Entity>Repository`
- Running twice for same entity+action is a no-op (idempotent)

**Test scenarios to cover:**

- Creates both files with correct content
- Patches `use_cases.rs` without removing existing entries
- Updates `puerto.toml` entity's use_cases array
- Regenerates bootstrap with new use case wired
- PascalCase normalisation (`orderItem` → `OrderItem`)
- Error when entity not in puerto.toml

---

### 2.3 `puerto generate migration <name>` ✅ DONE

> **Superseded in Phase 8.5:** the file is now written directly (no `sqlx` CLI needed), versions
> are unique even within one second, and the command is refused in projects without `--db`.

Wraps `sqlx migrate add` with Puerto conventions.

**Command signature:**

```
puerto generate migration <name>
```

- `<name>` must be snake_case (validated; error + hint if not)

**Pre-flight checks:**

1. `sqlx` binary found in `$PATH` — if not, print:
   ```
   error: sqlx CLI not found
   install it with: cargo install sqlx-cli --no-default-features --features postgres
   ```
   Then exit non-zero.
2. `infrastructure/migrations/` directory is created automatically if it doesn't exist — no error, no manual step needed.

**Files created:**

```
infrastructure/migrations/<timestamp>_<name>.sql
```

- `<timestamp>` = output of `sqlx migrate add` (managed by sqlx)
- File body is created by `sqlx migrate add` — puerto adds a comment header after creation:
  ```sql
  -- Puerto migration: <name>
  -- Run `make sqlx/prepare` after editing this file.
  ```

**No lib.rs patches.** No puerto.toml changes.

**Behavior:**

- Creates `infrastructure/migrations/` if it doesn't exist
- Delegates to `sqlx migrate add <name> --source infrastructure/migrations`
- After creation, prepends comment header to the generated file
- `<name>` normalised: spaces → underscores, lowercased

**Test scenarios:**

- Errors with install instructions when sqlx CLI is absent
- Creates `infrastructure/migrations/` automatically when it doesn't exist
- Normalises name with spaces to underscores

---

### 2.5 `puerto generate scaffold <Name> --crud` ✅ DONE

Generates a complete CRUD entity across all layers instead of a single `create_<name>` use case.

**Command signature:**

```
puerto generate scaffold <Name> --crud
puerto generate scaffold <Name> --crud --db
```

Without `--crud` the command behaves as in 2.1 / 4.1. With `--crud`:

- Generates 5 use case traits + impls (create, get, list, update, delete)
- Repository trait includes `find_all` in addition to `find_by_id` + `save`
- Presentation layer has all 5 HTTP endpoints (POST, GET ×2, PUT, DELETE)
- Compatible with `--db`: generates `PgEntityRepository` with full CRUD SQL

**Files created (delta over base scaffold):**

```
business/src/domain/<entity>/use_cases/
  create_<entity>.rs    # Params + CreateUseCaseTrait
  get_<entity>.rs       # Params (id) + GetUseCaseTrait
  list_<entity>.rs      # Params (unit) + ListUseCaseTrait
  update_<entity>.rs    # Params (id + name) + UpdateUseCaseTrait
  delete_<entity>.rs    # Params (id) + DeleteUseCaseTrait

business/src/application/<entity>/
  create_<entity>.rs    # UseCaseImpl + unit tests (AAA)
  get_<entity>.rs
  list_<entity>.rs
  update_<entity>.rs
  delete_<entity>.rs

infrastructure/src/<entity>/
  repository.rs         # InMemory (--crud) or PgPool (--crud --db)
  entity.rs             # only with --db

presentation/src/api/<entity>/
  dto.rs                # <Entity>Dto + Create<Entity>Request + Update<Entity>Request
  responses.rs          # Create/Get/List/Update/Delete<Entity>Response
  error_mapper.rs
  routes.rs             # <Entity>Api with POST /entities, GET /entities, GET /entities/:id,
                        #   PUT /entities/:id, DELETE /entities/:id
```

**Repository trait (CRUD):**

```rust
async fn find_by_id(&self, id: Uuid) -> Result<Option<Entity>, EntityError>;
async fn find_all(&self) -> Result<Vec<Entity>, EntityError>;
async fn save(&self, entity: &Entity) -> Result<(), EntityError>;
```

**SQLx repository (`--crud --db`):**

- `find_all` → `SELECT ... WHERE deleted = false ORDER BY created_at DESC`
- `find_by_id` → `SELECT ... WHERE id = $1 AND deleted = false`
- `save` → upsert with `ON CONFLICT (id) DO UPDATE`
- Includes `#[cfg(test)] mod integration_tests` with three `#[sqlx::test]` tests

**Auto-patches (same as base scaffold):**

- `business/src/lib.rs` — all 5 use case modules declared inline
- `infrastructure/src/lib.rs`, `presentation/src/api.rs`, `puerto.toml`
- `presentation/src/generated/bootstrap.rs` — all 5 use case impls wired

**Test scenarios:**

- Creates all 5 domain use case files
- Creates all 5 application use case files
- Repository trait contains `find_all`, `find_by_id`, `save`
- Routes file contains POST, GET, PUT, DELETE methods and all 5 use case fields
- `business/src/lib.rs` patched with all 5 use case modules
- `bootstrap.rs` wires all 5 `UseCaseImpl` structs
- Without `--crud`: behaviour identical to 2.1 / 4.1 (no regression)

---

## Phase 3 — Auto-patching lib.rs ✅ DONE

`puerto generate scaffold` automatically updates:

- `business/src/lib.rs` — inline `domain { }` and `application { }` blocks
- `infrastructure/src/lib.rs` — appends new module block
- `presentation/src/api.rs` — appends `pub mod <name>;`
- `puerto.toml` — appends `[[entity]]` block
- `presentation/src/generated/bootstrap.rs` — full regeneration

---

## Phase 4 — Database layer (SQLx) ✅ DONE

### 4.0 `puerto new --db` — project template with database support ✅

Extends the base `puerto new` with database plumbing baked into the generated project.

**Additional files vs. base template:**

```
.env.example                          # DATABASE_URL=postgres://user:pass@localhost/dbname
.cargo/config.toml                    # SQLX_OFFLINE=true (compile without live DB)
infrastructure/migrations/            # Empty dir (sqlx migrate looks here)
infrastructure/src/db.rs              # create_postgres_pool() + run_migrations()
infrastructure/Cargo.toml             # adds sqlx = { ..., features = ["postgres","runtime-tokio-rustls","macros","migrate"] }
```

**`infrastructure/src/db.rs` template:**

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_postgres_pool(database_url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("Failed to connect to database")
}

pub async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("../infrastructure/migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}
```

**Makefile additions (merged into generated Makefile):**

```makefile
db/up:
    docker compose up -d db

sqlx/migrate:
    sqlx migrate run --source infrastructure/migrations

sqlx/add-migration:
    @read -p "Migration name: " name; sqlx migrate add $$name --source infrastructure/migrations

sqlx/prepare:
    cargo sqlx prepare --workspace

sqlx/online:
    SQLX_OFFLINE=false cargo build

setup:
    cargo install sqlx-cli --no-default-features --features postgres --locked
```

**`presentation/src/generated/bootstrap.rs` template change:**

- When any entity has `db = true`: `pub async fn build_app() -> Route` — creates pool internally from `DATABASE_URL`
- When no db entities: `pub fn build_app() -> Route` (sync, unchanged)
- `main.rs` stays 5 lines in both cases; the `.await` difference is handled inside `build_app`

**Test scenarios:**

- `puerto new --db` generates all additional files
- `.cargo/config.toml` contains `SQLX_OFFLINE = "true"`
- `infrastructure/migrations/` directory exists
- `infrastructure/src/db.rs` exists with correct content
- `infrastructure/Cargo.toml` has sqlx dependency with postgres feature

---

### 4.1 `puerto generate scaffold <Name> --db` — entity with SQLx repository ✅

**Command signature:**

```
puerto generate scaffold <Name> --db
```

Without `--db` the command behaves as today (Phase 2.1). With `--db`:

- Instead of `InMemoryEntityRepository`, generates a `PgEntityRepository` backed by SQLx
- Adds `entity.rs` (DB row struct + conversions)
- Runs `puerto generate migration create_<snake>_table` automatically

**Files created (delta over base scaffold):**

```
business/src/domain/<entity>/use_cases/
  create_<entity>.rs          # unchanged — trait + Params (same as InMemory)

business/src/application/<entity>/
  create_<entity>.rs          # unchanged — use case impl (same as InMemory)

infrastructure/src/<entity>/
  entity.rs                   # <Entity>Db struct + TryFrom<EntityDb> for Entity + From<&Entity> for EntityDb
  repository.rs               # PgEntityRepository { pool: PgPool } + impl EntityRepositoryTrait

infrastructure/migrations/
  <timestamp>_create_<snake>_table.sql   # skeleton migration (created by sqlx migrate add)
```

**`infrastructure/src/<entity>/entity.rs` template:**

```rust
use sqlx::FromRow;
use uuid::Uuid;
use business::domain::{snake}::model::{pascal};

#[derive(FromRow)]
pub struct {pascal}Db {
    pub id: Uuid,
    pub name: String,
}

impl TryFrom<{pascal}Db> for {pascal} {
    type Error = business::domain::{snake}::errors::{pascal}Error;

    fn try_from(row: {pascal}Db) -> Result<Self, Self::Error> {
        Ok(Self::from_repository({pascal} {
            id: row.id,
            name: row.name,
        }))
    }
}

impl From<&{pascal}> for {pascal}Db {
    fn from(entity: &{pascal}) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
        }
    }
}
```

**`infrastructure/src/<entity>/repository.rs` template (SQLx):**

```rust
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use business::domain::{snake}::{
    errors::{pascal}Error,
    model::{pascal},
    repository::{pascal}RepositoryTrait,
};
use super::entity::{pascal}Db;

pub struct Pg{pascal}Repository {
    pub pool: PgPool,
}

#[async_trait]
impl {pascal}RepositoryTrait for Pg{pascal}Repository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{pascal}>, {pascal}Error> {
        let row = sqlx::query_as!(
            {pascal}Db,
            "SELECT id, name FROM {snake}s WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| {pascal}Error::RepositoryError)?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{pascal}) -> Result<(), {pascal}Error> {
        let db: {pascal}Db = entity.into();
        sqlx::query!(
            "INSERT INTO {snake}s (id, name) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET name = $2",
            db.id,
            db.name
        )
        .execute(&self.pool)
        .await
        .map_err(|_| {pascal}Error::RepositoryError)?;
        Ok(())
    }
}
```

**`presentation/src/generated/bootstrap.rs` regeneration change (with `--db` entities):**

- Imports `Pg{pascal}Repository` instead of `InMemory{pascal}Repository`
- `build_app(pool: PgPool)` receives pool, clones it per entity repo

**Auto-patches:**

- All patches from base scaffold (lib.rs files, api.rs, puerto.toml)
- `puerto.toml` entity block gets `db = true` flag
- Runs `sqlx migrate add create_{snake}_table --source infrastructure/migrations` as subprocess
- Regenerates `bootstrap.rs` with SQLx wiring

**`puerto.toml` schema addition:**

```toml
[[entity]]
name = "Product"
use_cases = ["create_product"]
db = true          # NEW: controls InMemory vs Pg repository in bootstrap
```

**Behavior:**

- Existing entities without `db = true` continue to use `InMemoryEntityRepository`
- `bootstrap.rs` mixes InMemory and Pg repos correctly (each entity independent)
- Error if `infrastructure/migrations/` does not exist — hint: use `puerto new --db` or create it manually
- Error if sqlx CLI not in `$PATH` — same install instructions as 2.3

**Test scenarios:**

- With `--db`: creates `entity.rs`, `repository.rs` contains `PgPool`, no `InMemory`
- Without `--db`: creates `repository.rs` contains `InMemoryProductRepository` (existing behaviour)
- `puerto.toml` gets `db = true` when `--db` passed
- `bootstrap.rs` uses `Pg{pascal}Repository` for db entities, `InMemory` for others
- Error when sqlx not installed and `--db` passed
- Error when `infrastructure/migrations/` not found and `--db` passed

---

### 4.2 `puerto generate use-case <Entity> <action>` with db entities — unchanged ✅

`puerto generate use-case` does not change in Phase 4. The use case trait + impl are database-agnostic. Bootstrap regeneration already handles the correct repo type via `db` flag in `puerto.toml`.

---

## Phase 5 — Logger abstraction ✅ DONE

Add a `LoggerTrait` domain port so every generated use case can log without coupling to a concrete logging library. Follows the same Ports & Adapters pattern as repositories.

### 5.0 — Logger trait + infrastructure impl ✅

**Goal:** every `puerto new` project ships with a working logger, wired automatically into every use case.

---

#### Domain port

**File:** `business/src/domain/logger.rs`

```rust
pub trait LoggerTrait: Send + Sync {
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
    fn debug(&self, message: &str);
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use super::*;

    mock! {
        pub Logger {}
        impl LoggerTrait for Logger {
            fn info(&self, message: &str);
            fn warn(&self, message: &str);
            fn error(&self, message: &str);
            fn debug(&self, message: &str);
        }
    }
}
```

`business/src/lib.rs` — add at top level:

```rust
pub mod logger;
```

---

#### Infrastructure adapter

**File:** `infrastructure/src/logger.rs`

```rust
use business::logger::LoggerTrait;

pub struct TracingLogger;

impl LoggerTrait for TracingLogger {
    fn info(&self, message: &str)  { tracing::info!("{}", message); }
    fn warn(&self, message: &str)  { tracing::warn!("{}", message); }
    fn error(&self, message: &str) { tracing::error!("{}", message); }
    fn debug(&self, message: &str) { tracing::debug!("{}", message); }
}
```

`infrastructure/src/lib.rs` — add:

```rust
pub mod logger;
```

`infrastructure/Cargo.toml` — add dependency:

```toml
tracing = "0.1"
```

---

#### Presentation / bootstrap

`presentation/Cargo.toml` — add:

```toml
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

`presentation/src/main.rs` — initialize tracing before `build_app()`:

```rust
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```

`presentation/src/generated/bootstrap.rs` — create logger once, clone into every use case:

```rust
use std::sync::Arc;
use infrastructure::logger::TracingLogger;

let logger = Arc::new(TracingLogger);

// use case wiring (example):
let greeting_use_case = Arc::new(GetGreetingUseCaseImpl {
    repository: Arc::new(InMemoryGreetingRepository::new()),
    logger: Arc::clone(&logger),
});
```

---

#### Greeting use case (template update)

`business/src/application/greeting/get_greeting.rs`:

```rust
pub struct GetGreetingUseCaseImpl {
    pub repository: Arc<dyn GreetingRepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

impl GetGreetingUseCaseTrait for GetGreetingUseCaseImpl {
    async fn execute(&self, params: GetGreetingParams) -> Result<Greeting, GreetingError> {
        self.logger.info(&format!("Getting greeting for: {}", params.name));
        // ...
        self.logger.info(&format!("Greeting created: {}", greeting.message));
        Ok(greeting)
    }
}
```

Tests use `MockLogger` from `business::logger::mocks`.

---

#### Scaffold generator update (`crates/cli/src/scaffold.rs`)

The `UC_IMPL` template constant must include the logger field and log calls. Generated use case impls follow this shape:

```rust
pub struct {uc_pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

impl {uc_pascal}UseCaseTrait for {uc_pascal}UseCaseImpl {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Executing {uc}: {:?}", params));
        // ...
        Ok(result)
    }
}
```

`generate_bootstrap_content()` must add:

- `use infrastructure::logger::TracingLogger;`
- `let logger = Arc::new(TracingLogger);`
- `logger: Arc::clone(&logger)` in every use case struct init

---

#### Files changed in template

| File                                                | Change                                         |
| --------------------------------------------------- | ---------------------------------------------- |
| `business/src/domain/logger.rs`                     | NEW — `LoggerTrait` + mockall mock             |
| `business/src/lib.rs`                               | Add `pub mod logger;`                          |
| `business/src/application/greeting/get_greeting.rs` | Add `logger` field + log calls + mock in tests |
| `infrastructure/src/logger.rs`                      | NEW — `TracingLogger`                          |
| `infrastructure/src/lib.rs`                         | Add `pub mod logger;`                          |
| `infrastructure/Cargo.toml`                         | Add `tracing = "0.1"`                          |
| `presentation/src/main.rs.liquid`                   | Add `tracing_subscriber` init                  |
| `presentation/Cargo.toml`                           | Add `tracing-subscriber`                       |
| `presentation/src/generated/bootstrap.rs`           | Wire `TracingLogger` into all use cases        |

#### Files changed in Puerto CLI

| File                         | Change                                               |
| ---------------------------- | ---------------------------------------------------- |
| `crates/cli/src/scaffold.rs` | `UC_IMPL` template: add `logger` field + log calls   |
| `crates/cli/src/scaffold.rs` | `generate_bootstrap_content()`: wire `TracingLogger` |

---

#### Test scenarios

- `puerto new` — generated project compiles with logger wired (`make test/full`)
- `puerto generate scaffold <Name>` — generated use case impl has `logger` field
- `puerto generate use-case <Entity> <action>` — generated impl has `logger` field
- `bootstrap.rs` contains `TracingLogger` import and `logger` wired into every use case
- Mock logger works in unit tests (no real tracing calls)

---

## Phase 6 — IDE Snippets ✅ DONE

Every `puerto new` project ships with snippet files for **Zed** and **VS Code** (VS Code format is also compatible with nvim+LuaSnip). Both files share the same JSON content — Zed and VS Code use the same TextMate snippet format.

### 6.0 Snippet files in `puerto new`

`puerto new` writes two files after cargo-generate completes:

```
.zed/snippets/rust.json           # Zed project-local snippets — auto-loaded by Zed
.vscode/puerto.code-snippets      # VS Code workspace snippets — auto-loaded; LuaSnip-compatible
```

Written by `snippets::apply(base, None)` called from `new_project()` after cargo-generate.

**Zed note:** project snippets at `.zed/snippets/` are loaded automatically — no copying needed.
**LuaSnip note:** add `require("luasnip.loaders.from_vscode").lazy_load({ paths = { "./.vscode" } })` to init.

---

### 6.1 `puerto generate snippets [--ide <ide>]`

Adds or regenerates snippet files in an existing Puerto project.

**Command signature:**

```
puerto generate snippets                 # writes both Zed + VS Code
puerto generate snippets --ide zed       # .zed/snippets/rust.json only
puerto generate snippets --ide vscode    # .vscode/puerto.code-snippets only
```

**Behavior:**

- Overwrites existing files (idempotent)
- `--ide` values: `zed`, `vscode` (error on unknown value)
- Prints file path(s) written + IDE-specific setup note

---

### Snippet inventory

| Prefix                    | Layer          | Description                                        |
| ------------------------- | -------------- | -------------------------------------------------- |
| `lib-domain-entity`       | lib.rs         | Inline domain entity block                         |
| `lib-application-entity`  | lib.rs         | Inline application entity block                    |
| `domain-model`            | domain         | Struct + Props + new(props) + from_repository()    |
| `domain-errors`           | domain         | thiserror enum with machine-readable codes         |
| `repository-trait`        | domain         | RepositoryTrait + Send+Sync + mockall mock         |
| `domain-use-case`         | domain         | Params + UseCaseTrait                              |
| `app-use-case`            | application    | UseCaseImpl + LoggerTrait + unit tests             |
| `lib-infra-entity`        | infra lib.rs   | Infrastructure entity block (InMemory)             |
| `lib-infra-entity-db`     | infra lib.rs   | Infrastructure entity block (SQLx)                 |
| `persistence-entity`      | infrastructure | EntityDb struct + TryFrom/From conversions         |
| `persistence-repo`        | infrastructure | PgEntityRepository + find_by_id + save             |
| `lib-presentation-entity` | api.rs         | Presentation entity module decls                   |
| `poem-dto`                | presentation   | EntityDto (Object) + from_domain()                 |
| `poem-request-dto`        | presentation   | Request DTO struct                                 |
| `poem-response-enum`      | presentation   | ApiResponse enum + from_status()                   |
| `poem-error-mapper`       | presentation   | IntoErrorResponse impl                             |
| `poem-api-struct`         | presentation   | EntityApi struct + POST endpoint                   |
| `poem-endpoint`           | presentation   | Single #[oai] endpoint handler                     |
| `cfg-test`                | test           | #[cfg(test)] mod tests block                       |
| `should-do-test`          | test           | #[tokio::test] with AAA pattern                    |
| `object-mother`           | test           | Object Mother builder pattern                      |
| `sqlx-test`               | test           | #[sqlx::test(migrations="migrations")] single test |
| `sqlx-repo-test-module`   | test           | Full #[cfg(test)] for PgEntityRepository           |

**Puerto adaptations vs ant_backend:**

- `LoggerTrait` (not `Logger`) — already has `Send + Sync`
- Mocks in `repository.rs pub mod mocks` (not separate file)
- `from_repository(data: Entity) -> Self` (not individual fields)
- No `SecurityService`/JWT, no events, no UoW, no SSE handlers
- `EntityError::RepositoryError` terminal (no `From<RepositoryError>`)
- SQL params escaped as `\$1` in snippet JSON to avoid tab-stop conflict

---

### 6.2 SQLx integration tests in `--db` scaffold (already implemented)

`puerto generate scaffold <Name> --db` generates `infrastructure/src/<entity>/repository.rs`
with a `#[cfg(test)] mod integration_tests` block containing three `#[sqlx::test]` tests:

- `should_persist_and_retrieve_by_id`
- `should_return_none_for_nonexistent_id`
- `should_update_entity_on_save_conflict`

Migrations path: `"migrations"` (relative to infrastructure crate root = `infrastructure/migrations/`).

---

### Test scenarios

- `puerto new` → `.zed/snippets/rust.json` exists with valid JSON
- `puerto new` → `.vscode/puerto.code-snippets` exists with valid JSON
- `puerto generate snippets` → overwrites both files (idempotent)
- `puerto generate snippets --ide zed` → only `.zed/snippets/rust.json` created
- `puerto generate snippets --ide vscode` → only `.vscode/puerto.code-snippets` created
- `puerto generate snippets --ide unknown` → error message

---

## Phase 7 — Project-level metadata & DX improvements

### 7.0 `[project] db = true` in puerto.toml ✅ DONE

Track project-level database support in `puerto.toml` so tools and future generators can read whether the project was created with `--db` without inspecting generated files.

**Schema addition:**

```toml
[project]
name = "my-app"
db = true          # present only when puerto new --db was used
```

**Rules:**

- `puerto new --db` sets `project.db = true` via `apply_db_to_new_project`
- `puerto new` (no db) omits the field entirely (serialised with `skip_serializing_if`)
- `puerto generate scaffold <Name>` can read this flag in the future to default `--db` behaviour
- `puerto generate scaffold <Name> --db` errors when `project.db` is absent (future enforcement)

**Files changed:**

- `crates/cli/src/puerto_toml.rs` — add `db: bool` to `Project` struct
- `crates/cli/src/scaffold.rs` — `apply_db_to_new_project` sets `project.db = true`

**Test scenarios:**

- `puerto new --db` → `puerto.toml` contains `db = true` under `[project]`
- `puerto new` (no db) → `puerto.toml` does not contain `db = true`

---

### 7.1 HTTP-level request logging via poem Tracing middleware ✅ DONE

Every generated project logs HTTP requests automatically via poem's built-in `Tracing` middleware — no user configuration needed.

**Invariant:** ALL generated code logs through `LoggerTrait` (never `tracing::` directly). The middleware is the only exception: it uses `tracing::` internally and is wired once in `bootstrap.rs`.

**Changes to `bootstrap.rs` generation (`generate_bootstrap_content`):**

- `build_app()` is always `pub async fn` (regardless of db flag)
- Return type is `impl poem::Endpoint` (not `Route`) — required for `.with(Tracing)` to type-check
- Route wiring ends with `.with(Tracing)`:
  ```rust
  Route::new().nest("/api", api_service).nest("/", ui).with(Tracing)
  ```
- Each repo binding is annotated as `Arc<dyn {Pascal}RepositoryTrait>` to satisfy trait-object coercion
- Imports `use poem::middleware::Tracing;`

**Changes to template `bootstrap.rs`:**

- Same as above for the initial `Greeting` entity

**Changes to generated use case impls (scaffold generator):**

- `CREATE_USE_CASE_IMPL`: `warn` on `ValidationError`, `error` on repo error
- `GET_USE_CASE_IMPL`: `warn` on `NotFound`, `error` on repo error
- `LIST_USE_CASE_IMPL`: `error` on repo error
- `UPDATE_USE_CASE_IMPL`: `warn` on `NotFound` + `ValidationError`, `error` on repo errors
- `DELETE_USE_CASE_IMPL`: `warn` on `NotFound`, `error` on repo errors (unused `model::` import removed)

**Changes to generated infrastructure repos:**

- `INFRA_REPOSITORY` / `CRUD_INFRA_REPOSITORY`: `debug` log on each operation
- `INFRA_DB_REPOSITORY` / `CRUD_INFRA_DB_REPOSITORY`: `error` log on DB errors via `map_err`

**Changes to generated routes (presentation):**

- `{Pascal}Api` struct gains `pub logger: Arc<dyn LoggerTrait>` field
- Each endpoint handler calls `self.logger.warn(...)` on error before returning

**`main.rs.liquid`:** calls `.run(generated::bootstrap::build_app().await)` (with `.await`)

**Cargo.toml changes:**

- `presentation/Cargo.toml.liquid`: `poem-openapi` gains `uuid` feature; adds `uuid = { version = "1", features = ["v4"] }`
- `infrastructure/Cargo.toml.liquid`: `uuid` gains `serde` feature; `chrono` gains `serde` feature

**What the logs look like at runtime (RUST_LOG=info):**

```
INFO request{remote_addr=127.0.0.1 method=POST uri=/api/products}: infrastructure::logger: Creating product: Widget
INFO request{remote_addr=127.0.0.1 method=POST uri=/api/products}: infrastructure::logger: Product created: Widget
INFO request{remote_addr=127.0.0.1 method=POST uri=/api/products}: poem::middleware::tracing_mw: response status=201 Created duration=56ms
```

App logs appear nested inside the request span — every log line automatically carries request context.

**Test scenarios:**

- `scaffold_bootstrap_wires_logger_for_all_entities` — verifies `Arc::clone(&logger)` count (3 per entity)
- `make test/full` — generated project compiles with middleware wired
- Demo project (`puerto-demo`) verified end-to-end: POST/GET products (SQLx), GET orders (InMemory)

---

### 7.2 Spring Boot-like startup banner ✅ DONE

Print a Puerto ASCII banner on startup, similar to Spring Boot's banner:

```
  _   _             _
 | | | | __ _ _ __| |__   ___  _ __
 | |_| |/ _` | '__| '_ \ / _ \| '__|
 |  _  | (_| | |  | |_) | (_) | |
 |_| |_|\__,_|_|  |_.__/ \___/|_|

 :: Puerto ::  (v0.3.0)
```

Printed to stdout before `build_app()` is called, controlled by `HARBOR_BANNER=false` env var to suppress in tests.

Options to evaluate:

- Static string in `main.rs.liquid` template
- Read version from `CARGO_PKG_VERSION` at compile time
- Optional: color via `colored` crate (adds dependency — may not be worth it)

---

### 7.3 `puerto new --no-demo` — skip the Greeting demo entity ✅ DONE

By default `puerto new` includes a `Greeting` entity to show the project structure in action. With `--no-demo` the project is created empty — no entities, no demo code.

**Command signature:**

```
puerto new                              # prompts for db + demo
puerto new --name my-app --no-demo
puerto new --name my-app --db --no-demo
```

**Behavior:**

- Without `--no-demo` and in a TTY → prompts: `Include Greeting demo entity? [Y/n]`
- Without `--no-demo` and non-interactive → includes demo (default)
- With `--no-demo`: project is minimal — no entity files, empty `puerto.toml` (just `[project]`), empty module declarations, minimal `bootstrap.rs`

**What changes when `--no-demo` is set:**

Files omitted:

```
business/src/domain/greeting/
business/src/application/greeting/
infrastructure/src/greeting/
presentation/src/api/greeting/
presentation/src/api/greeting.rs
```

Files generated differently:

- `puerto.toml` — no `[[entity]]` block
- `business/src/lib.rs` — empty `domain {}` and `application {}` blocks
- `infrastructure/src/lib.rs` — no entity module
- `presentation/src/api.rs` — empty (or just `// Add entities with: puerto generate scaffold <Name>`)
- `presentation/src/generated/bootstrap.rs` — minimal: empty OpenApiService, no use case wiring

**Implementation approach:**

- Add `--no-demo` flag to `NewArgs` in `main.rs`
- Pass `include_demo = !no_demo` as a cargo-generate template variable
- Use `[conditional.'!include_demo'] ignore = [...]` in `cargo-generate.toml` to skip greeting files
- Make `lib.rs`, `api.rs`, and `bootstrap.rs` liquid templates with `{% if include_demo %}` guards

**Test scenarios:**

- `puerto new --no-demo` → no greeting files exist
- `puerto new --no-demo` → `puerto.toml` has no `[[entity]]` block
- `puerto new --no-demo` → generated project compiles clean
- `puerto new` (no flag) → greeting files present (no regression)
- `puerto new --no-demo` followed by `puerto generate scaffold Player --crud` → compiles clean

---

### 7.4 `puerto generate scaffold <Name>` — infer db and crud from context ✅ DONE

**Problem:** the current CLI asks the user to declare things Puerto already knows.

- `--db` is redundant: `puerto.toml` already has `[project] db = true`
- `--crud` is redundant: `scaffold` semantically means full CRUD. Single use cases already have `puerto generate use-case`

**Redesign:**

```bash
# Before (noisy)
puerto generate scaffold Team --crud --db

# After (inferred)
puerto generate scaffold Team
```

**New behavior:**

- `db` → auto-detected from `puerto.toml` `[project] db`
  - `project.db = true` → generates `PgTeamRepository`
  - no db → generates `InMemoryTeamRepository`
- `crud` → always full CRUD (create, get, list, update, delete)
  - for a single use case, use `puerto generate use-case <Entity> <action>` (unchanged)

**Breaking change:** removes `--crud` and `--db` flags from `puerto generate scaffold`.

**Migration:** any existing scripts using `puerto generate scaffold <Name> --crud --db` should drop both flags. Behavior is identical.

**Also fix:** `puerto generate scaffold` with db should auto-create the migration (bug from Phase 4.1 — `run_migration()` was never called from `run()`).

**Updated command signature:**

```bash
puerto generate scaffold <Name>    # CRUD + db inferred from puerto.toml
```

**Test scenarios:**

- `puerto generate scaffold Team` in db project → `PgTeamRepository`, 5 use cases, migration created
- `puerto generate scaffold Team` in non-db project → `InMemoryTeamRepository`, 5 use cases
- `--crud` and `--db` flags removed — passing them is an error
- Migration auto-created when project has db

---

## Phase 8 — Layer-by-layer generators ✅ DONE

Four focused generators that let users (and AI agents) work on one DDD layer at a time. Keeps model context tight — instead of scaffolding all layers at once, each command has a single responsibility.

### Commands

```bash
puerto generate domain <Name>           # domain layer + Object Mother
puerto generate application <Name>      # application layer (validates entity in puerto.toml)
puerto generate repository <Name>       # infrastructure layer (validates entity in puerto.toml)
puerto generate presentation <Name>     # presentation layer + bootstrap regeneration
```

`puerto generate scaffold` remains the all-in-one shortcut.

### Files generated per command

**`puerto generate domain <Name>`**

```
business/src/domain/<snake>/model.rs
business/src/domain/<snake>/errors.rs
business/src/domain/<snake>/repository.rs        # CRUD trait + mockall mock
business/src/domain/<snake>/use_cases/create_<snake>.rs
business/src/domain/<snake>/use_cases/get_<snake>.rs
business/src/domain/<snake>/use_cases/list_<snake>.rs
business/src/domain/<snake>/use_cases/update_<snake>.rs
business/src/domain/<snake>/use_cases/delete_<snake>.rs
business/src/tests/mothers/<snake>_mother.rs     # Object Mother
```

Auto-patches: `business/src/lib.rs` (domain block + `tests::mothers` block), `puerto.toml`.

**`puerto generate application <Name>`**

```
business/src/application/<snake>/create_<snake>.rs
business/src/application/<snake>/get_<snake>.rs
business/src/application/<snake>/list_<snake>.rs
business/src/application/<snake>/update_<snake>.rs
business/src/application/<snake>/delete_<snake>.rs
```

Auto-patches: `business/src/lib.rs` (application block).

**`puerto generate repository <Name>`**

```
infrastructure/src/<snake>/repository.rs         # InMemory or Pg (reads project.db)
infrastructure/src/<snake>/entity.rs             # only when db = true
infrastructure/migrations/<ts>_create_<snake>_table.sql  # only when db = true
```

Auto-patches: `infrastructure/src/lib.rs`.

**`puerto generate presentation <Name>`**

```
presentation/src/api/<snake>.rs
presentation/src/api/<snake>/routes.rs
presentation/src/api/<snake>/dto.rs
presentation/src/api/<snake>/responses.rs
presentation/src/api/<snake>/error_mapper.rs
```

Auto-patches: `presentation/src/api.rs`. Regenerates `bootstrap.rs`.

### Object Mother (also added to `puerto generate scaffold`)

Every entity now gets `business/src/tests/mothers/<snake>_mother.rs`:

```rust
let w  = WidgetMother::random();
let ws = WidgetMother::random_vec(5);
let p  = WidgetMother::new().with_empty_name().build_props();
```

### Validation

- `generate application/repository/presentation`: error if entity not in `puerto.toml` — hint: run `puerto generate domain <Name>` first
- `generate domain`: error if entity already in `puerto.toml`
- After each command: print `Next: puerto generate <next-layer> <Name>` hint

### Idempotency

All patching functions (`patch_business_lib_domain_crud`, `patch_business_lib_application_crud`, `patch_infra_lib`, `patch_api_rs`, `patch_mothers_lib`) are now idempotent — safe to re-run.

---

## Phase 8.5 — Generated-project hardening (review fixes) ✅ DONE

An end-to-end review generated real projects with every option and found output that did not
compile, or did not pass the generated project's own `make lint`. Root cause: the compile matrix
never ran clippy, never generated a `--db` project, and never ran `generate use-case` after
`scaffold`.

### Behaviour

- **Generated code is clippy-clean** (`clippy --all-targets -D warnings`): `inspect_err` instead of
  pass-through `map_err`; shorthand init for VO props; no argument-less `format!`; enum VOs
  implement `std::str::FromStr` (`.parse::<Vo>()`) instead of an inherent `from_str`; test modules
  and Object Mothers import only what their literals name; Mothers implement `Default`; response
  enums allow `large_enum_variant` (documented in the template).
- **`generate use-case <Entity> <action>`**: `<action>` must be snake_case; refused when the action
  already exists (in puerto.toml or on disk — including the scaffolded CRUD ones) and nothing is
  written; adds `pub <action>: Arc<…UseCaseImpl>` to `{Entity}Api` in `routes.rs`. `scaffold
  --force` and `generate presentation` re-add the fields of custom use cases after rewriting
  `routes.rs`.
- **`generate value-object <Name> <type>`**: name PascalCase, type in the shared-VO list
  (`DateTime` shorthand), refused if already declared.
- **`generate migration`**: refused without `[project] db = true`; version = now, or one second
  past the newest existing migration.
- **Names**: duplicate field names rejected; in `--db` projects PostgreSQL reserved words are
  rejected as field names (CLI and `puerto validate`); `puerto new --name` must be
  `[a-z][a-z0-9_-]*`.
- **`--db` projects**: `HashMap<String, String>` columns go through `sqlx::types::Json` (sqlx `json`
  feature enabled); Pg repository tests import what their seed names, seed timestamps at whole
  seconds (Postgres stores µs) and vary `unique` fields per seed; unique indexes are partial
  (`WHERE deleted = false`); `.sqlx/` is no longer git-ignored (builds run with `SQLX_OFFLINE=true`).
- **Create use case** logs the value of its first field (`{:?}`), not the literal `params.x`.
- `puerto new` "next steps" prints `make run` (the package is `presentation`, not the project name).

### Test scenarios

- Matrix runs `cargo clippy --workspace --all-targets -- -D warnings` before `cargo test` in every
  generated project.
- New matrix scenarios: enum VO as the only String field; custom use case after scaffold;
  `postgres_project_with_every_type…` (every primitive, VO kind, shared VO, unique fields, custom
  use case) against Postgres via `make test/db`.
- Structural: use-case refusal / snake_case / routes field (+ survives `--force`); value-object
  validation; migration refusal without db and distinct versions within one second; SQL reserved
  words (db vs no-db, CLI and validate); duplicate fields; project name; create log; enum `as_str`
  assertion; partial unique index; JSONB mapping; sqlx `json` feature + `.gitignore`; Pg seed
  imports and unique variation.

---

## Phase 8.6 — document-api conventions (T-31 … T-38) ✅ DONE

Adopted from the comparison with `document-api` (`puerto_vs_document_api.md` §6). Every decision
has an ADR in `docs/adr/`.

- **Logger port removed** (ADR-0006): `tracing` macros directly in every layer; no `LoggerTrait`,
  `TracingLogger`, `logger` fields or `silent_logger()` mocks. Entity names `Tracing` and `Db` are
  rejected (module collisions).
- **T-37 Error contract** (ADR-0007): `ErrorResponse::from_code`, code in `name`, validation and
  invalid VOs → 422, exhaustive mappers with one generated test per arm.
- **T-38 Health** (ADR-0008): `/health`, `/health/ready` (pings the pool in `--db` projects),
  graceful shutdown on SIGTERM with an 8 s deadline.
- **T-35 Toolchain** (ADR-0009): `rust-toolchain.toml` (1.95.0) + edition 2024 + resolver 3 in
  generated projects; let-chains where clippy now asks for them.
- **T-31 / T-32 Agent rules** (ADR-0010): root `AGENTS.md` + `CLAUDE.md` in every project; the
  per-crate files rewritten (no `mod.rs`, real DI location, real error contract); tests that they
  never describe `mod.rs` or the logger port; the template's `bootstrap.rs` tested against the
  generator.
- **T-33 Risk tiers**: `.claude/rules/workflow.md` § Risk tiers (Puerto) and the root `AGENTS.md`
  (generated projects). One commit per piece, in both.
- **T-34 ADRs**: `docs/adr/` with 0001–0010.
- **T-36 Mutation testing**: `make test/mutation` (`.cargo/mutants.toml`, scoped to validation,
  migrations, types, naming, patchers; `TMPDIR` on disk — a tmpfs `/tmp` runs out mid-link).
  First run: 351 mutants, **50 survivors**. Fixed by deleting dead code hidden behind blanket
  `#[allow(dead_code)]` in `types.rs` (5 functions), a redundant guard in `field_vo_constructor`,
  extracting `timestamp_from_unix_secs` so the clock logic is testable, and exact-output tests for
  the patchers. Final run: 335 mutants, **332 caught, 0 survivors**, 3 unviable.

- **Snippets verified** (0.10.1): fixed to match the generated code, six added, and two compile
  scenarios build an entity from snippets alone (`snippets_compose_…`, `postgres_snippets_…`).

### Test scenarios

Structural: no logger in any generated file (CLI + `--no-demo`); mapper names the code, maps to
422, one test per arm; responses carry 422 and no 400; `ErrorResponse::from_code`; health routes
and graceful shutdown; readiness pings the pool in `--db`; toolchain pin + edition 2024 + resolver
3; agent rules free of `mod.rs` and the logger port; root `AGENTS.md`/`CLAUDE.md`; template
`bootstrap.rs` identical to the generator's. Compile matrix and `make test/db` unchanged in shape —
they now run on the pinned toolchain and edition 2024.

---

## Phase 8.7 — Breaking batch before the first users (T-26, T-13, T-20) ✅ DONE

Three changes to the shape of generated output, done together while no generated project exists
yet (no migration guide needed — see `puerto_arch_plan.md`). One commit each, `!` in the type.

### T-26 — Presentation holds ports, routes are tested

- `{Entity}Api` fields are `Arc<dyn {Uc}UseCaseTrait>`; `routes.rs` no longer imports
  `business::application::*` (only `bootstrap.rs`, the composition root, does). `bootstrap.rs`
  is unchanged: `Arc<{Uc}UseCaseImpl>` coerces at the struct literal.
- Every use case trait file (scaffolded CRUD, `generate use-case`, the template's `get_greeting`)
  ships a `mocks` module behind `#[cfg(any(test, feature = "test-utils"))]` with
  `Mock{Uc}UseCase` — same shape as the repository mocks.
- Object Mothers are compiled under `#[cfg(any(test, feature = "test-utils"))]` so other crates'
  tests can build entities.
- `presentation/Cargo.toml` dev-dependencies: `business` with `test-utils`, `poem` with `test`.
- `generate use-case` adds `pub {uc}: Arc<dyn {Uc}UseCaseTrait>` (and imports the trait).
- CRUD `routes.rs` gets a `#[cfg(test)]` module driving the real `OpenApiService` through
  `poem::test::TestClient` with mocked use cases, one test per status path: create 201 / 422,
  get 200 / 404, list 200 / 500, update 200 / 404, delete 204 / 404. The template's
  `GreetingApi` gets 200 / 422.

Test scenarios: structural — routes hold `Arc<dyn …UseCaseTrait>` and import no
`application::`; every trait template has `Mock{Uc}UseCase`; mothers block gated on
`test-utils`; presentation dev-dependencies; patched custom use case field is `dyn`; routes test
module present with each status. Compile matrix runs the route tests in every scenario.

### T-13 — Real plurals for tables and paths

- `naming::pluralize(snake)` pluralises the last word: `category` → `categories`, `box` →
  `boxes`, `person` → `people`, `order_item` → `order_items`, `status` → `statuses`; a few
  uncountables (`data`, `information`, `equipment`, `news`, `series`, `species`) stay as they are.
- Used for the SQL table (migration + Pg repository queries) and the REST paths (`/categories`,
  `/categories/:id`) and the list endpoint doc comment.

Test scenarios: unit tests for each rule; migration `CREATE TABLE categories`; Pg repository
queries `FROM categories`; routes path `/categories`.

### T-20 — `list_{plural}` use case

- The list use case is `list_{plural}` (`list_products`, `ListProductsUseCaseTrait`,
  `ListProductsParams`, `ListProductsUseCaseImpl`, `ProductApi.list_products`) — in puerto.toml,
  file names, lib.rs patches, conflict guard and every template. One helper
  (`naming::crud_use_cases`) is the single source of the five names.

Test scenarios: scaffold writes `list_products.rs` in domain and application; puerto.toml lists
`list_products`; lib.rs declares `pub mod list_products;`; routes/bootstrap name
`list_products`; `OrderItem` → `list_order_items`.

---

## Phase 9 — Full-stack (frontend) — `web/` ✅ DONE

**Status:** ✅ DONE — 9.0 (`33d4113`) · 9.1 (`4ea452e`) · 9.2 (`752dd04`) · 9.3. Next: Phase 10
(optional, triggered by the first non-trivial validation rule).

Rails-style: `scaffold` leaves a working CRUD screen per entity, and the project carries the
structure to grow the frontend by hand. Same Dependency Rule as the backend, but **no layer
without logic**: the generated CRUD has a domain (types + port), an HTTP adapter and the screens —
`application/` exists empty until a real use case appears. Why this stack, and why not WASM /
HTMX / openapi-generator: [ADR-0013](docs/adr/0013-web-frontend-react-daisyui-bun.md).

### Stack (pinned in the template's `package.json`, exact versions)

| Concern | Choice | Not |
|---|---|---|
| Runtime + package manager | **Bun** (`packageManager` field) | pnpm / Node |
| UI | **React 19** + **daisyUI 5** on Tailwind v4 (CSS classes, no JS) | shadcn |
| Routing | **wouter** (~2 KB, hooks) — route table generated | TanStack Router |
| Server state | `useAsync` hook in `presentation/core/hooks/` | TanStack Query |
| Forms | native `<form>` + controlled state | form libraries |
| Wire types | **openapi-typescript** (types only) + **openapi-fetch** (~6 KB client) | openapi-generator, Orval |
| Bundler / dev server | Vite (`/api` proxied to `:8080` — no CORS in dev) | — |
| Tests | `bun test` + happy-dom + `@testing-library/react` | Vitest |
| Lint / format | Biome | ESLint + Prettier |
| Dependency Rule | dependency-cruiser (`bun run lint:arch`), as in bitacora | — |

`bun test` with happy-dom and testing-library was verified in 9.0 (no Vitest). Tool scripts run as
`bun --bun <tool>`; TypeScript stays on 5.x — both explained in ADR-0013 § Verified in 9.0.

### Generated layout (entity `Product`)

**[G]** regenerated from puerto.toml on every scaffold / `generate bootstrap` — never hand-edit ·
**[S]** written once by scaffold, then the user's · **[T]** from the `puerto new` template.

```
docs/api/openapi.json                              [G] written by the Rust spec test
web/
  package.json · tsconfig.json · vite.config.ts · biome.json
  .dependency-cruiser.cjs · index.html             [T]
  src/
    main.tsx · index.css                           [T] @import "tailwindcss"; @plugin "daisyui";
    domain/product/
      model.ts                                     [S] Product, ProductDraft, enum VO unions
      errors.ts                                    [S] ProductError { code } — code = ErrorResponse.name
      repository.ts                                [S] ProductRepository (port): list/get/create/update/remove
    application/
      README.md                                    [T] where use cases go once there is logic
    infrastructure/
      http/client.ts                               [S] createApiClient() + apiErrorCode() — written
                                                       with the first slice, never overwritten
      http/schema.d.ts                             [G] openapi-typescript
      product/product-repository-http.ts (+ .test.ts)  [S] DTO ↔ model, ErrorResponse → ProductError
    presentation/
      core/generated/dependencies.ts               [G] composition root: Dependencies + createDependencies()
      core/generated/routes.tsx                    [G] AppRoutes (wouter <Switch>) + nav entries
      core/dependencies.tsx                        [T] DependenciesProvider + useDependencies()
      core/hooks/use-async.ts                      [T] AsyncState<T>: loading | error | ok
      shared/ui/                                   [T] layout · error-alert · empty-state · confirm-dialog
      pages/home.tsx                               [T] links to each entity and the Swagger UI
      features/product/
        containers/ product-list-container.tsx · product-detail-container.tsx
                    product-form-container.tsx (create + edit)  (+ tests)       [S]
        views/      product-list-view.tsx · product-detail-view.tsx · product-form-view.tsx  [S]
    test-support/product.ts                        [S] ProductMother + fakeProductRepository
    test-support/render.tsx                        [T] renderScreen(ui, deps, path) for screen tests
```

Routes: `/products` · `/products/new` · `/products/:id` · `/products/:id/edit`. `{plural}` comes
from `naming::entity_ctx`; TS file names are the kebab-case of the snake ones (`order-item`).

Dependency Rule (`.dependency-cruiser.cjs`): `domain` imports nothing (no npm, no other layer);
`application` → `domain`; `infrastructure` → `domain`; `presentation` → all; only
`presentation/core/generated/dependencies.ts` imports `infrastructure`. Containers and views reach
adapters only through `useDependencies()`.

**Containers call the repository port directly.** A use case class that only delegates is
ceremony; when real logic arrives (a rule, an orchestration of two repositories) the user writes
`application/<entity>/<action>.ts` and the container switches to it. Puerto never generates one.

### Behaviour

- **`puerto new` includes `web/` by default.** `--no-web` skips it; the interactive flow asks
  (default yes). puerto.toml gets `[project] web = true` — absent means `false`, so a project
  without it generates nothing on the frontend.
- **`scaffold <Name>`** in a `web = true` project also writes the entity's web slice and
  regenerates both `core/generated/*` files. `--force` and the conflict guard cover the [S] web
  files exactly like the Rust ones.
- **`generate web <Name>`** — layer command (Phase 8 family): the web slice only + regeneration.
  Refused for an entity that is not in puerto.toml.
- **`generate bootstrap`** regenerates `bootstrap.rs` **and** the two web [G] files.
- Only CRUD-scaffolded entities get a web slice (all five CRUD use cases in puerto.toml). The
  Greeting demo gets none.
- **`generate use-case`** writes nothing in `web/` — a custom use case has no endpoint (ADR-0012).
- **`puerto validate`** drift check: in a `web = true` project, a CRUD entity without
  `web/src/presentation/features/<kebab>/` is reported, like a missing Rust module.
- **Fields → TS types and form controls** — one decision per field in `types.rs` (ADR-0011):

  | Rust type | TS type | Control |
  |---|---|---|
  | `String`, `Uuid` | `string` | `input type=text`, `required` |
  | `i64`, `f64` | `number` | `input type=number` (`step=any` for `f64`) |
  | `bool` | `boolean` | `checkbox` |
  | `DateTime<Utc>` | `string` (ISO 8601) | `input type=datetime-local` |
  | `Option<T>` | `T \| null` | as `T`, not `required`; empty → `null` |
  | `Vec<String>`, `Vec<i64>` | `T[]` | text input, comma-separated |
  | `HashMap<String, String>` | `Record<string, string>` | textarea, `key=value` per line |
  | enum VO | union of the variants (`"Draft" \| "Paid"`) | `<select>` |

  Non-enum VOs are their inner primitive on the wire, so on the frontend too. Enum VOs are a
  plain `string` in the spec; the adapter narrows it to the union.
- **No client-side copy of domain validation.** The server is the authority: a 422's
  `ErrorResponse.name` (`product.validation_error.name_empty`) becomes `ProductError.code` and the
  form shows it. HTML `required` / `type` only. Sharing the real rules is Phase 10.
- **Spec export.** When `web = true`, `bootstrap.rs` carries a `#[cfg(test)]` test that builds
  every `{Entity}Api` from the `Mock{Uc}UseCase`s (ADR-0012) and compares `spec()` with
  `docs/api/openapi.json`; `UPDATE_OPENAPI=1` rewrites it. No server, no database. The
  `OpenApiService` construction is shared between `build_app()` and the test so they cannot
  differ.
- **Sync step.** Project `make web/sync` = `UPDATE_OPENAPI=1 cargo test openapi` + `bun run
  openapi:types` (writes `schema.d.ts`). Puerto never runs `cargo` or `bun` itself; after a
  scaffold it prints `make web/sync` as the next step. Until then `bun run typecheck` fails —
  that failure is the drift check.
- Project Makefile: `web/install`, `web/dev`, `web/test`, `web/lint`, `web/sync`; `make test` and
  `make lint` include `web/` when the directory exists.
- Generated `AGENTS.md` gets a frontend section (layers, [G] files, `make web/sync` after
  touching a DTO, where use cases go); its test (ADR-0010) covers it.

### Out of scope (v1)

Auth · i18n · pagination (the list endpoint has none) · serving `web/dist` from `presentation` in
production · theme switching beyond daisyUI's defaults · custom use cases on the frontend ·
generated `application/` use cases.

### Pieces (one commit each)

1. ✅ **9.0 Shell** — `[project] web`, `--no-web`, prompt, static `web/` template (no entity),
   `bun test` spike, Makefile targets, AGENTS.md section, compile-matrix scenario `web_shell`.
   As built: `web/` lives in `crates/cli/template/web/` and `--no-web` removes it (with the root
   `AGENTS.md` section), the way `--no-demo` removes Greeting — so the snapshot is `no_web`
   (what the flag deletes); `web_shell` is the matrix test. `web/AGENTS.md` holds the frontend
   rules. `make web/sync` moves to 9.1 (nothing to sync before the spec export). The two
   `core/generated/*` files are template copies until 9.2 generates them.
2. ✅ **9.1 Spec export** — the `bootstrap.rs` spec test + `docs/api/openapi.json`.
   As built: a **missing** `openapi.json` is written and the test passes — a fresh project's
   `make test` is green with nothing to sync; an existing one that differs fails. The compile
   matrix runs the test twice per web scenario (writes, then compares: the export is
   deterministic) and `web_shell` runs `openapi:types` before typecheck. `make web/sync` and the
   scaffold's "Next: make web/sync" hint land here. The `domain-use-case` IDE snippet now ships
   the `mocks` module too — the spec test needs a mock per use case. `client.ts` waits for 9.2,
   its first importer.
3. ✅ **9.2 CRUD slice** — all-primitive entity: domain, infrastructure, presentation, test-support,
   generated `dependencies.ts` / `routes.tsx`, `generate web`, conflict guard, validate drift.
   As built: "all-primitive" = `String`, `Uuid`, `i64`, `f64`, `bool`, and non-enum VOs over them
   (`types::web_supported`). A CRUD entity with any other field type gets **no slice** and no
   route — scaffold warns, `generate web` refuses, validate does not report it — so every
   intermediate project still typechecks. `client.ts` is written with the first slice, never
   overwritten. Tests render screens with `test-support/render.tsx` (`renderScreen(ui, partial
   deps, path)`, wouter memory location); two dependency-cruiser rules cover `test-support/`.
   `make web/sync` also runs `bun run format` (Biome, linter off). Decisions: ADR-0013 § Decided
   in 9.2.
4. ✅ **9.3 Every type** — Option / Vec / HashMap / DateTime / enum VO types and controls.
   As built: the 9.2 gate is gone — every CRUD entity gets a slice (`has_web_slice` = the five
   CRUD use cases). Each field's shape (`types::WebShape`) decides its expressions in
   `web_field_ctx`; conversions are helpers of a new template file
   `presentation/shared/forms.ts` (+ tests): `textOf` / `textOrNull` / `numberOrNull` /
   `booleanOrNull`, `splitList` / `splitNumbers`, `formatEntries` / `parseEntries`,
   `toDateTimeLocal` / `fromDateTimeLocal` / `dateTimeOrNull`, `formatDateTime`. Each file imports
   only the helpers its fields call. An `Option` is an optional property on the wire (poem omits
   `required`), `null` in the model: the adapter maps `dto.x ?? null` in, `draft.x ?? undefined`
   out. `Option<bool>` is a `<select>` (—/Yes/No), not a checkbox, so it can be `null`. An enum VO
   is a union with `statusValues` + `parseStatus()` in `model.ts`, rejecting with the backend's
   `{entity}.invalid_{vo}`; the adapter and the form narrow through it. Mothers hold present,
   non-empty values (`Option` set, `["example"]`, `{ key: "value" }`, a whole-minute instant) so
   the form round-trip tests convert every field. Snapshot + matrix scenario `web_every_type`
   (entity `Case`, every type — and a TypeScript keyword as its name).

### Test scenarios

- Structural: `puerto new` writes `web/` and `web = true`; `--no-web` writes neither; scaffold in a
  `web` project writes every [S] file of the layout and no `application/<entity>/`; in a non-`web`
  project writes nothing under `web/`; `dependencies.ts` / `routes.tsx` list every CRUD entity
  and skip Greeting; `generate use-case` leaves `web/` untouched; `generate web` refuses an
  undeclared entity; re-scaffold without `--force` refuses when web files exist; validate reports
  a missing feature dir; `OrderItem` → `order-item` files, `/order_items` routes,
  `orderItemRepository`.
- Snapshots: `no_web` (9.0 — the shell is template, so its snapshot is what `--no-web` deletes),
  `web_crud`, `web_every_type` (`make test/snapshots`).
- Compile matrix (`make test/full`, needs `bun` on `PATH`): generate, `cargo test` (writes the
  missing spec, then a second run compares — 9.1), `bun install`, `bun run openapi:types`, `bun run typecheck`,
  `bun run lint`, `bun run lint:arch`, `bun test`.
- Generated web tests (shipped in every project): the HTTP repository maps 2xx → model and an
  `ErrorResponse` → `ProductError(code)`, with `fetch` stubbed; list container renders rows /
  empty state / error alert; detail container renders the entity / 404 code; form container
  creates a draft, edits an existing one and shows a 422 code; delete asks for confirmation.
- CI: `oven-sh/setup-bun` in the `test/full` job and in `release.yml` (9.0 ✅).

---

## Phase 10 — Shared domain rules via WASM (optional, later)

**Not started. Trigger:** custom validation rules on value objects (`min` / `max` modifiers —
`value_objects.md` § Future scope) or hand-written invariants that the frontend would otherwise
duplicate. Until then the generated rules are trivial (non-empty string, enum variant) and
puerto.toml already keeps both sides consistent at generation time.

Idea: compile `business::domain` to WASM (`wasm-bindgen` + `tsify`) and expose the VO
constructors and enums to `web/src/domain/`, so a form validates with the **same** Rust code the
API runs. Repository traits, `async-trait` and `mockall` stay behind a feature the WASM build
leaves off.

To decide when it starts (an ADR): opt-in flag vs default; bundle cost (`chrono`, `uuid`);
wasm-pack in the toolchain and the compile matrix; how a VO error reaches the form (the same
`ErrorResponse.name` code the server would send).

---

## Phase 11 — Typed ids, one repository error, paged lists (T-24, T-28, T-27) ✅ DONE

**Status:** ✅ DONE. Backlog items from `puerto_vs_document_api.md` § 2.4, 2.6, 2.7. Each one
changes generated output (`feat!`); no project generated before this phase is in use, so there is
no migration guide beyond the changelog. One commit per piece, in this order — 11.1 changes the
repository's error type and 11.2 rewrites `find_all`, so `find_all` is written once, over
`RepositoryError`.

### 11.0 — Newtype ids (T-24) ✅

Every CRUD entity's id is its own type, so `find_by_id(company_id)` no longer compiles where a
`ProductId` is expected.

- `model.rs` declares, above the entity:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct ProductId(Uuid);
  impl ProductId {
      pub fn generate() -> Self;          // Uuid::new_v4
      pub fn from_uuid(value: Uuid) -> Self;
      pub fn value(&self) -> Uuid;
  }
  impl std::fmt::Display for ProductId   // `%id` in the tracing calls keeps working
  ```
  Generated per entity, not a `macro_rules!`: the type is readable where it is defined, and the
  generator already is the macro. `Product::new` uses `ProductId::generate()`.
- `Product.id`, `Get/Update/DeleteProductParams.id`, `ProductRepositoryTrait::find_by_id(id)` and
  its mock take `ProductId`. The custom use case template is unchanged (no id).
- The edges convert: routes wrap `Path<Uuid>` with `ProductId::from_uuid`, `ProductDto.id` is
  `entity.id.value()` (the wire stays a `string(uuid)` — `web/` is untouched); `ProductDb.id` stays
  `Uuid` (`from_uuid` in `TryFrom`, `.value()` in `From`) and the Pg repository binds `id.value()`.
- Tests that need a missing id use `ProductId::generate()`, not `Uuid::new_v4()`. A `Uuid` import
  appears only where a `Uuid` *field* needs it.
- The non-CRUD path (`scaffold::run(…, crud = false, …)`, tests only) gets the same id.

Test scenarios: model declares `ProductId` and `pub id: ProductId`; get/update/delete params and
`find_by_id` take `ProductId`; routes convert with `ProductId::from_uuid`; DTO uses `.value()`;
Pg row conversion uses `from_uuid` / `.value()`; no generated file of a fieldless CRUD entity
imports `uuid::Uuid` without using it (compile matrix, `-D warnings`). Snapshots re-recorded.

### 11.1 — One repository error (T-28) ✅

Repositories speak one error, whatever the entity and the adapter; the entity's error converts
it. The `23505 → Duplicate` mapping moves out of each repository into one function.

- `business/src/domain/shared.rs` + `business/src/domain/shared/repository_error.rs` ship in the
  template (`pub mod shared;` in the `domain` block of `lib.rs`), with:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
  pub enum RepositoryError {
      #[error("repository.duplicate")]    Duplicate,    // a unique constraint rejected the write
      #[error("repository.invalid_data")] InvalidData,  // a stored row fails the domain's rules
      #[error("repository.unavailable")]  Unavailable,  // the store failed or is unreachable
  }
  ```
  Shared value objects (`generate value-object`) now land next to it: `shared/value_objects.rs`
  and `shared/errors.rs` are declared by appending to `shared.rs` — no more `shared/mod.rs` (the
  rule is no `mod.rs`); `shared.rs` is created if missing.
- Every repository trait (CRUD, non-CRUD, Greeting) returns `Result<_, RepositoryError>`; the
  mocks follow.
- `errors.rs` gains `impl From<RepositoryError> for ProductError`: `Duplicate → Duplicate`,
  `InvalidData | Unavailable → RepositoryError`, with one test per arm. The entity's error codes
  — the API contract — do not change. Use cases keep `?` (the `From` converts).
- `infrastructure/src/db.rs` gains `pub fn map_sqlx_error(err: &sqlx::Error) -> RepositoryError`
  (`23505` → `Duplicate`, anything else → `Unavailable`), unit-tested on non-database errors. Pg
  repositories log with context and convert with it; a row whose `TryFrom` fails becomes
  `InvalidData` (logged with the domain error).
- Pg integration test, when the entity has a `unique` field: saving a second entity with the same
  value returns `RepositoryError::Duplicate`.

Test scenarios: template ships `shared.rs` + `repository_error.rs` and `lib.rs` declares
`shared`; `generate value-object` + scaffold writes `shared/value_objects.rs` and `errors.rs`,
appends both to `shared.rs`, never writes `shared/mod.rs`, and is idempotent; repository traits
return `RepositoryError`; `errors.rs` has the `From` impl; `db.rs` has `map_sqlx_error` and the
Pg repositories call it, with no inline `23505`; the `--no-demo` project still declares `shared`.

### 11.2 — Paged lists (T-27) ✅

`list_*` reads one page — never the whole table.

- `business/src/domain/shared/pagination.rs` (template, declared in `shared.rs`):
  `DEFAULT_PAGE_LIMIT = 20`, `MAX_PAGE_LIMIT = 100`; `PageRequest` with private `limit: u32` /
  `offset: u32`, `PageRequest::new(limit: Option<u32>, offset: Option<u32>)` clamps the limit to
  `1..=MAX_PAGE_LIMIT` (absent → default, absent offset → 0) — no route can bypass the cap;
  `Default`; `Page<T> { items, total: u64, limit: u32, offset: u32 }` with `Page::new(items,
  total, request)` and `map`. Unit tests on the clamping.
- `find_all(&self, page: PageRequest) -> Result<Page<Product>, RepositoryError>`. Pg: `ORDER BY
  created_at DESC, id LIMIT $1 OFFSET $2` plus a `COUNT(*)` of the non-deleted rows. InMemory:
  an empty page. `ListProductsParams { page: PageRequest }`, the use case returns
  `Page<Product>`.
- `GET /products?limit=&offset=` (`Query<Option<u32>>` each) answers `ProductPageDto { items,
  total, limit, offset }` — `limit` is the one actually used, after the cap.
- `web/`: `src/domain/shared/page.ts` (template: `PageRequest`, `Page<T>`, `PAGE_SIZE = 20`);
  the port's `list(page)` returns `Page<Product>`; the HTTP adapter sends `limit` / `offset` as
  the query; the fake repository slices; the list container keeps the offset and renders a new
  static `shared/ui/pagination.tsx` ("1–20 of 45", Previous / Next, nothing when everything fits
  on one page). Empty state when `total` is 0.

Test scenarios: domain — the limit is clamped to the max, a zero limit becomes 1, absent values
take the defaults; application — the use case passes the page to the repository and returns its
`Page`; routes — `?limit=5&offset=10` reaches the use case as that page, the body carries
`items` and `total`; Pg — a page of a larger table has `limit` items and the full `total`; web —
the HTTP adapter sends the query and maps the items, the list container shows the next page when
Next is pressed, `Pagination` disables Previous on the first page and Next on the last.

---

## Phase 12 — Runtime crates (ADR-0017)

**Status:** done — 12.0, 12.1 and 12.2. Decision: [ADR-0017](docs/adr/0017-runtime-crates-next-to-the-generator.md).
Every piece is a **move**: code that is identical in every generated project leaves the template
and becomes a crate in Puerto's workspace. No new behaviour, no error code or HTTP status changes
(ADR-0007, 0008, 0015, 0016 hold) — the generated projects' tests prove it, unchanged in intent.
Each piece changes generated output (`feat!`); no project generated before it is in use, so the
changelog is the migration guide. One commit per piece, in this order (12.1 and 12.2 depend on
12.0's workspace and `[patch]` plumbing).

### 12.0 — Workspace, release plumbing and `puerto-core` ✅

As built, two refinements of the spec below: the requirement is rendered **once**, in the root
`Cargo.toml`'s `[workspace.dependencies]` (`puerto-core = "0.13"`), and `business`,
`infrastructure` and `presentation` each inherit it with `{ workspace = true }` — the Pg/InMemory
repositories, routes and DTOs import `puerto_core` too, and 12.1/12.2 add their crates in the same
place. The version is substituted when the embedded template is extracted
(`runtime::render_manifest`), so `new` and the test harness get the same manifest. A shared value
object's first scaffold creates `shared.rs` and `pub mod shared;` (what `generate value-object`
alone never did: it only writes puerto.toml).

- Puerto's workspace gains `crates/core` (`puerto-core`). Same version as `crates/cli`; the
  workspace sets `version.workspace = true` for both so they cannot drift. `publish = true`,
  `description`, `license`, `repository`, `rust-version` as `puerto` has.
- `puerto-core` dependencies: `thiserror` only (`async-trait` arrives with the first port). It
  holds, moved verbatim with their tests:
  - `pagination.rs` — `PageRequest`, `Page<T>`, `DEFAULT_PAGE_LIMIT`, `MAX_PAGE_LIMIT`;
  - `repository_error.rs` — `RepositoryError` (`repository.*` codes unchanged).
  Re-exported at the root: `puerto_core::{Page, PageRequest, RepositoryError}`. `RepositoryError`
  becomes `#[non_exhaustive]`; generated `From<RepositoryError>` impls gain no `_` arm — they live
  outside the crate, so the compiler would demand one: the arm maps to `RepositoryError` (500) and
  the per-arm tests stay one per *named* variant.
- Template: `business/src/domain/shared/{pagination,repository_error}.rs` are deleted;
  `shared.rs` no longer declares them (it is deleted when empty, and `pub mod shared;` with it —
  `generate value-object` recreates both, as it already handles a missing `shared.rs`).
  `business/Cargo.toml` gains `puerto-core = "<cli minor>"`, rendered from the CLI's own
  `CARGO_PKG_VERSION` (`"0.14"` for `0.14.x`), never hard-coded in the template.
- Generators import `puerto_core::{…}` instead of `crate::domain::shared::…` (`errors.tera`,
  repository traits and mocks, `list_*` use case, Pg/InMemory repositories, routes, DTOs).
- Compile matrix (`make test/full`, `make test/db`): every generated project gets
  ```toml
  [patch.crates-io]
  puerto-core = { path = "<puerto>/crates/core" }
  ```
  appended to its root `Cargo.toml` by the test harness (not by `puerto new`), so templates and
  the unreleased runtime are tested together.
- `release.yml`: publish runtime crates first, in dependency order, each skipped when its version
  is already on crates.io (same check as `puerto`), then `puerto`. The tag check covers every
  crate's version. `release.md` lists the order and the hand-publish fallback.
- `puerto validate` warns (does not fail) when a runtime crate in the project's `Cargo.toml`
  is on a different minor than the CLI: `warning: puerto-core 0.13 but puerto 0.14 — see the
  CHANGELOG migration guide`.
- Docs: `architecture.md` (domain may depend on `puerto-core`), `AGENTS.md` of Puerto and the
  generated root `AGENTS.md` (which runtime crate owns what — ADR-0010's test checks it names
  them), `puerto-toml.md` where it names the moved paths.

Test scenarios: `puerto new` writes `puerto-core = "<minor>"` in `business/Cargo.toml` and no
`domain/shared/{pagination,repository_error}.rs`; `--no-demo` project still compiles; scaffold
imports `puerto_core::RepositoryError` / `PageRequest` / `Page` and never `domain::shared::
repository_error`; `generate value-object` on a project without `shared.rs` creates it and
declares `pub mod shared;`; the rendered version follows `CARGO_PKG_VERSION` (unit test on the
render function); `validate` warns on a minor mismatch and is silent on a patch difference;
`puerto-core` unit tests (clamping, codes) pass in Puerto's workspace; `make test/mutation`
covers `crates/core`. Snapshots re-recorded — the `.snap` diff is deletions plus import changes.

### 12.1 — `puerto-web` ✅

As built, where it departs from the spec below:

- **`IntoErrorResponse` stays in the project** (`presentation/src/api/error.rs`, the trait alone);
  only `ErrorResponse` moved. Each `error_mapper.rs` implements the trait for an error of
  `business`: with the trait in `puerto-web` both would be foreign to `presentation`, which the
  orphan rule (E0117) forbids. `api.rs` keeps `pub mod error;`.
- **Health is two functions, not a `HealthApi`**: `puerto_web::health::{probe, always_ready}` build
  the `ReadinessProbe`, `health::mount(route, probe)` adds `/health` + `/health/ready` (still plain
  poem routes outside the OpenAPI spec, as before). `puerto_web::routes(api, &config, probe)` puts
  them next to `/api` and the Swagger UI.
- **`App::serve(build)` adds tracing and CORS around what `build` returns**, so `bootstrap.rs`
  names neither: `build_app(config: Config) -> impl Endpoint` ends with
  `crate::app::configure(puerto_web::routes(openapi_service(…), &config, ready))`.
- **`app::configure(routes: Route) -> impl Endpoint`**, not `-> Route`: a `Route` cannot carry
  middleware (`.with` returns another type). An adapter swap is not possible there — the
  repositories are built before it — so the hook is documented for middleware and routes only.
- `puerto-web` does not depend on `puerto-core` yet: nothing in it uses it.
- `Config` is built by `from_env`, `from_vars(lookup)` (tests) or `Config::default().with_*`.
  `PUERTO_BANNER` accepts `true`/`false`/`1`/`0`; an empty value counts as unset. The banner says
  `PUERTO` and the runtime's version. `puerto_web::Error`'s `Debug` is its message, so `main`
  returning it prints `SERVICE_PORT="abc" is not valid: …`.
- `presentation/Cargo.toml` drops `tracing-subscriber`, `dotenvy` and tokio's `signal` feature.

- `crates/web` (`puerto-web`), depends on `puerto-core`, `poem`, `poem-openapi`, `tokio`,
  `tracing`, `tracing-subscriber`, `dotenvy`. Moved, with tests:
  - `ErrorResponse`, `ErrorResponse::from_code`, `IntoErrorResponse` (from `api/error.rs`);
  - health: `puerto_web::health::HealthApi` taking a readiness probe
    (`Arc<dyn Fn() -> BoxFuture<'static, bool> + Send + Sync>`; `--db` passes
    `puerto_sqlx::ping` in 12.2, otherwise always ready);
  - `Config::from_env()` — `SERVICE_PORT` (default 8080), `CORS_ALLOWED_ORIGINS`,
    `PUERTO_BANNER` (replaces `HARBOR_BANNER`); reads `.env` via `dotenvy`; an invalid value is an
    error at startup, not a silent default (today `SERVICE_PORT=abc` silently falls back to 8080);
    `#[non_exhaustive]`, built only through `from_env` / a builder;
  - `App::new(config).serve(build_app)` — tracing subscriber, banner, listener, graceful
    shutdown with `SHUTDOWN_DEADLINE` (ADR-0008), CORS from the config;
  - `openapi_service(apis)` setup now in `bootstrap.rs` (title, version, Swagger UI path).
- Template: `main.rs` shrinks to `Config::from_env()` + `App::new(config).serve(…)`;
  `health.rs` and `api/error.rs` are deleted; `api.rs` stops declaring `error`. Generated error
  mappers and routes import `puerto_web::{ErrorResponse, IntoErrorResponse}`.
- New hand-written hook, created once by `puerto new`, never touched by any generator:
  `presentation/src/app.rs` with `pub fn configure(route: Route) -> Route { route }` — called by
  `bootstrap.rs` last, the one place for extra middleware or an adapter swap. `conflict` guards
  it like any user file.
- `bootstrap.rs` calls `puerto_web` for the service and CORS; its shape is otherwise unchanged
  (still generated, still the composition root — ADR-0017 § 3).
- `[patch.crates-io]` in the matrix gains `puerto-web`.

Test scenarios: generated `main.rs` has no server/shutdown code and names `puerto_web::App`;
no `health.rs` / `api/error.rs`; `app.rs` exists after `new`, survives `scaffold --force` and
`generate bootstrap` byte for byte; `bootstrap.rs` calls `app::configure`; route tests (ADR-0012)
still assert the same statuses and `name` codes; in `crates/web`: `/health` 200, `/health/ready`
503 when the probe is false, `Config::from_env` rejects a non-numeric port, CORS answers a
preflight from an allowed origin and not from another. Snapshots re-recorded.

### 12.2 — `puerto-sqlx` ✅

As built, where it departs from the spec below:

- `bootstrap.tera` changes too: `run_migrations` takes the migrator, so the generated call is
  `infrastructure::db::run_migrations(&pool, &infrastructure::db::MIGRATOR)`. The repository
  templates are untouched (`crate::db::map_sqlx_error` resolves through the re-export).
- `puerto-sqlx` is a `--db` dependency only: `new --db` adds `puerto-sqlx = "<cli minor>"` to the
  root `[workspace.dependencies]` and `{ workspace = true }` to `infrastructure` (which keeps
  `sqlx` for its queries and `migrate!`). A project without a database never names it.
- The matrix's `[patch.crates-io]` lists only the runtime crates the project's manifest names — an
  unused patch is a cargo warning in every build.
- `DatabaseError` becomes `#[non_exhaustive]`; `map_sqlx_error` drops the let-chain (the
  published crate's `rust-version` is 1.85). Puerto's root manifest gains
  `[workspace.dependencies] puerto-core = { version, path }`, bumped with the workspace version.
- Tests in `crates/sqlx`: the non-database mapping, the error codes, `ping` false on an unreachable
  server, an invalid URL refused; under `make test/db` a real `23505` → `Duplicate` and `ping` true.
  `/health/ready` 503 itself is `puerto-web`'s test (a false probe).

- `crates/sqlx` (`puerto-sqlx`), depends on `puerto-core`, `sqlx` (postgres, runtime-tokio).
  Moved from `templates/infrastructure/db.tera`, with tests: `create_postgres_pool`, `ping`,
  `map_sqlx_error`, `DatabaseError`.
- `run_migrations` cannot move as is: `sqlx::migrate!("./migrations")` embeds the *project's*
  directory at compile time. It becomes `puerto_sqlx::run_migrations(pool, &MIGRATOR)`, and the
  project keeps one line: `pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");` in
  `infrastructure/src/db.rs`, which otherwise re-exports `puerto_sqlx::*` — so generated
  repositories keep `crate::db::map_sqlx_error` and no `.tera` beyond `db.tera` changes.
- `--db` template `infrastructure/Cargo.toml` gains `puerto-sqlx`; `/health/ready` gets
  `puerto_sqlx::ping` through 12.1's probe.
- `[patch.crates-io]` in `make test/db` gains `puerto-sqlx`.

Test scenarios: `--db` project's `db.rs` is the `MIGRATOR` line plus the re-export; Pg
repositories compile unchanged; `make test/db` — a duplicate `unique` value still returns
`RepositoryError::Duplicate`, `/health/ready` 503 with Postgres stopped; in `crates/sqlx`:
`map_sqlx_error` maps a non-database error to `Unavailable` (Postgres-backed `23505` test runs
under `make test/db`). Snapshots re-recorded.

### After Phase 12

New infrastructure lands in the runtime, split per ADR-0017 § 1: ports in `puerto-core`,
adapters in their own crate, domain-facing parts (the `User` entity, login use cases) generated.
Candidates, in roadmap order: `UnitOfWork` (T-23), auth, jobs (T-29), mailer, storage.
T-23 (with T-29's second binary) is Phase 13; the mailer, Phase 14, is the first consumer built on it.
`puerto eject <component>` once a runtime crate has something worth ejecting.

---

## Phase 13 — Domain events: unit of work, outbox, worker (T-23, T-29)

**Status:** done — 13.0 (`7b83e3f`) · 13.1 (`9947162`) · 13.2 (`789a0ed`) · 13.3
(`02578b8`, with `960e01f`) · 13.4 (`31f6200`, `ea1c46f`, `2ad0263`) · 13.5 (`5441eeb`,
`7db138a`, `a798b26`, with `372596d`). Decision:
[ADR-0018](docs/adr/0018-domain-events-outbox-pluggable-transport.md).
Every write in an `events` project saves the aggregate and its event in one transaction (outbox).
A second binary, `worker/`, relays the outbox to a transport and runs the consumers. The transport
is an adapter: `db` (Postgres, no broker) and `kafka` in this phase. A project without `events`
is generated exactly as today: no snapshot of the existing matrix scenarios changes, except where
a piece says otherwise. One commit per piece, in this order.

Glossary used below, for entity `Product`:

| Name | Where | Generated |
|---|---|---|
| `ProductSnapshot`, `ProductEvent` | `business/src/domain/product/events.rs` | always regenerated from puerto.toml |
| `ProductUnitOfWork`, `ProductUnitOfWorkFactory` | `business/src/domain/product/unit_of_work.rs` | once, by scaffold |
| `PgProductUnitOfWorkFactory` | `infrastructure/src/product/unit_of_work.rs` | once, by scaffold |
| `EventEnvelope`, `DomainEvent`, `EventHandler`, `HandlerOutcome` | `puerto-core` | runtime |
| `<consumer>` handler | `worker/src/consumers/<consumer>.rs` | once, by `generate consumer` |
| consumer registry | `worker/src/generated/registry.rs` | always regenerated from puerto.toml |

### puerto.toml additions

```toml
[project]
name = "shop"
db = true
events = "db"                       # "db" | "kafka"; absent = no events. Requires db = true

[[entity]]
name = "Product"
use_cases = ["create_product", "get_product", "list_products", "update_product", "delete_product"]
events = ["product_published"]      # optional: custom events beyond created/updated/deleted

[[consumer]]
name = "notify_new_product"         # snake_case, unique; file worker/src/consumers/notify_new_product.rs
events = ["product_created", "product_published"]   # event types it subscribes to
```

Additive (a minor bump). `puerto validate` adds these checks: `events` is `db` or `kafka`; `events`
without `db = true` is an error; custom event names are snake_case, unique per entity and do not
collide with the CRUD ones; a consumer's `events` are all declared by some entity (CRUD or
custom); consumer names are snake_case and unique, and every declared consumer has its file on
disk (the same drift check as entities).

### 13.0 — ADR-0018 + this spec ✅

### 13.1 — `puerto-core`: events, ports and delivery policy ✅

Settled ports (`puerto_core::events::transport`): `Outbox` (`try_lead`, `fetch_unpublished`,
`mark_published` — the relay's input, so `puerto-events` names no Postgres), `EventPublisher`
(`publish(&EventEnvelope)`, one event at a time so the relay stops at the first failure),
`EventSubscription` (`consumer()`, `next_batch(max) -> Vec<Claimed>`, `settle(&Claimed,
&Delivery)`; `Claimed { envelope, attempt }`), `DeadLetterSink::record(&DeadLetterRecord)`
(`for_event` / `unreadable`, payload trimmed to 4000 chars) and `TransportError::{Unavailable,
InvalidData}`. Envelopes read back from a store are built with `EventEnvelope::from_parts`.

No generated output changes. `crates/core` gains an `events` module and the dependencies
`async-trait`, `serde`, `serde_json`, `uuid`, `chrono` (ADR-0018 extends ADR-0017's table).

- `trait DomainEvent`: `event_type() -> &'static str`, `aggregate_type() -> &'static str`,
  `aggregate_id() -> Uuid`, `schema_version() -> u16`, `payload() -> serde_json::Value`.
- `EventEnvelope`, `#[non_exhaustive]`: `id: Uuid`, `event_type`, `aggregate_type`,
  `aggregate_id`, `schema_version`, `occurred_at: DateTime<Utc>`, `payload: Value`,
  `headers: BTreeMap<String, String>`. `EventEnvelope::new(&impl DomainEvent)` assigns the id
  (v7, which sorts by time) and `occurred_at`. `with_header(k, v)`. `decode::<T: DeserializeOwned>()`
  returns `Result<T, DecodeError>`.
- `trait EventHandler: Send + Sync { async fn handle(&self, event: &EventEnvelope) -> HandlerOutcome }`;
  `HandlerOutcome::{Ack, Retry, DeadLetter { reason: &'static str, detail: String }}`,
  `#[non_exhaustive]`. The reason constants are `REASON_PAYLOAD_INVALID`,
  `REASON_MAX_DELIVERIES_EXCEEDED` and `REASON_PERMANENT_ERROR`.
- `delivery::{decide, backoff_for, Delivery, DEFAULT_MAX_DELIVERIES}`, the same policy as
  document-api: `Ack → Ack`, `DeadLetter → DeadLetter`, `Retry` at or above the cap →
  `DeadLetter(max_deliveries_exceeded)`, otherwise `Retry { delay }`. Backoff 1/5/15/60/300 s,
  the last value repeating.
- The transport ports the worker drives (`EventPublisher` for the relay's output,
  `EventSubscription` for a consumer's input) and `DeadLetterRecord` / `DeadLetterSink`. Their
  exact signatures are settled here, with a fake of each in the tests.

Test scenarios: an envelope built from a test event exposes the event's metadata; its `id` is
unique per call; `decode` round-trips the payload and rejects a payload of another shape with
`DecodeError`; `decide` covers every arm plus the `>=` boundary (attempt 4 of 5 retries, attempt 5
dead-letters); backoff grows and is capped; reason codes are stable strings. `make test/mutation`
covers `events` with zero survivors.

### 13.2 — `puerto-sqlx`: Pg unit of work, outbox, delivery store ✅

Done as specified, plus what the code settled: `dead_letters` gets
`UNIQUE (event_id, consumer)` (`dead_letters_event_consumer`) so the sink is
`ON CONFLICT DO NOTHING` — a message with no `event_id` never conflicts. A claim **counts as a
delivery**: `next_batch` increments `attempts` and hides the row for a lease
(`DEFAULT_CLAIM_LEASE`, 60 s, `with_lease`), so a worker that dies mid-handle neither loses the
event nor gets unlimited tries; `settle(Retry)` only moves `next_attempt_at`. Claims come in
outbox `seq` order. `PgDeliveries::route(consumer, &[event_type])` is what the worker's registry
will call per `[[consumer]]`; an event nobody subscribed to is still marked published.
`PgOutbox::try_lead` keeps the advisory-lock connection detached from the pool;
`with_lock_key` exists for two projects sharing one database. Tests:
`crates/sqlx/tests/events_postgres.rs` (`make test/db`).

No generated output changes. Runtime only, tested under `make test/db`.

- Outbox schema, one migration. The CLI embeds it as a template, and a structural test checks it
  byte for byte against `crates/sqlx/migrations/outbox.sql`, the copy the crate's tests run:
  ```sql
  CREATE TABLE outbox (
      id             UUID        PRIMARY KEY,
      seq            BIGSERIAL   NOT NULL,          -- relay order
      aggregate_type TEXT        NOT NULL,
      aggregate_id   UUID        NOT NULL,
      event_type     TEXT        NOT NULL,
      schema_version INT         NOT NULL,
      payload        JSONB       NOT NULL,
      headers        JSONB       NOT NULL DEFAULT '{}',
      occurred_at    TIMESTAMPTZ NOT NULL,
      published_at   TIMESTAMPTZ
  );
  CREATE INDEX outbox_unpublished ON outbox (seq) WHERE published_at IS NULL;

  CREATE TABLE event_deliveries (                  -- db transport only
      event_id        UUID        NOT NULL REFERENCES outbox (id),
      consumer        TEXT        NOT NULL,
      status          TEXT        NOT NULL DEFAULT 'pending',   -- pending | done | dead
      attempts        INT         NOT NULL DEFAULT 0,
      next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
      last_error      TEXT,
      PRIMARY KEY (event_id, consumer)
  );
  CREATE INDEX event_deliveries_due ON event_deliveries (consumer, next_attempt_at) WHERE status = 'pending';

  CREATE TABLE dead_letters (                      -- every transport
      id           UUID        PRIMARY KEY,
      event_id     UUID,
      consumer     TEXT        NOT NULL,
      event_type   TEXT,
      attempts     INT         NOT NULL,
      reason       TEXT        NOT NULL,
      detail       TEXT        NOT NULL,
      payload      TEXT        NOT NULL,           -- trimmed to 4000 chars
      recorded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
  );
  ```
- `puerto_sqlx::uow::PgTransaction`: `begin(&PgPool)`, `tx()` for the generated `save`,
  `record(&impl DomainEvent)` (builds the envelope, inserts the outbox row) and `commit()`.
  Dropping it rolls back. Generated UoWs wrap it, so they only write their entity's upsert.
- `puerto_sqlx::events::PgOutbox`: `fetch_unpublished(limit)` (by `seq`) and `mark_published(ids)`.
  The relay's advisory lock is `pg_try_advisory_lock(<constant>)`.
- `puerto_sqlx::events::PgDeliveries`, the `db` transport: the publisher (fan out to the
  registered consumers and mark the row published, in one transaction) and the subscription
  (claim due rows with `FOR UPDATE SKIP LOCKED`, settle them as done, retry or dead).
- `puerto_sqlx::events::PgDeadLetters`, the `DeadLetterSink`. It is idempotent on
  `(event_id, consumer)`.

Test scenarios (`make test/db`): a committed `PgTransaction` leaves the entity and its outbox row
written; a dropped one leaves neither; unpublished rows come back in `seq` order and marked ones
do not; fan-out writes one delivery per consumer and marks the row published atomically; two
concurrent claims never return the same delivery; `Retry` moves `next_attempt_at` forward and
increments `attempts`; `dead` writes a dead letter, and recording the same one twice writes one row.

### 13.3 — Generated domain events and UoW (`puerto new --events db`) ✅

Done as specified. What the code settled:
- An entity uses a unit of work when the project has `events`, the entity is `db` and it has the
  five CRUD use cases (`bootstrap::uses_unit_of_work`). `create` gets `uow_factory` only, `get` /
  `list` `repository` only, everything else both.
- `{Entity}Snapshot` is the DTO's shape (`id` + fields, value objects unwrapped,
  `serde(rename_all = "camelCase")`); `events.rs` also exports `pub const {ENTITY}_CREATED…`
  for consumers to match on. A custom event's variant drops the entity prefix
  (`product_published` → `Published`), and `generate event` / `validate` refuse a second name for
  the same variant.
- `RecordingUnitOfWorkFactory::failing_at(FailAt::{Begin, Save, Commit}, RepositoryError)`.
- `infrastructure` needs no `serde_json`: its tests read the outbox with sqlx. The "POST
  `/products` → one row + one `product_created`; a duplicate `unique` → neither" check is the Pg
  unit of work's `#[sqlx::test]`s, run by `make test/db` through the new
  `postgres_events_project_compiles_lints_and_passes_integration_tests` (scenario `events_db`).
- Found on the way: the web generator sorted import specifiers case-insensitively, Biome puts the
  same letter in upper case first (`type Product` before `parseStatus`) — fixed in its own commit.

Tier 1. New matrix scenario `events_db` (`--db --events db`, CRUD entity with VO fields plus a
custom event). Its snapshot is new; the existing scenarios' snapshots do not change.

- `puerto new --db --events db|kafka`. Interactively, once db is chosen, Puerto asks
  "Domain events? none / db / kafka". `--events` without `--db` is refused with a message naming
  `--db`. `[project] events` is written. The outbox migration is written to
  `infrastructure/migrations/<ts>_outbox.sql`. `business` gains `serde`/`serde_json`, and
  `infrastructure` gains `serde_json` for the snapshot round-trip test.
- `scaffold` in an `events` project, per CRUD entity:
  - `business/src/domain/<e>/events.rs` (regenerated on every scaffold and on
    `generate bootstrap`; header `// AUTO-GENERATED`): `{E}Snapshot` (DTO-shaped primitives,
    serde, `from_entity(&E)`), `{E}Event::{Created, Updated, Deleted, <Custom>…}(Snapshot)`, and
    `impl puerto_core::DomainEvent`. Tests: metadata per variant, and a snapshot round-trip that
    pins the JSON contract.
  - `business/src/domain/<e>/unit_of_work.rs`: the two ports (ADR-0018 § 2) and, behind the
    `test-utils` gate, `mocks::RecordingUnitOfWorkFactory`: it records saved entities, recorded
    events and whether `commit` ran, and can be scripted to fail at `begin`, `save` or `commit`.
  - The repository port loses `save` (and so do its mock, the Pg repository and the InMemory
    repository's trait impl). Reads are unchanged.
  - `create`: `uow_factory` only. `update`/`delete`: `repository` (to read) and `uow_factory`. Each
    one: `begin` → `save` → `record({E}Event::X(Snapshot::from_entity(&e)))` → `commit`, with the
    existing `tracing` lines (ADR-0006).
  - `infrastructure/src/<e>/unit_of_work.rs`: `Pg{E}UnitOfWorkFactory { pool }` over
    `puerto_sqlx::uow::PgTransaction`. `save` is an upsert keyed by `id` (the insert and update
    SQL the Pg repository has today, moved here).
  - `lib.rs` patches: `pub mod events; pub mod unit_of_work;` in the entity block. The
    infrastructure entity module gains `pub mod unit_of_work;`.
  - `bootstrap.rs` builds one `Pg{E}UnitOfWorkFactory` per entity and passes it to the write use
    cases.
- `generate use-case` in an `events` project: the impl gets both `repository` and `uow_factory`,
  `bootstrap.rs` wires both, and the stub's comment says writes go through the UoW.
- `generate event <Entity> <name>`: appends to `[[entity]] events`, regenerates `events.rs`, and
  refuses a duplicate, a CRUD name or a non-snake_case name. It does not touch any use case.
- Simple (non-CRUD) entities keep their repository unchanged. Events are a CRUD-write concern
  until a custom use case records one.
- Docs: `architecture.md` (UoW, events.rs, repository without `save`), `puerto-toml.md`
  (`events`, `[[entity]] events`), and the generated root `AGENTS.md`, which ADR-0010's test
  makes name `events.rs` as never hand-edited.

Test scenarios: `new --events db` without `--db` fails, and with it writes `events = "db"` and the
outbox migration; scaffold in an events project writes `events.rs` and `unit_of_work.rs`, a
repository trait with no `save`, and `create`/`update`/`delete` that name `uow_factory` and
`record(`; scaffold in a non-events project is byte-identical to before (the existing snapshots);
`events.rs` is regenerated after `generate event`; `generate event` refuses `product_created` and
duplicates; `validate` rejects `events` without `db`; generated use case tests:
`should_record_product_created_when_fields_are_valid`,
`should_not_commit_when_the_store_rejects_a_duplicate` and
`should_return_not_found_and_record_nothing_when_the_product_does_not_exist`, using the recording
factory. The compile matrix: `events_db` passes clippy and its tests. `make test/db`: POST
`/products` leaves one row in `products` and one `product_created` row in `outbox`, and a
duplicate `unique` leaves neither.

### 13.4 — `puerto-events` + the `db` transport + the `worker/` binary ✅

Tier 1. The `events_db` snapshot grows a `worker/` crate.

Done as specified. What the code settled — the runtime crate `crates/events` (`31f6200`):
- `Relay<O: Outbox, P: EventPublisher>::relay_once` and `Consumer<S>::consume_once` are one pass
  each, answering `Progress::{More, Idle}` (`More` = a full batch: run again at once). The worker
  drives them (`driver::drive`); a pass in flight at SIGTERM is finished, bounded by
  `puerto_web::app::SHUTDOWN_DEADLINE`.
- `Worker::new(config).run(build)`, where `build(WorkerConfig) -> Loops` and
  `Loops::new(&config).relay(outbox, publisher).consumer(subscription, handler, dead_letters)`
  applies the config. `puerto_events::Error` is `puerto_web::Error`.
- Readiness: every loop finished a pass **that did not fail** within its interval plus
  `STALL_GRACE` (30 s): a database that is down shows up as not ready. Before the first pass:
  not ready.
- A consumer claims `DEFAULT_CONSUMER_BATCH_SIZE` (10) per pass. One more variable:
  `RETRY_MAX_DELAY_MS` caps the policy's backoff (unset: the policy's), which is how
  `make test/db` reaches the dead letter in seconds. `RELAY_BATCH_SIZE` / `MAX_DELIVERIES` of 0
  are rejected.
- `puerto-web` exposes `app::{init_tracing, shutdown_signal}` and `config::load_dotenv`, so the
  worker shares them. `puerto-web` joins `[workspace.dependencies]`; release order is
  core → web → sqlx → events → puerto.

The `worker/` template and `generate consumer` (`ea1c46f`):
- Only `--events db` writes `worker/`; `--events kafka` gets it in 13.5, with its own registry.
- `registry.rs` routes and imports the events by their `{ENTITY}_{VERB}` constants, so an event
  removed from puerto.toml is a compile error in the worker. It runs the migrations too (as
  `bootstrap.rs` does), so the worker starts on its own.
- `generate consumer` also refuses a name whose type the worker already uses (`loops` → `Loops`,
  `product_event` → `ProductEvent`) and a handler file already on disk. `consumers.rs` keeps its
  `pub mod`s sorted, as rustfmt would.
- A `[[consumer]]` whose file is missing is drift: `validate` fails and `generate bootstrap`
  refuses, like an entity's.
- The consumer stub's tests build the envelope from the first subscribed event and the entity's
  Object Mother (`business` `test-utils`).
- End to end (`2ad0263`, `tests/worker_e2e.rs`, `make test/db`): the generated API and worker run as real
  processes; delivery, the dead letter after 4 retries + 1 (`RETRY_MAX_DELAY_MS=0`) and delivery
  after a worker restart are read from the worker's log.

Starting point (what 13.1–13.3 left to build on):
- Ports: `puerto_core::events::transport::{Outbox (try_lead / fetch_unpublished /
  mark_published), EventPublisher::publish, EventSubscription (consumer / next_batch /
  settle), Claimed { envelope, attempt }, DeadLetterSink, DeadLetterRecord::{for_event,
  unreadable}, TransportError}`; policy `puerto_core::events::delivery::{decide, backoff_for}`.
  The relay and consume loops in `crates/events` are generic over these and test against fakes
  like the ones in `crates/core/src/events/transport.rs`.
- `db` adapters: `puerto_sqlx::events::{PgOutbox, PgDeliveries (.route(consumer, &[types])),
  PgSubscription (.with_lease), PgDeadLetters}` — `PgDeliveries::publish` already marks the row
  published in its transaction; the relay's `mark_published` after it is a no-op.
- Generator: `[project] events` / `[[entity]] events` in `puerto_toml.rs`;
  `generators/events.rs` (`regenerate_events`, `run_generate_event`);
  `bootstrap::uses_unit_of_work`; `pub const {ENTITY}_{VERB}` event types in each `events.rs`
  for the consumer stubs to match on. `[[consumer]]` is not in the schema yet.
- Tests: scenario `EVENTS_DB` (`compile_matrix.rs`, fields `events` / `entity_events`),
  `tests/domain_events.rs`, and `postgres_events_project_compiles_lints_and_passes_integration_tests`
  under `make test/db`. `release.yml`, `RUNTIME_CRATES` (`runtime.rs`) and
  `patch_runtime_crates` must learn `puerto-events`.

- New runtime crate `crates/events` (`puerto-events`), depending on `puerto-core`, `puerto-web`
  (health only), `tokio` and `tracing`:
  - `Relay<P: EventPublisher>`: holds the advisory lock, fetches unpublished rows in batches,
    publishes in order, stops at the first failure (order preserved) and marks only what was
    confirmed. It drains back-to-back while rows remain and sleeps `poll_interval` otherwise.
  - `Consumer<S: EventSubscription>`: hands each delivery to its `EventHandler`, applies
    `decide`, and records a dead letter before retiring. A failed dead-letter write → retry. A
    payload that does not decode → `DeadLetter(payload_invalid)`. A handler panic is caught and
    treated as `Retry`.
  - `Worker::new(WorkerConfig::from_env()?)` with `.run(build)`: spawns the relay and the
    consumers, serves `/health` and `/health/ready` on `WORKER_PORT` (default 8081; ready while
    every loop has ticked within its interval) and drains on SIGTERM (ADR-0008).
    `WorkerConfig`: `WORKER_PORT`, `RELAY_BATCH_SIZE` (100), `RELAY_POLL_INTERVAL_MS` (500),
    `CONSUMER_POLL_INTERVAL_MS` (500), `MAX_DELIVERIES` (5).
- Template, for `events` projects only: a `worker/` member (`Cargo.toml`, `src/main.rs` written
  once, `src/generated.rs`, `src/generated/registry.rs` regenerated, and `src/consumers.rs`
  declaring the consumer modules), `make worker` (run it) and `make events/dead-letters` (list
  recent dead letters with `psql`), and `.env` gains `WORKER_PORT`.
- `puerto generate consumer <name> --on <event_type>[,<event_type>…]`: appends `[[consumer]]`,
  writes `worker/src/consumers/<name>.rs`, patches `consumers.rs` and regenerates `registry.rs`.
  It refuses an existing name or an undeclared event type. The stub is an `EventHandler`: it
  matches `event.event_type`, decodes the snapshot (a decode error dead-letters), logs, returns
  `Ack`, and carries a comment that it must be idempotent. Its tests: it acks a valid envelope,
  it acks the same envelope delivered twice, and it dead-letters a malformed payload.
- `generate bootstrap` also regenerates `registry.rs` in `events` projects.

Test scenarios: in `crates/events`, the relay against fake ports publishes in order and marks
everything, stops at the first failure and marks only the confirmed prefix, and does nothing on
an empty outbox; the consumer applies every `decide` arm, records the dead letter before
retiring, retries when the sink fails, and treats a panic as a retry; `WorkerConfig` rejects a
non-numeric port. Structural: `new --events db` writes `worker/` and adds it to the workspace;
`generate consumer` writes the stub, the toml entry and the registry line, and refuses an unknown
event. `make test/db` end to end: start the worker, POST `/products`, and the generated consumer
sees `product_created` within a bounded wait; a consumer that always retries ends up in
`dead_letters` after 5 attempts (with the backoff shortened through config); with the worker
stopped, the outbox keeps the row and the worker delivers it after a restart.

### 13.5 — `puerto-kafka` (`puerto new --events kafka`) ✅

Done as specified. What the code settled — the runtime crate `crates/kafka`:
- The retry question: `KafkaSubscription::settle(Retry)` does not commit; it keeps the record and
  hands it out again once the backoff has passed, as `attempt + 1`. So that a `Retry` holds back
  the records behind it instead of letting the handler see them first, the subscription hands out
  **one record per batch**: `EventSubscription` gained `max_batch()` (default `u32::MAX`, Kafka 1)
  and `Consumer` never asks for more — a batch of 1 is "full", so the loop does not sleep between
  records. While a retry waits nothing is read (the whole subscription stalls, not only the
  partition), hence `max.poll.interval.ms` = 600 s, above the 300 s backoff. The attempt count is
  in memory: a restart starts it over.
- The group id is `<prefix>.<consumer>`, like topics `<prefix>.<aggregate>`, so two projects on
  one cluster never share a group. Topics are created by `KafkaConfig::ensure_topics` with the
  broker's default partitions and replication (`-1`); the registry calls it for every aggregate
  that records events, before the relay starts.
- Wire format: the value is the JSON payload, the key the aggregate id, the envelope fields
  (`event_id`, `event_type`, `aggregate_type`, `aggregate_id`, `schema_version`, `occurred_at`)
  headers, then the envelope's own headers (one named like a field is left out). A record that
  does not read back as an envelope is dead-lettered by the subscription itself
  (`DeadLetterRecord::unreadable`, `payload_invalid`) and committed; if the sink fails it is kept
  and retried first on the next batch. A record of a type the consumer did not subscribe to is
  committed and skipped. Commits are synchronous, off the async threads (`spawn_blocking`).
- Building librdkafka (`cmake-build`) needs the libcurl headers as well as `cmake`: librdkafka
  2.12 guards `#include <curl/curl.h>` with `#ifdef WITH_OAUTHBEARER_OIDC`, which its
  `config.h` always defines (as 0). CI installs `libcurl4-openssl-dev`; the README says so.
- The producer: `acks=all`, idempotent, `message.timeout.ms` 30 s — a publish that times out is a
  relay pass that failed, so the worker shows not ready while the broker is down.

The generator:
- Redpanda is a service added to the project's `docker-compose.yml` (with its volume), not a
  separate `compose.yaml`: Compose prefers `compose.yaml` and would stop starting Postgres.
- The `kafka` outbox migration is the template with its `event_deliveries` block cut out
  (`project::without_event_deliveries`), so `templates/infrastructure/outbox.sql` stays the byte
  copy of `puerto-sqlx`'s.
- `make test/kafka` runs `-E 'test(/(^|::)kafka_/)'` against Redpanda and a Postgres of its own
  (:55433): the crate's broker tests, the `events_kafka` scenario (it needs Postgres to compile,
  not a broker) and the generated worker end to end (`worker_e2e.rs`). Puerto's CLI gains an
  `rdkafka` dev-dependency for the poisoned record.

Tier 1. New matrix scenario `events_kafka`. A new `make test/kafka` brings up a Redpanda
container, which is lighter than Kafka and speaks the same protocol.

Starting point (what 13.4 left to build on):
- `puerto-events` is transport-neutral: `Loops::new(&config).relay(outbox, publisher)
  .consumer(subscription, handler, dead_letters)` takes any `Outbox` / `EventPublisher` /
  `EventSubscription` / `DeadLetterSink`. Kafka needs `KafkaPublisher: EventPublisher` and
  `KafkaSubscription: EventSubscription`; the outbox (`PgOutbox`) and the dead letters
  (`PgDeadLetters`) stay Postgres. The in-process retry means `KafkaSubscription::settle(Retry)`
  must not commit, or the consume loop needs a kafka-specific path — decide it in the code.
- Generator: `generators/worker.rs` — `WORKER_TRANSPORT = "db"` and `has_worker` gate the worker
  to `--events db`; `apply_worker_to_new_project`, `generate_registry` (`templates/worker/
  registry.tera`) and `run_generate_consumer` (refuses non-`db` projects) need a kafka branch.
  `apply_events_to_new_project` (`generators/project.rs`) writes the outbox migration from
  `templates/infrastructure/outbox.sql`, which has `event_deliveries`: kafka needs a variant
  without it (and `crates/sqlx/migrations/outbox.sql` stays the byte-for-byte `db` copy).
- Tests: `tests/worker.rs` (`new_with_kafka_events_writes_no_worker_yet`,
  `generate_consumer_is_refused_outside_a_db_events_project` change meaning), the `EVENTS_DB`
  scenario as the model for `EVENTS_KAFKA`, `tests/worker_e2e.rs` as the model for
  `make test/kafka`. `RUNTIME_CRATES`, `release.yml`, `.cargo/mutants.toml` learn `puerto-kafka`.

- New runtime crate `crates/kafka` (`puerto-kafka`) on `rdkafka` (`cmake-build`):
  `KafkaPublisher` (topic `<project>.<aggregate_type>`, key `aggregate_id`, the envelope fields
  in headers, `acks=all`, idempotent producer; topics created at startup if missing) and
  `KafkaSubscription` (group id = the consumer name, subscribed to the topics of its event types,
  filtering by the `event_type` header, auto-commit off, commit after the outcome). `Retry` is
  retried in-process with `backoff_for`, then dead-lettered, so a partition stalls instead of
  reordering. Config: `KAFKA_BROKERS`, `KAFKA_TOPIC_PREFIX` (default: the project name).
- `--events kafka` projects: the worker's registry wires `KafkaPublisher` and
  `KafkaSubscription`, dead letters stay in Postgres, `event_deliveries` is not created, and a
  `compose.yaml` with Redpanda is added (the db transport needs none).
- CI: the `test/full` job installs `cmake`. Puerto's `make test/kafka` lives next to
  `make test/db`.

Test scenarios: structural (`new --events kafka` writes the kafka registry, the compose file, and
no `event_deliveries` in the migration). `make test/kafka` end to end: POST `/products` → the
consumer sees `product_created`; two events of one aggregate arrive in order; a poisoned message
ends up in `dead_letters` and the next one is still consumed; a worker restart does not
re-deliver committed offsets.

### Out of scope (Phase 13)

Cross-aggregate transactions (hand-written UoW); backfilling a consumer registered late on the
`db` transport; `LISTEN/NOTIFY` to cut the `db` transport's latency below the poll interval;
RabbitMQ and NATS adapters (one crate each, same ports); enabling events on an existing project
(`puerto add events`); event schema evolution beyond `schema_version`; the web frontend (events
are backend-only).

---

## Phase 14 — Mailer: email as a consumer of domain events

**Status:** not started; needs Phase 13.4. Its ADR decides the open points below before code.

Goal: send an email when an event happens, by declaring it, without writing a consumer:

```toml
[[mail]]
on = "user_created"            # a declared event type
template = "welcome"           # mail/templates/welcome.html (+ welcome.txt if present)
to = "email"                   # snapshot field holding the recipient
subject = "Welcome, {{ name }}"
```

Sketch: a `Mailer` port in `puerto-core` and `puerto-mail` with an SMTP adapter (`lettre`) and a
`log` adapter for development (`MAILER=smtp|log`). Templates are rendered with the snapshot as
context. Each `[[mail]]` entry becomes a generated consumer (registry entry plus handler): a
template or render error dead-letters, and an SMTP transient error retries. `puerto validate`
checks that the event exists, that `to` is a `String` field of its snapshot and that the
template file exists. Open, for the ADR: the config in puerto.toml (checked at generation time,
recommended) or in a separate runtime-read file (editable without a rebuild); the template engine
(`minijinja`); idempotency, because an SMTP send cannot share a transaction with the delivery
(record a `sent_mails` row keyed by `(event_id, mail)` before sending, accepting rare duplicates
over losses).

---

## Phase 15 — Developer experience: what the 2026-10-12 walkthrough found

**Status:** in progress (15.0–15.2 done). Source: a manual run of every command and option on five generated
projects (in-memory, `--db`, `--events db`, `--events kafka`, `--no-demo`), the apps driven in a
browser. Already fixed outside this phase, one commit each: generated code is formatted
(ADR-0019); a forced re-scaffold that drops every local value object removes `value_objects.rs`;
`generate presentation` before the other layers no longer rewrites `bootstrap.rs`; the `--db`
Makefile waits for Postgres, defines each target once, prints colors, and the web targets ask for
`make web/install`; the migration hint adds `make sqlx/prepare` and `generate consumer --help`
says `--events`. One commit per piece below, in this order.

### 15.0 — An in-memory repository that keeps what it is given ✅

**Problem.** Without `--db`, `InMemory{Entity}Repository` stores nothing ("a stand-in"): `POST`
answers 201, then `GET /…/:id` answers 404 and the list is empty. With the web frontend, which is
on by default, *create → detail* always lands on "not found": the default project's first screen
flow looks broken.

**Behaviour.**
- `InMemory{Entity}Repository { rows: RwLock<HashMap<{Entity}Id, {Entity}>> }` (std `RwLock`, no
  new dependency), `Default` + `new()` unchanged, so `bootstrap.rs` does not change.
- `save` upserts by id. `find_by_id` and `find_all` skip `deleted` rows, like the Pg adapter.
  `find_all` orders by `created_at` desc, then id; `total` counts live rows; `limit`/`offset` from
  the `PageRequest`.
- `unique` fields: `save` answers `RepositoryError::Duplicate` when another live row has the same
  value — the same 409 the Pg adapter gives.
- Process memory only: lost on restart, said in the file's doc comment and in `make run`'s banner
  line for non-db projects.
- The `inmemory-repo` snippet follows.

**Test scenarios.** Structural: the file holds the map, the unique check exists only for `unique`
fields. Generated unit tests in the adapter: save then find; find skips a deleted row; `find_all`
pages and counts; a duplicate unique value → `Duplicate`. The matrix runs them.

### 15.1 — Snippets for what the generators now write ✅

**Problem.** The 32 Rust snippets describe a project without events: `repository-trait` has `save`
and `app-use-case` calls `self.repository.save`, which do not compile in an `events` project. The
endpoint for a custom use case — what `generate use-case` asks the user to write — has no snippet,
nor does a route test. The frontend has none at all.

**Rust, added** (`.zed/snippets/rust.json`, `.vscode/puerto.code-snippets`):

| Prefix | Body |
|---|---|
| `uow-write` | `begin` → `save` → `record({Entity}Event::X(…Snapshot::from_entity(&e)))` → `commit`, with the `inspect_err` logs |
| `app-use-case-uow` | A use case impl holding `repository` + `uow_factory`, its test with `RecordingUnitOfWorkFactory` |
| `consumer-arm` | One `match event.event_type` arm: decode the snapshot, dead-letter `REASON_PAYLOAD_INVALID` |
| `poem-endpoint-action` | `POST /entities/:id/<action>`: `Path<Uuid>` → `{Entity}Id` → use case → mapped response |
| `poem-route-test` | `TestClient` over the real `OpenApiService`, one mocked use case, one status assertion |

**Frontend, new** (`.zed/snippets/typescript.json` + `tsx.json`, `.vscode/puerto.ts.code-snippets`):
`web-container` (container + view pair, `useAsync`), `web-repo-method` (an openapi-fetch call in
the HTTP repository, error code to `Error`), `web-container-test` (render with the in-memory fake,
assert on text), `web-form-field` (label + daisyUI input bound to the form state), `web-route`
(a route in `routes.tsx` — not `core/generated`, which is regenerated).

`web-route` needs a route table the user owns: the template gains `web/src/presentation/core/routes.tsx`
(`customRoutes: ReactElement[]`, written once by `new`), which the generated `<Switch>` renders right
after `/`. `generate use-case` puts the custom use case's mock in the route tests' `Mocks`
(`mocks.<action>`), where `poem-route-test` sets its expectation.

**Test scenarios.** The existing snippet test builds an entity from snippets alone; extend it to an
`events` project (`uow-write`, `app-use-case-uow`, `consumer-arm`) and to a custom endpoint with its
route test, all through clippy and the tests. Frontend snippets: the web scenario pastes each one
into a generated slice and runs typecheck, Biome and `bun test`.

### 15.2 — A startup failure is an error message, not a panic ✅

**Problem.** `bootstrap.rs` and `registry.rs` `expect` on `DATABASE_URL`, the pool and the
migrations — `make run` with the database down prints a panic and `ConnectionError`, with no
cause (refused? wrong password?) and no hint. It also breaks the generated rule "no `expect`
outside tests".

**Behaviour.**
- `puerto-web`'s `App::serve` and `puerto-events`' `Worker::run` accept a `build` whose future
  returns `Result<_, E: Display>`; an `Err` is logged and the process exits 1. The infallible form
  keeps working (minor release, ADR-0017's runtime contract).
- `puerto_sqlx::DatabaseError` keeps the driver's message (`ConnectionError { cause }`), and its
  `Display` names the host of `DATABASE_URL` (never the password).
- The generated `build_app` / `build_worker` use `?`; a missing `DATABASE_URL` says "set it in
  `.env`", a refused connection says "is the database up? `make docker-compose/up`".

Done as `App::try_serve` / `Worker::try_run` next to `serve` / `run` (one method cannot take both:
a blanket impl over `IntoEndpoint` and one over `Result` overlap), `Error::Startup`. The pool's own
`connect` retries a refused connection for its whole acquire timeout and then reports only
"pool timed out": `create_postgres_pool` opens one connection first, so the cause shows at once.

**Test scenarios.** Runtime unit tests: an `Err` from `build` exits non-zero with the message.
Matrix: a `--db` scenario starts the binary with `DATABASE_URL` pointing at a closed port and
asserts the exit code and the hint, no `panicked` in stderr.

### 15.3 — The frontend says what went wrong, next to the field

**Problem.** A 409 or 422 shows the raw code in the alert (`product.invalid_email`) and no field is
marked.

**Behaviour.** Each entity's web slice gets `domain/<entity>/messages.ts`, generated from the same
error list as `errors.rs`: `product.duplicate` → "A product with these values already exists",
`validation_error.<field>_empty` → "<Field> is required", `invalid_<vo>` → "<Field> is not valid".
The form maps a code to its field when it names one (`<field>_empty`, the VO's field) and shows the
message under that input; other codes go to the alert. Unknown codes fall back to the code itself.
English only; the file is the place to translate.

**Test scenarios.** `bun test`: a 422 `name_empty` marks the name input, a 409 shows the duplicate
message, an unknown code shows the code.

### 15.4 — Smaller things

- The list's page in the URL (`?page=2`): reload and back keep it.
- `puerto list` shows each entity's custom events, the consumers and the shared value objects.
- `generate use-case`: the impl logs `id`, not `?params` (the payload is user data —
  `architecture.md`), and the generated test is `#[ignore = "write this test"]` instead of an
  empty body that passes.
