# 0004 — `generate use-case` never overwrites and patches `routes.rs`

**Status:** Accepted (2026-10-10) — field type amended by [0012](0012-presentation-holds-use-case-ports.md)

## Context

`puerto generate use-case` wrote its two files unconditionally. Naming an existing action —
including a scaffolded one, `create_product` — replaced a working implementation with a `todo!()`
stub, and re-running the command did the same while printing "added". Separately, `bootstrap.rs`
wires every use case in `puerto.toml` into `{Entity}Api`, but `routes.rs` is rendered from the CRUD
template, so the new use case named a field the struct did not have (E0560).

## Decision

- The action must be snake_case. If it is already in `puerto.toml` or either file exists, the
  command refuses and writes nothing.
- It adds `pub {action}: Arc<…UseCaseImpl>` to `{Entity}Api`, marked `#[allow(dead_code)]` with a
  comment: no endpoint calls it until the user writes one. `scaffold --force` and
  `generate presentation` re-add these fields after rewriting `routes.rs`.

## Consequences

The command is not idempotent any more — by design: a second run is a mistake to report, not a
no-op. Generating an endpoint for a custom use case is left to the user, because its HTTP shape
(method, path, body) cannot be inferred from an action name.
