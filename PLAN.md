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

### 2.3 `harbor generate migration <name>` — TODO

Wraps `sqlx migrate add` with Harbor conventions.

**Spec:**

**What it does:**
1. Creates `infrastructure/persistence/migrations/<timestamp>_<name>.sql`
2. Adds comment header: `-- Harbor migration: <name>`
3. Prints reminder to run `make sqlx/prepare` after editing

**Requires:** SQLx CLI installed + `DATABASE_URL` set in `.env`

**Error if:** SQLx CLI not found (print install instructions)

---

## Phase 3 — Auto-patching lib.rs ✅ DONE

`harbor generate scaffold` automatically updates:
- `business/src/lib.rs` — inline `domain { }` and `application { }` blocks
- `infrastructure/src/lib.rs` — appends new module block
- `presentation/src/api.rs` — appends `pub mod <name>;`
- `harbor.toml` — appends `[[entity]]` block
- `presentation/src/generated/bootstrap.rs` — full regeneration

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
