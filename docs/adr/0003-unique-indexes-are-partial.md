# 0003 — Unique indexes are partial

**Status:** Accepted (2026-10-10)

## Context

Generated repositories soft-delete (`deleted = true`) and every query filters
`WHERE deleted = false`. A `unique` field (`sku:String!`) produced a plain
`CREATE UNIQUE INDEX …`, so a deleted row kept its value taken: re-creating a product with the
SKU of a deleted one answered `409 Conflict` for a row no query can see.

## Decision

`CREATE UNIQUE INDEX {table}_{field}_key ON {table} ({field}) WHERE deleted = false;` — a named,
partial index. Named so a later migration can drop it predictably.

## Consequences

Uniqueness holds among live rows only, which is what the API exposes. Restoring a soft-deleted row
can now collide with a live one; a restore use case must handle `Duplicate`.
