# 0022 — References between entities are typed ids, checked by foreign keys

**Status:** Accepted (2026-10-13). Spec: `PLAN.md` § Phase 18. Amends ADR-0014 (its "there is no
relation syntax" consequence) and builds on ADR-0015 (one `RepositoryError`) and ADR-0007 (the
error contract).

## Context

An `Order` that belongs to a `Customer` had to be declared `customer_id:Uuid`: a bare `Uuid` in
the domain (ADR-0014 typed the entity's own id, not the ids it holds), a `UUID` column with no
foreign key, and nothing between the API and the table that says the customer exists. An order
for a customer that never existed was stored, and a `CustomerId` could be passed where a
`ProductId` was meant, which is exactly what ADR-0014 set out to make impossible.

## Decision

- **A reference is an aggregate holding another aggregate's id — never the aggregate.** There is
  no navigation (`order.customer.name`), no lazy loading, no ORM: a use case that needs the
  customer asks the customer's repository. Aggregates stay the unit of consistency and of loading.
- **Syntax.** `customer_id:ref:Customer`, nullable `reviewer_id:ref:opt:Customer`. In `puerto.toml`
  the field is `type = "Uuid"` (or `"Option<Uuid>"`) with `references = "Customer"`: the registry
  type stays what the row and the wire hold, `references` says whose id it is.
- **The domain holds the target's newtype.** `pub customer_id: CustomerId`, imported from
  `crate::domain::customer::model`. Params, the repository port, the Object Mother and the tests
  use it; the edges (`{Entity}Db`, DTOs, the events snapshot) hold a `Uuid`, as with the entity's
  own id (ADR-0014).
- **The database checks that the target exists.** A relational database is there to keep this
  kind of invariant: the column is `UUID NOT NULL REFERENCES customers (id)` under a named
  constraint (`orders_customer_id_fkey`), with an index on it. The use case does not look the
  customer up first: that would couple one aggregate's write to another's repository, cost a
  query, and still race with a concurrent delete. Postgres rejects the write atomically.
- **The rejection is a validation error, 422.** `puerto-core` gains
  `RepositoryError::MissingReference(&'static str)`, the field the store could not resolve (it
  stays `Copy`). `puerto-sqlx` maps SQLSTATE `23503` to it through the entity's own table of
  constraint names (`map_sqlx_error_with(err, REFERENCES)`); an unknown constraint is
  `Unavailable`, as before. The entity's `From<RepositoryError>` turns it into
  `ValidationError("customer_id_not_found")` — the code `order.validation_error.customer_id_not_found`,
  which the error mapper already answers with 422 and the web form shows under the field. No new
  error variant per field: the error enum is the same whatever the entity references.
- **Soft deletes do not cascade and are not refused.** Rows are never removed (`deleted = true`),
  so the foreign key keeps pointing at a row that exists: an order keeps its deleted customer.
  Refusing the delete (409) or cascading through events are behaviour a project decides; neither is
  generated.
- **The target must exist before the reference.** `scaffold` refuses a `ref:` to an entity that is
  not a CRUD entity with code on disk: its `create_<snake>_table` migration then has an earlier
  version, so the `REFERENCES` resolves when migrations run in order.
- **Projects without `--db` accept references too** — the typed id is worth having — but nothing
  checks that the target exists: the in-memory repository is a placeholder, not a store.

## Alternatives considered

- **Check existence in the use case** through a port of the target. Works in memory too, but is
  a second source of truth next to the foreign key, is racy, and makes every write of `Order`
  depend on `Customer`'s repository.
- **A bare `Uuid` with a foreign key.** Keeps the database honest but not the code: ids of two
  entities stay interchangeable.
- **One error variant per reference** (`OrderError::CustomerNotFound`). Readable, but the error
  enum, its mapper and their tests would change with every reference added, and two references to
  the same entity would need field names anyway.
- **`ON DELETE CASCADE` / `RESTRICT`.** Meaningless under soft deletes: the row is never deleted.

## Consequences

- Seeding a row in a Pg integration test needs its targets: the generated tests seed them first,
  through the targets' own repository (or unit of work), transitively.
- A reference to the same entity (`parent_id:ref:opt:Category`), many-to-many (`vec:ref:`, a join
  table), expanding a reference in a response, and an owner filled from the caller's `Principal`
  are not part of this decision — each is a later spec.
- A filter by reference on the list (`GET /orders?customerId=`) is generated: it is the one query
  every reference is wanted for, and its `WHERE` is the indexed column.
