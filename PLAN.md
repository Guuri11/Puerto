# Harbor — Roadmap

Harbor is a Rust full-stack framework built around DDD + Clean Architecture (Hexagonal / Ports & Adapters), inspired by the developer experience of Laravel and Ruby on Rails.

**Core principles:**
1. Delightful coding experience
2. AI-ready — Rust compiler's tight feedback loop makes LLM-generated code immediately verifiable
3. Convention over configuration

---

## Current state

`harbor new <name>` scaffolds a workspace with three crates that mirror the DDD layers:

```
<name>/
├── business/          # Domain + Application (pure Rust, no framework deps)
│   └── src/
│       ├── domain/greeting/
│       │   ├── model.rs           # Entity with business rules
│       │   ├── errors.rs          # thiserror domain errors
│       │   ├── repository.rs      # Trait (port) + mockall mock
│       │   └── use_cases/
│       │       └── get_greeting.rs  # Use case trait
│       └── application/greeting/
│           └── get_greeting.rs    # Use case implementation + unit tests
├── infrastructure/    # Adapters (DB, HTTP clients, etc.)
│   └── src/greeting/
│       └── repository.rs          # In-memory implementation (replace with SQLx)
└── presentation/      # poem-openapi REST API
    └── src/
        ├── main.rs                # DI wiring + server bootstrap
        └── api/greeting/
            ├── routes.rs          # OpenApi endpoints
            ├── dto.rs             # Request/Response objects
            ├── responses.rs       # poem ApiResponse enums
            └── error_mapper.rs    # Domain error → HTTP status
```

Dependency rule (inward only): `presentation → infrastructure → application → domain`

---

## Phase 2 — Generators

### 2.1 `harbor generate entity <Name>`

Scaffolds all files for a new DDD entity across every layer.

**Files created:**
```
business/src/domain/<name>/
  model.rs           # Entity struct + Props struct + new() + business rules
  errors.rs          # <Name>Error enum with thiserror
  repository.rs      # <Name>RepositoryTrait + mockall mock behind test-utils feature
  use_cases.rs       # pub mod <action>;  (sibling to use_cases/)
  use_cases/
    <action>.rs      # Use case trait + Params

business/src/application/<name>/
  <action>.rs        # UseCaseImpl + unit tests

infrastructure/src/<name>/
  repository.rs      # Stub implementation (InMemory or SQLx skeleton)

presentation/src/api/<name>/
  routes.rs          # Empty OpenApi impl struct
  dto.rs             # Empty DTO placeholder
  responses.rs       # Basic ApiResponse enum (Ok / BadRequest / NotFound / InternalError)
  error_mapper.rs    # IntoErrorResponse impl for <Name>Error
```

**Updates required after generation (no auto-patch yet):**
- `business/src/lib.rs` — add `pub mod <name>;` inside `domain { }` and `application { }` inline blocks
- `infrastructure/src/lib.rs` — add `pub mod <name>;` inside `pub mod <name> { }` block
- `presentation/src/api.rs` — add `pub mod <name>;`

> Phase 3 will auto-patch these.

---

### 2.2 `harbor generate use-case <Entity> <action>`

Adds a single use case (trait + implementation + unit test) for an existing entity.

**Files created:**
```
business/src/domain/<entity>/use_cases/<action>.rs   # Use case trait + Params + Response structs
business/src/application/<entity>/<action>.rs         # UseCaseImpl + #[tokio::test] unit tests
```

**Updates required after generation:**
- `business/src/domain/<entity>/use_cases.rs` — add `pub mod <action>;`
- `business/src/lib.rs` — add `pub mod <action>;` inside `application { <entity> { } }` inline block

**Test convention (from ant_backend):**
```rust
#[tokio::test]
async fn should_<expected_outcome>_when_<condition>() {
    // Arrange
    // Act
    // Assert
}
```

---

### 2.3 `harbor generate migration <name>`

Wraps `sqlx migrate add` with Harbor conventions.

**What it does:**
1. Creates `infrastructure/persistence/migrations/<timestamp>_<name>.sql`
2. Adds a comment header with entity name and description
3. Reminds to run `make sqlx/prepare` after editing

**Requires:** SQLx CLI installed + `DATABASE_URL` set in `.env`

---

## Phase 3 — Auto-patching lib.rs

When a generator creates new files, it should automatically update `lib.rs` / `api.rs` / `use_cases.rs` to declare the new module, instead of printing manual instructions.

Approach: parse existing file, find the right inline block, insert `pub mod <name>;` at the correct indentation level.

---

## Phase 4 — Database layer (SQLx)

When generating an entity with `--db` flag:

- `infrastructure/src/<name>/repository.rs` becomes a full SQLx implementation
- `infrastructure/src/<name>/entity.rs` added (SQLx row struct + `From` impl)
- Migration file created automatically (see 2.3)
- `DATABASE_URL` injected into `.env` if not present

---

## Phase 5 — Full-stack (frontend)

TBD. Options to evaluate:
- Server-side rendering with a Rust template engine (Askama / Tera)
- HTMX + Askama for reactive UIs without JS build step
- API-only + separate frontend (Leptos / Dioxus)
