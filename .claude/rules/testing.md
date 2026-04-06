# Testing Guidelines (TDD)

## Core Principle

**Write tests FIRST, before any implementation. Always.**

TDD is not optional. It is the workflow. Tests define the contract; implementation fulfills it.

---

## The Cycle

```
RED   → Write a failing test that describes a business requirement
GREEN → Write the minimal code that makes it pass
REFACTOR → Improve quality without changing behavior (run tests after every change)
```

Repeat. Never skip RED.

---

## Test Naming

**Pattern:** `should_<BUSINESS_EXPECTATION>_when_<BUSINESS_SCENARIO>`

Names describe **business behavior**, not implementation details.

```rust
// Good — business-focused
should_create_greeting_when_name_is_valid
should_reject_greeting_when_name_is_empty
should_return_cached_greeting_when_found_in_repository

// Bad — implementation-focused
should_return_ok_result
should_call_repository_once
should_set_message_field
```

---

## Test Location

Tests live **inside the file being tested** in a `#[cfg(test)]` module:

```rust
// business/src/application/greeting/get_greeting.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn should_create_greeting_when_name_is_valid() {
        // ...
    }
}
```

---

## AAA Pattern (Arrange-Act-Assert)

Always use explicit comments. They communicate intent to humans and AI agents alike.

```rust
#[tokio::test]
async fn should_return_error_when_name_is_empty() {
    // Arrange
    let mut mock_repo = MockGreetingRepository::new();
    mock_repo.expect_find_by_name().times(0);

    let use_case = GetGreetingUseCaseImpl {
        repository: Arc::new(mock_repo),
    };

    // Act
    let result = use_case
        .execute(GetGreetingParams { name: "".into() })
        .await;

    // Assert
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "greeting.validation_error.name_empty"
    );
}
```

---

## Mocking

Mocks live in the `repository.rs` of each domain entity, behind a feature flag:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use super::*;

    mock! {
        pub GreetingRepository {}
        #[async_trait]
        impl GreetingRepositoryTrait for GreetingRepository {
            async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, GreetingError>;
        }
    }
}
```

Import in tests:

```rust
use crate::domain::greeting::repository::mocks::MockGreetingRepository;
```

The `business/Cargo.toml` has `mockall` both as an optional dependency (feature `test-utils`) **and** as a dev-dependency — so tests inside `business` itself work without the feature flag.

---

## Test Categories (by priority)

| Priority | Category | Description |
|----------|----------|-------------|
| CRITICAL | Business Rule | Core domain invariants — always write these |
| HIGH | Edge Case | Boundary conditions: empty strings, limits, status transitions |
| HIGH | Regression | One test per bug fix — prevents regressions |
| MEDIUM | Integration | Compilation + full stack (see `make test/full`) |

---

## DO and DON'T

### DO
- Write tests before implementation (RED first)
- Test business behavior, not internal state
- Use realistic data (`"Alice"`, not `"test"`)
- Assert on error code strings (`"greeting.not_found"`)
- Cover happy path AND edge cases
- Use `#[tokio::test]` for all async tests

### DON'T
- Test implementation details (private methods, mock call counts unless semantically meaningful)
- Write tests just for coverage metrics
- Mock everything — prefer real domain objects when possible
- Make tests depend on execution order
- Use `unwrap()` in test assertions — use `assert!` / `assert_eq!` / `match` explicitly

---

## Running Tests

```bash
make test        # Fast — structural tests (file layout, cargo-generate output)
make test/full   # Slow (~20s) — generates a real project, compiles it, runs its internal tests
```

Single test with output:

```bash
cargo test should_create_greeting_when_name_is_valid -- --nocapture
```
