# 0014 — Entity ids are generated newtypes

**Status:** Accepted (2026-10-11).

## Context

Every entity's id was a bare `Uuid` — in the model, the use case params, `find_by_id` and the
mocks. `get_product.execute(GetProductParams { id: customer.id })` compiled. `document-api`
closes that gap with a `macro_rules! newtype_id` (backlog T-24).

## Decision

- `model.rs` declares `pub struct {Entity}Id(Uuid)` above the entity: `Copy`, `Eq`, `Hash`,
  `generate()` (the only `Uuid::new_v4()`), `from_uuid(Uuid)`, `value() -> Uuid` and `Display` (so
  `%id` in the existing `tracing` calls keeps working). No `Default` and no `new()` — clippy's
  `new_without_default` would ask for one, and a default id has no meaning.
- **Written out by the generator, not a macro.** Puerto already is the macro: a generated struct is
  readable and navigable where it is defined, and a user who wants a different id (ULID, `i64`)
  edits one file. A `macro_rules!` would add a shared module every entity depends on and hide the
  type from "go to definition".
- The newtype stops at the edges. Routes take `Path<Uuid>` and wrap it (`from_uuid`); DTOs and
  `{Entity}Db` rows keep `id: Uuid` (`.value()`), so the OpenAPI contract (`string(uuid)`), the
  SQL column and `web/` are unchanged. `from_uuid` does not validate: any `Uuid` is a well-formed
  id, and whether it exists is the repository's answer.
- Tests that need a missing id call `{Entity}Id::generate()`.

## Consequences

- Mixing two entities' ids is a compile error in the domain, application and infrastructure
  layers.
- An entity with a `Uuid` *field* (`owner_id: Uuid`) still sees a bare `Uuid` for it: references
  to other entities are fields chosen by the user, and typing them is the user's decision (there
  is no relation syntax in `puerto.toml`).
- A `sqlx::Type` / poem `Type` impl for the newtype would remove the edge conversions; it is not
  generated because it would couple the domain crate to sqlx and poem.
