# 0007 — Error contract: the code is `name`, validation is 422

**Status:** Accepted (2026-10-10)

## Context

Generated error bodies were `{ "name": "product_error", "message": "product.not_found" }` — a
generic `name` with the machine code in `message` — and domain validation answered `400`, the
status poem already uses for a body it cannot parse. Clients could not tell "malformed JSON" from
"title is empty" by status, and branched on a field documented as human-readable. Mappers had no
tests.

## Decision

- `ErrorResponse::from_code(err.to_string())`: `name` is the code (`product.not_found`,
  `product.validation_error.title_empty`). For domain errors, whose `Display` is the code,
  `message` is equal to it.
- Domain validation and invalid value objects → `422 Unprocessable Entity`; not found → `404`;
  duplicate → `409`; repository / unknown → `500`. `400` stays poem's.
- Mappers are exhaustive (no `_` arm) and generated with one unit test per arm.

## Consequences

A new error variant does not compile until it has a status, and a status cannot change without a
failing test. The response enums carry an `UnprocessableEntity` variant instead of `BadRequest`.
