# Business Layer — AI Agent Notes

This crate contains **pure Rust business logic**. It has no dependency on any web framework, database driver, or HTTP client.

## Layer Structure

```
business/src/
├── lib.rs                    # The whole module tree, as nested `pub mod {}` blocks — no `mod.rs`
├── domain/
│   ├── shared/               # Shared value objects (`[[value_object]]` in puerto.toml), if any
│   └── <entity>/
│       ├── model.rs          # Entity + Props + constructors
│       ├── value_objects.rs  # Value objects of this entity, if any
│       ├── errors.rs         # EntityError enum (thiserror, machine-readable codes)
│       ├── repository.rs     # Repository trait (port) + mockall mock
│       └── use_cases/
│           └── <action>.rs   # Params struct + UseCaseTrait (declared in lib.rs)
├── application/
│   └── <entity>/
│       └── <action>.rs       # UseCaseImpl + its tests
└── tests/mothers/            # Object Mothers (test fixtures)
```

Adding a use case is one line in `lib.rs` (under both `domain::<entity>::use_cases` and
`application::<entity>`) plus its two files — `puerto generate use-case` does both.

## Critical Rules

- **Two constructors**: `new(props)` validates and returns `Result<Self, Error>`. `from_repository(data)` bypasses validation — only infrastructure calls this.
- **Typed ids**: an entity's id is its own type, `ProductId` (`model.rs`): `ProductId::generate()` for
  a new one, `from_uuid` / `.value()` only at the edges (routes, DTOs, rows). Never pass a bare `Uuid`.
- **Typed fields**: when `entity.fields` is defined in `puerto.toml`, generated structs include custom fields (Props, Params, DTOs, DB rows). When empty, defaults to `name: String`.
- **Use case file structure**: exactly `Params struct` + `Trait`. The `Impl` struct lives in the **application layer**.
- **Dependencies injected as `Arc<dyn Trait>`** — never concrete types.
- **No `unwrap()`/`expect()`** outside of tests.
- **Logging with `tracing` directly** (`tracing::info!(id = %params.id, "get_product: start")`) —
  there is no logger port to inject or mock. Log start and end of a use case; never the payload.
- **Value objects**: private inner value, `new()` returning `Result`; enum value objects implement
  `std::str::FromStr` (`"Active".parse::<Status>()`) and `as_str()`.
- **Error codes are machine-readable**: `"entity.not_found"`, `"entity.validation_error.name_empty"` — never human sentences.

## Tests

- Tests live **inside the implementation file** in `#[cfg(test)] mod tests { ... }`.
- Write tests **before** implementation (RED-GREEN-REFACTOR).
- Test naming: `should_<BUSINESS_EXPECTATION>_when_<BUSINESS_SCENARIO>`.
- Always use `// Arrange`, `// Act`, `// Assert` comments.
- Import mocks with: `use crate::domain::<entity>::repository::mocks::Mock<Entity>Repository;`
- Assert on `.to_string()` error codes — never match on human-readable messages.

## Mocks

The `mocks` module in each `repository.rs` is gated with:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub mod mocks { ... }
```

`mockall` is both an optional dep (feature `test-utils`) and a dev-dependency — so tests inside this crate work without activating the feature.

## Adding a New Entity

Run `puerto generate domain <Name>` to scaffold the domain layer in isolation, or `puerto generate scaffold <Name>` to do all layers at once.

**Layer-by-layer (recommended for AI-assisted development):**
```bash
puerto generate domain <Name>          # domain + Object Mother → Next: application
puerto generate application <Name>     # use case impls        → Next: repository
puerto generate repository <Name>      # infra adapter         → Next: presentation
puerto generate presentation <Name>    # HTTP layer + bootstrap wired
```
Each command validates that the prior step completed (entity must be in `puerto.toml`).

## Object Mother

Every entity gets a test fixture in `business/src/tests/mothers/<entity>_mother.rs`. The
`tests` module is compiled under `#[cfg(any(test, feature = "test-utils"))]`, so presentation's
route tests use the same mothers — and the use case mocks (`use_cases::<action>::mocks`):

```rust
let widget = WidgetMother::random();
let invalid = WidgetMother::new().with_empty_name().build_props();
let batch   = WidgetMother::random_vec(5);
```

When `entity.fields` is defined, the Mother generates builder methods for each custom field:

```rust
// For a Product entity with fields: name, price, sku
let product = ProductMother::new()
    .with_name("Widget")
    .with_price(999)
    .build();
let invalid = ProductMother::new()
    .with_empty_name()
    .build_props();
```

- `String` fields get `.with_<field>(value)` and `.with_empty_<field>()`
- `Option<T>` fields get `.with_<field>(value)` only
- Numeric/bool/Uuid/DateTime fields get `.with_<field>(value)`
- `Vec<T>` fields get `.with_<field>(values)`

The `tests::mothers` block in `lib.rs` is patched automatically by `puerto generate domain` and `puerto generate scaffold`.
