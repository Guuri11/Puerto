# 0015 — Repositories return one `RepositoryError`

**Status:** Accepted (2026-10-11).

## Context

Every repository port returned its entity's error (`ProductError`), and every Pg repository
re-implemented the driver mapping inline — `23505` → `Duplicate`, anything else →
`RepositoryError` — once per entity and once more in the IDE snippet. A row that no longer passed
the domain's rules surfaced as whatever VO error `TryFrom` produced (`product.invalid_name`, a
422), blaming the caller for stored data. `document-api` splits it: one shared `RepositoryError`
that adapters map onto, and per-context errors with `From<RepositoryError>` (backlog T-28).

## Decision

- `business/src/domain/shared/repository_error.rs` ships in the template:
  `RepositoryError { Duplicate, InvalidData, Unavailable }` (`repository.*` codes, `Copy`, `Eq`).
  Three variants, not more: they are the three outcomes a use case reacts to differently. "Not
  found" is not one — `find_by_id` answers `Ok(None)` and the use case decides it is an error.
- Every repository port and its mock return `Result<_, RepositoryError>`. Each entity error has
  `impl From<RepositoryError>`: `Duplicate → Duplicate`, `InvalidData | Unavailable →
  RepositoryError`, one test per arm. Use cases keep `?`.
- **The entity's error codes do not change.** The API contract (ADR-0007) still answers
  `product.duplicate` / `product.repository_error`; `RepositoryError`'s own codes only reach logs.
- Pg repositories call `infrastructure::db::map_sqlx_error(&sqlx::Error)` — the one place that
  knows Postgres codes — after logging with their own context. A row whose `TryFrom` fails is
  logged with the domain error and becomes `InvalidData` (500, not 422).
- The module lives in `domain/shared/`, next to shared value objects, now declared in
  `domain/shared.rs` instead of a generated `shared/mod.rs` (the module convention forbids
  `mod.rs`). `generate value-object` appends its two modules to `shared.rs`.

## Consequences

- An adapter for another store (an HTTP API, Redis) maps onto the same three variants; the
  domain and application layers do not change.
- `RepositoryError::InvalidData` hides the underlying VO error from the caller; the log keeps it.
- Use cases can no longer receive an entity-specific error from a repository. A repository that
  needs a richer failure (an optimistic-lock conflict) adds a variant here, with its `From` arm in
  every entity — the exhaustive match makes the compiler list them.
