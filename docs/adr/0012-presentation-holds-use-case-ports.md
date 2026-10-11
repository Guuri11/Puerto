# 0012 — Presentation holds use case ports; routes are tested with mocks

**Status:** Accepted (2026-10-11). Amends [0004](0004-generate-use-case-never-overwrites.md): the
field `generate use-case` adds is `Arc<dyn …UseCaseTrait>`.

## Context

`architecture.md` says dependencies are injected as `Arc<dyn Trait>`, never concrete types. The
application layer honoured it; `{Entity}Api` did not — it held `Arc<CreateProductUseCaseImpl>`.
Presentation was coupled to `business::application`, and a route could not be exercised without
the real use case and a repository behind it. Generated projects shipped **no presentation
tests**.

## Decision

- `{Entity}Api` fields are `Arc<dyn {Uc}UseCaseTrait>`. `bootstrap.rs` is unchanged: the
  `Arc<{Uc}UseCaseImpl>` it builds coerces to the trait object at the struct literal, so the
  composition root is the only file naming implementations. No `{Entity}Api::new(…)` — the fields
  are public and a struct literal is already the constructor.
- Every use case trait file carries `mocks::Mock{Uc}UseCase` (`mockall::mock!`) behind
  `#[cfg(any(test, feature = "test-utils"))]` — the gate the repository mocks already use.
- The Object Mothers' `tests` module in `business/src/lib.rs` moves from `#[cfg(test)]` to the same
  gate, so presentation builds return values with the mothers instead of a second set of fixtures.
- `presentation` dev-depends on `business` with `test-utils` and on `poem` with `test`.
- CRUD `routes.rs` is generated with a test module that drives the **real** `OpenApiService`
  through `poem::test::TestClient` — the paths, the extractors, the JSON body parsing and the
  `ApiResponse` status mapping are all exercised. One mock per use case lives in a `Mocks` struct;
  a test sets an expectation only on the use case its route should call, so a route wired to the
  wrong one panics. One test per status path: create 201/422, get 200/404, list 200/500,
  update 200/404, delete 204/404. Request bodies come from the type registry's default literals
  (`FieldCtx::default_literal`) serialised with poem-openapi's `ToJSON`.

## Consequences

- `test-utils` now also compiles the Object Mothers into `business` when another crate's tests
  enable it; they use `expect`, which is acceptable in test support code. A plain build is
  unaffected.
- The mocked use cases do not validate, so the route tests check HTTP wiring and status mapping
  only. Validation stays covered where it lives (domain and application tests).
- Custom use cases added by `generate use-case` get the port field and the mock, but no endpoint
  and no route test — their HTTP shape still cannot be inferred (ADR-0004).
