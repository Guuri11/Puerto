# Infrastructure Layer — AI Agent Notes

This crate contains **adapters** — implementations of the ports defined in the domain. No business
logic lives here.

## Layer Structure

```
infrastructure/
├── src/
│   ├── lib.rs               # Module tree, inline (`pub mod <entity> { pub mod repository; }`) — no `mod.rs`
│   ├── db.rs                # `MIGRATOR` + `pub use puerto_sqlx::*` (pool, `ping`, `map_sqlx_error`) — --db projects
│   └── <entity>/
│       ├── repository.rs    # Implements the domain repository trait
│       └── entity.rs        # `<Entity>Db` row + conversions (--db projects)
└── migrations/              # SQLx migrations (--db projects)
```

## Critical Rules

- **No business logic** — repositories only translate between domain objects and storage.
- **`from_repository()` is the only door** from persistence into the domain — never call
  `Entity::new()` inside a repository.
- Repositories depend on `business` only. Never import from `presentation`.
- Logging with `tracing::debug!` / `error!` directly. Repositories return `RepositoryError`
  (`puerto_core::RepositoryError`): log the driver error with context, then convert
  it with `db::map_sqlx_error` (`puerto-sqlx`'s: `23505` unique violation → `Duplicate`, the rest → `Unavailable`);
  a row the domain rejects is `InvalidData`. Never leak a driver error upward.
- An entity with references (`ref:`) writes through `db::map_sqlx_error_with(&e, REFERENCES)`:
  `23503` on one of its foreign keys → `MissingReference("<field>")`. A foreign key added by hand
  goes in that `REFERENCES` table too. Its Pg tests seed the referenced rows first, with the
  target's `integration_tests::seed`.

## In-memory (default without `--db`)

`InMemory<Entity>Repository` keeps rows in a `RwLock<HashMap<Id, Entity>>`, in process memory:
they are lost on restart. It behaves like the Postgres adapter — deleted rows hidden, lists newest
first and paged, a `unique` value held by another live row answers `RepositoryError::Duplicate` —
and its unit tests say so. Replace it with a real adapter when storage is added.

## Postgres (`puerto new --db`)

- `<Entity>Db` (`#[derive(sqlx::FromRow)]`) holds the system columns (`id`, `created_at`,
  `updated_at`, `deleted`, `deleted_at`) plus the custom fields. `TryFrom<EntityDb> for Entity`
  (can fail: value objects are re-validated) and `From<&Entity> for EntityDb`.
- Deletes are soft (`deleted = true`); every query filters `WHERE deleted = false`, and unique
  indexes are partial (`… WHERE deleted = false`).
- `HashMap<String, String>` fields are `JSONB` columns read through `sqlx::types::Json`.
- Queries use `sqlx::query!` / `query_as!`, checked at compile time. After changing a query or a
  migration, run `make sqlx/prepare` and **commit `.sqlx/`**: builds run with `SQLX_OFFLINE=true`.
- Integration tests use `#[sqlx::test]`, which creates a throwaway database per test — the role in
  `DATABASE_URL` needs `CREATEDB`. Seed data must vary `unique` fields between calls.

## Adding a New Entity Adapter

Run `puerto generate repository <Name>`. It reads `[project] db` from `puerto.toml` to choose
InMemory vs Postgres and, for Postgres, writes the migration.
