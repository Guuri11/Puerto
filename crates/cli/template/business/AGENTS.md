# Business Layer — AI Agent Notes

This crate contains **pure Rust business logic**. It has no dependency on any web framework, database driver, or HTTP client.

## Layer Structure

```
business/src/
├── domain/
│   └── <entity>/
│       ├── model.rs          # Entity + Props + constructors
│       ├── errors.rs         # EntityError enum (thiserror)
│       ├── repository.rs     # Repository trait (port) + mock
│       ├── use_cases/
│       │   ├── <action>.rs   # Params struct + Trait
│       │   └── mod.rs
│       └── mod.rs
└── application/
    └── <entity>/
        ├── <action>.rs       # UseCaseImpl + tests
        └── mod.rs
```

## Critical Rules

- **Two constructors**: `new(props)` validates and returns `Result<Self, Error>`. `from_repository(data)` bypasses validation — only infrastructure calls this.
- **Use case file structure**: exactly `Params struct` + `Trait`. The `Impl` struct lives in the **application layer**.
- **Dependencies injected as `Arc<dyn Trait>`** — never concrete types.
- **No `unwrap()`/`expect()`** outside of tests.
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

Run `harbor generate domain <Name>` to scaffold the domain layer in isolation, or `harbor generate scaffold <Name>` to do all layers at once.

**Layer-by-layer (recommended for AI-assisted development):**
```bash
harbor generate domain <Name>          # domain + Object Mother → Next: application
harbor generate application <Name>     # use case impls        → Next: repository
harbor generate repository <Name>      # infra adapter         → Next: presentation
harbor generate presentation <Name>    # HTTP layer + bootstrap wired
```
Each command validates that the prior step completed (entity must be in `harbor.toml`).

## Object Mother

Every entity gets a test fixture in `business/src/tests/mothers/<entity>_mother.rs`:

```rust
let widget = WidgetMother::random();
let invalid = WidgetMother::new().with_empty_name().build_props();
let batch   = WidgetMother::random_vec(5);
```

The `tests::mothers` block in `lib.rs` is patched automatically by `harbor generate domain` and `harbor generate scaffold`.
