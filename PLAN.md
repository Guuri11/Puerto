# Harbor — Roadmap

Harbor is a Rust full-stack framework built around DDD + Clean Architecture (Hexagonal / Ports & Adapters), inspired by the developer experience of Laravel and Ruby on Rails.

**Core principles:**
1. Delightful coding experience
2. AI-ready — Rust compiler's tight feedback loop makes LLM-generated code immediately verifiable
3. Convention over configuration

---

## Current state ✅

`harbor new <name>` scaffolds a workspace with three crates mirroring DDD layers, plus a `harbor.toml` and auto-generated DI bootstrap:

```
<name>/
├── harbor.toml                    # Source of truth for entities + use cases
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
        ├── main.rs                # 5-line entry point — never changes
        ├── generated.rs           # pub mod bootstrap; — never changes
        ├── generated/
        │   └── bootstrap.rs       # AUTO-GENERATED from harbor.toml
        └── api/greeting/
            ├── greeting.rs        # pub mod dto/routes/responses/error_mapper
            ├── routes.rs
            ├── dto.rs
            ├── responses.rs
            └── error_mapper.rs
```

`harbor generate scaffold <Name>` creates all DDD files, patches `lib.rs` files, updates `harbor.toml`, and regenerates `bootstrap.rs` — zero manual wiring.

---

## Phase 2 — Generators

### 2.0 `harbor new` — interactive project creation ✅ DONE

**Command signature:**
```
harbor new [--name <name>] [--db]
```

**Behavior:**
- If `--name` is not provided → prompt: `Project name:`
- If `--db` is not provided → prompt: `Include database support (SQLx + Postgres)? [y/N]`
- If both flags are provided → no prompts, fully scriptable (CI-friendly)
- Flags and prompts are independent: any combination works

**Examples:**
```bash
harbor new                          # prompts for both name and db
harbor new --name my-app            # prompts only for db
harbor new --db                     # prompts only for name
harbor new --name my-app --db       # no prompts, creates db project
```

**Test scenarios:**
- Non-interactive: `--name` flag passed to cargo-generate, project name appears in Cargo.toml
- Non-interactive: `--db` flag skips db prompt, db files created
- Non-interactive: both flags, fully silent (no stdin required)
- Interactive path is validated manually (`harbor new` with TTY)

---

### 2.1 `harbor generate scaffold <Name>` ✅ DONE

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

Auto-patches: `business/src/lib.rs`, `infrastructure/src/lib.rs`, `presentation/src/api.rs`, `harbor.toml`, `presentation/src/generated/bootstrap.rs`.

---

### 2.2 `harbor generate use-case <Entity> <action>` ✅ DONE

Adds a single use case (trait + impl + unit tests) for an existing entity.

**Spec:**

**Files created:**
```
business/src/domain/<entity>/use_cases/<action>.rs   # Params struct + UseCaseTrait
business/src/application/<entity>/<action>.rs         # UseCaseImpl + unit tests
```

**Auto-patches:**
- `business/src/domain/<entity>/use_cases.rs` — append `pub mod <action>;`
- `harbor.toml` — append `<action>` to the entity's `use_cases` array
- `presentation/src/generated/bootstrap.rs` — regenerated

**Behavior:**
- `<Entity>` must be PascalCase; error if not found in `harbor.toml`
- `<action>` must be snake_case
- Generated `UseCaseImpl` struct has one field: `repository: Arc<dyn <Entity>RepositoryTrait>`
- Tests follow AAA pattern with `Mock<Entity>Repository`
- Running twice for same entity+action is a no-op (idempotent)

**Test scenarios to cover:**
- Creates both files with correct content
- Patches `use_cases.rs` without removing existing entries
- Updates `harbor.toml` entity's use_cases array
- Regenerates bootstrap with new use case wired
- PascalCase normalisation (`orderItem` → `OrderItem`)
- Error when entity not in harbor.toml

---

### 2.3 `harbor generate migration <name>` ✅ DONE

Wraps `sqlx migrate add` with Harbor conventions.

**Command signature:**
```
harbor generate migration <name>
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
- File body is created by `sqlx migrate add` — harbor adds a comment header after creation:
  ```sql
  -- Harbor migration: <name>
  -- Run `make sqlx/prepare` after editing this file.
  ```

**No lib.rs patches.** No harbor.toml changes.

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

## Phase 3 — Auto-patching lib.rs ✅ DONE

`harbor generate scaffold` automatically updates:
- `business/src/lib.rs` — inline `domain { }` and `application { }` blocks
- `infrastructure/src/lib.rs` — appends new module block
- `presentation/src/api.rs` — appends `pub mod <name>;`
- `harbor.toml` — appends `[[entity]]` block
- `presentation/src/generated/bootstrap.rs` — full regeneration

---

## Phase 4 — Database layer (SQLx) ✅ DONE

### 4.0 `harbor new --db` — project template with database support ✅

Extends the base `harbor new` with database plumbing baked into the generated project.

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
- `harbor new --db` generates all additional files
- `.cargo/config.toml` contains `SQLX_OFFLINE = "true"`
- `infrastructure/migrations/` directory exists
- `infrastructure/src/db.rs` exists with correct content
- `infrastructure/Cargo.toml` has sqlx dependency with postgres feature

---

### 4.1 `harbor generate scaffold <Name> --db` — entity with SQLx repository ✅

**Command signature:**
```
harbor generate scaffold <Name> --db
```

Without `--db` the command behaves as today (Phase 2.1). With `--db`:
- Instead of `InMemoryEntityRepository`, generates a `PgEntityRepository` backed by SQLx
- Adds `entity.rs` (DB row struct + conversions)
- Runs `harbor generate migration create_<snake>_table` automatically

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
- All patches from base scaffold (lib.rs files, api.rs, harbor.toml)
- `harbor.toml` entity block gets `db = true` flag
- Runs `sqlx migrate add create_{snake}_table --source infrastructure/migrations` as subprocess
- Regenerates `bootstrap.rs` with SQLx wiring

**`harbor.toml` schema addition:**
```toml
[[entity]]
name = "Product"
use_cases = ["create_product"]
db = true          # NEW: controls InMemory vs Pg repository in bootstrap
```

**Behavior:**
- Existing entities without `db = true` continue to use `InMemoryEntityRepository`
- `bootstrap.rs` mixes InMemory and Pg repos correctly (each entity independent)
- Error if `infrastructure/migrations/` does not exist — hint: use `harbor new --db` or create it manually
- Error if sqlx CLI not in `$PATH` — same install instructions as 2.3

**Test scenarios:**
- With `--db`: creates `entity.rs`, `repository.rs` contains `PgPool`, no `InMemory`
- Without `--db`: creates `repository.rs` contains `InMemoryProductRepository` (existing behaviour)
- `harbor.toml` gets `db = true` when `--db` passed
- `bootstrap.rs` uses `Pg{pascal}Repository` for db entities, `InMemory` for others
- Error when sqlx not installed and `--db` passed
- Error when `infrastructure/migrations/` not found and `--db` passed

---

### 4.2 `harbor generate use-case <Entity> <action>` with db entities — unchanged ✅

`harbor generate use-case` does not change in Phase 4. The use case trait + impl are database-agnostic. Bootstrap regeneration already handles the correct repo type via `db` flag in `harbor.toml`.

---

## Phase 5 — Logger abstraction ✅ DONE

Add a `LoggerTrait` domain port so every generated use case can log without coupling to a concrete logging library. Follows the same Ports & Adapters pattern as repositories.

### 5.0 — Logger trait + infrastructure impl ✅

**Goal:** every `harbor new` project ships with a working logger, wired automatically into every use case.

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

| File | Change |
|------|--------|
| `business/src/domain/logger.rs` | NEW — `LoggerTrait` + mockall mock |
| `business/src/lib.rs` | Add `pub mod logger;` |
| `business/src/application/greeting/get_greeting.rs` | Add `logger` field + log calls + mock in tests |
| `infrastructure/src/logger.rs` | NEW — `TracingLogger` |
| `infrastructure/src/lib.rs` | Add `pub mod logger;` |
| `infrastructure/Cargo.toml` | Add `tracing = "0.1"` |
| `presentation/src/main.rs.liquid` | Add `tracing_subscriber` init |
| `presentation/Cargo.toml` | Add `tracing-subscriber` |
| `presentation/src/generated/bootstrap.rs` | Wire `TracingLogger` into all use cases |

#### Files changed in Harbor CLI

| File | Change |
|------|--------|
| `crates/cli/src/scaffold.rs` | `UC_IMPL` template: add `logger` field + log calls |
| `crates/cli/src/scaffold.rs` | `generate_bootstrap_content()`: wire `TracingLogger` |

---

#### Test scenarios

- `harbor new` — generated project compiles with logger wired (`make test/full`)
- `harbor generate scaffold <Name>` — generated use case impl has `logger` field
- `harbor generate use-case <Entity> <action>` — generated impl has `logger` field
- `bootstrap.rs` contains `TracingLogger` import and `logger` wired into every use case
- Mock logger works in unit tests (no real tracing calls)

---

## Phase 6 — IDE Snippets

Every `harbor new` project ships with snippet files for **Zed** and **VS Code** (VS Code format is also compatible with nvim+LuaSnip). Both files share the same JSON content — Zed and VS Code use the same TextMate snippet format.

### 6.0 Snippet files in `harbor new`

`harbor new` writes two files after cargo-generate completes:

```
.zed/snippets/rust.json           # Zed project-local snippets — auto-loaded by Zed
.vscode/harbor.code-snippets      # VS Code workspace snippets — auto-loaded; LuaSnip-compatible
```

Written by `snippets::apply(base, None)` called from `new_project()` after cargo-generate.

**Zed note:** project snippets at `.zed/snippets/` are loaded automatically — no copying needed.
**LuaSnip note:** add `require("luasnip.loaders.from_vscode").lazy_load({ paths = { "./.vscode" } })` to init.

---

### 6.1 `harbor generate snippets [--ide <ide>]`

Adds or regenerates snippet files in an existing Harbor project.

**Command signature:**
```
harbor generate snippets                 # writes both Zed + VS Code
harbor generate snippets --ide zed       # .zed/snippets/rust.json only
harbor generate snippets --ide vscode    # .vscode/harbor.code-snippets only
```

**Behavior:**
- Overwrites existing files (idempotent)
- `--ide` values: `zed`, `vscode` (error on unknown value)
- Prints file path(s) written + IDE-specific setup note

---

### Snippet inventory

| Prefix | Layer | Description |
|--------|-------|-------------|
| `lib-domain-entity` | lib.rs | Inline domain entity block |
| `lib-application-entity` | lib.rs | Inline application entity block |
| `domain-model` | domain | Struct + Props + new(props) + from_repository() |
| `domain-errors` | domain | thiserror enum with machine-readable codes |
| `repository-trait` | domain | RepositoryTrait + Send+Sync + mockall mock |
| `domain-use-case` | domain | Params + UseCaseTrait |
| `app-use-case` | application | UseCaseImpl + LoggerTrait + unit tests |
| `lib-infra-entity` | infra lib.rs | Infrastructure entity block (InMemory) |
| `lib-infra-entity-db` | infra lib.rs | Infrastructure entity block (SQLx) |
| `persistence-entity` | infrastructure | EntityDb struct + TryFrom/From conversions |
| `persistence-repo` | infrastructure | PgEntityRepository + find_by_id + save |
| `lib-presentation-entity` | api.rs | Presentation entity module decls |
| `poem-dto` | presentation | EntityDto (Object) + from_domain() |
| `poem-request-dto` | presentation | Request DTO struct |
| `poem-response-enum` | presentation | ApiResponse enum + from_status() |
| `poem-error-mapper` | presentation | IntoErrorResponse impl |
| `poem-api-struct` | presentation | EntityApi struct + POST endpoint |
| `poem-endpoint` | presentation | Single #[oai] endpoint handler |
| `cfg-test` | test | #[cfg(test)] mod tests block |
| `should-do-test` | test | #[tokio::test] with AAA pattern |
| `object-mother` | test | Object Mother builder pattern |
| `sqlx-test` | test | #[sqlx::test(migrations="migrations")] single test |
| `sqlx-repo-test-module` | test | Full #[cfg(test)] for PgEntityRepository |

**Harbor adaptations vs ant_backend:**
- `LoggerTrait` (not `Logger`) — already has `Send + Sync`
- Mocks in `repository.rs pub mod mocks` (not separate file)
- `from_repository(data: Entity) -> Self` (not individual fields)
- No `SecurityService`/JWT, no events, no UoW, no SSE handlers
- `EntityError::RepositoryError` terminal (no `From<RepositoryError>`)
- SQL params escaped as `\$1` in snippet JSON to avoid tab-stop conflict

---

### 6.2 SQLx integration tests in `--db` scaffold (already implemented)

`harbor generate scaffold <Name> --db` generates `infrastructure/src/<entity>/repository.rs`
with a `#[cfg(test)] mod integration_tests` block containing three `#[sqlx::test]` tests:
- `should_persist_and_retrieve_by_id`
- `should_return_none_for_nonexistent_id`
- `should_update_entity_on_save_conflict`

Migrations path: `"migrations"` (relative to infrastructure crate root = `infrastructure/migrations/`).

---

### Test scenarios

- `harbor new` → `.zed/snippets/rust.json` exists with valid JSON
- `harbor new` → `.vscode/harbor.code-snippets` exists with valid JSON
- `harbor generate snippets` → overwrites both files (idempotent)
- `harbor generate snippets --ide zed` → only `.zed/snippets/rust.json` created
- `harbor generate snippets --ide vscode` → only `.vscode/harbor.code-snippets` created
- `harbor generate snippets --ide unknown` → error message

---

## Phase 7 — Project-level metadata & DX improvements

### 7.0 `[project] db = true` in harbor.toml ✅ DONE

Track project-level database support in `harbor.toml` so tools and future generators can read whether the project was created with `--db` without inspecting generated files.

**Schema addition:**
```toml
[project]
name = "my-app"
db = true          # present only when harbor new --db was used
```

**Rules:**
- `harbor new --db` sets `project.db = true` via `apply_db_to_new_project`
- `harbor new` (no db) omits the field entirely (serialised with `skip_serializing_if`)
- `harbor generate scaffold <Name>` can read this flag in the future to default `--db` behaviour
- `harbor generate scaffold <Name> --db` errors when `project.db` is absent (future enforcement)

**Files changed:**
- `crates/cli/src/harbor_toml.rs` — add `db: bool` to `Project` struct
- `crates/cli/src/scaffold.rs` — `apply_db_to_new_project` sets `project.db = true`

**Test scenarios:**
- `harbor new --db` → `harbor.toml` contains `db = true` under `[project]`
- `harbor new` (no db) → `harbor.toml` does not contain `db = true`

---

### 7.1 Review logging in ant_backend for presentation layer

Investigate how `ant_backend` handles logging at the presentation / HTTP handler level:
- Does it log request/response metadata (path, status, latency)?
- Is there a middleware or `poem` hook that logs per-request?
- Should Harbor wire `tower-http` `TraceLayer` or a poem equivalent automatically?

Goal: decide whether Harbor-generated `main.rs` / `bootstrap.rs` should include HTTP-level request logging out of the box.

---

### 7.2 Spring Boot-like startup banner

Print a Harbor ASCII banner on startup, similar to Spring Boot's banner:

```
  _   _             _
 | | | | __ _ _ __| |__   ___  _ __
 | |_| |/ _` | '__| '_ \ / _ \| '__|
 |  _  | (_| | |  | |_) | (_) | |
 |_| |_|\__,_|_|  |_.__/ \___/|_|

 :: Harbor ::  (v0.3.0)
```

Printed to stdout before `build_app()` is called, controlled by `HARBOR_BANNER=false` env var to suppress in tests.

Options to evaluate:
- Static string in `main.rs.liquid` template
- Read version from `CARGO_PKG_VERSION` at compile time
- Optional: color via `colored` crate (adds dependency — may not be worth it)

---

## Phase 8 — Full-stack (frontend)

TBD. Options to evaluate:
- Server-side rendering with a Rust template engine (Askama / Tera)
- HTMX + Askama for reactive UIs without JS build step
- API-only + separate frontend (Leptos / Dioxus)
