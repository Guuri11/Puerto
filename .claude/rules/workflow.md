# Implementation Workflow (TDD)

**CRITICAL:** Tests are written FIRST. Never write implementation code before a failing test exists.

---

## The 10-Step Workflow

### 1. Analyze — Business Requirements First

- Identify the business rule being implemented
- List ALL test scenarios before writing any code:
  - Happy path
  - Empty / invalid inputs
  - Edge cases (boundary values, status transitions)
  - Regression cases (if fixing a bug)

### 2. Write Failing Tests — RED

Do NOT write any implementation code yet.

```rust
#[tokio::test]
async fn should_create_entity_when_name_is_valid() {
    // Arrange, Act, Assert
}

#[tokio::test]
async fn should_reject_entity_when_name_is_empty() {
    // Arrange, Act, Assert
}
```

Run `make test` → expected: tests fail (compile error or assertion failure). That is correct.

### 3. Implement Domain Model — GREEN

Update `business/src/domain/<entity>/model.rs`:
- Add entity struct + Props struct
- Implement `new(props)` with validation
- Implement `from_repository()` (no validation)

Run `make test` → tests that only test the model should now pass.

### 4. Define Use Case Contract

In `business/src/domain/<entity>/use_cases/<action>.rs`, define:
- `Params` struct
- `UseCaseTrait` (`#[async_trait]`)

This is the contract. Implementation comes next.

### 5. Implement Use Case — GREEN

In `business/src/application/<entity>/<action>.rs`:
- Implement `UseCaseTrait` for `UseCaseImpl`
- Wire repository via `Arc<dyn RepositoryTrait>`
- Write minimal code to make all tests pass

Run `make test` → all tests should pass.

### 6. Refactor

Improve code quality without changing behavior. Run `make test` after every change.

Common refactors:
- Extract validation into domain model methods
- Improve error messages
- Simplify repository mock setup

### 7. Infrastructure

Implement (or update) the repository adapter in `infrastructure/src/<entity>/repository.rs`:
- Start with `InMemoryRepository` as the default
- Replace with SQLx adapter when adding database support

### 8. Presentation — API First

**Update `routes.rs` + `dto.rs` BEFORE writing any handler code.**

Then implement:
1. `dto.rs` — request/response structs with `#[derive(Object)]`
2. `responses.rs` — `#[derive(ApiResponse)]` enum with `from_status()` helper
3. `error_mapper.rs` — `impl IntoErrorResponse for EntityError`
4. `routes.rs` — `#[OpenApi]` handler that calls the use case

### 9. Wire Dependency Injection

Update `presentation/src/main.rs`:
- Instantiate the infrastructure adapter
- Instantiate the use case with the adapter
- Pass the use case to the API struct

### 10. Final Check

```bash
make test       # Structural tests pass
make test/full  # Generated project compiles + internal tests pass (~20s)
make lint       # Zero clippy warnings
make format     # Code formatted
```

---

## Key Paths (Generated Projects)

| Layer | Path |
|-------|------|
| Domain model | `business/src/domain/<entity>/model.rs` |
| Domain errors | `business/src/domain/<entity>/errors.rs` |
| Repository trait | `business/src/domain/<entity>/repository.rs` |
| Use case trait | `business/src/domain/<entity>/use_cases/<action>.rs` |
| Use case impl | `business/src/application/<entity>/<action>.rs` |
| Infra repository | `infrastructure/src/<entity>/repository.rs` |
| API routes | `presentation/src/api/<entity>/routes.rs` |
| API DTOs | `presentation/src/api/<entity>/dto.rs` |
| API responses | `presentation/src/api/<entity>/responses.rs` |
| Error mapping | `presentation/src/api/<entity>/error_mapper.rs` |
| DI bootstrap | `presentation/src/main.rs` |

---

## Code Review Checklist

- [ ] Tests written before implementation (verify via git history if needed)
- [ ] All tests pass: `make test` + `make test/full`
- [ ] Lint clean: `make lint`
- [ ] Formatted: `make format`
- [ ] No `unwrap()`/`expect()` outside test code
- [ ] Domain model has no infrastructure or presentation imports
- [ ] DTOs never expose domain models directly
- [ ] Error codes are machine-readable identifiers (`"entity.not_found"`, not `"Entity not found"`)
- [ ] New entity modules declared inline in `lib.rs` (no `mod.rs` files)
