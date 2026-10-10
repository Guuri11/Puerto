# 0002 — Response enums allow `large_enum_variant`

**Status:** Accepted (2026-10-10)

## Context

`#[derive(ApiResponse)]` enums carry the whole DTO in their success variant and an `ErrorResponse`
in every other one. As an entity gains fields, clippy's `large_enum_variant` fires and the
generated project fails `make lint`. Clippy's fix is to box the large variant.

## Decision

`responses.rs` starts with `#![allow(clippy::large_enum_variant)]` and a comment saying why. A
response is built once per request and immediately serialized; boxing the DTO adds an allocation
to every successful request to save stack space in a value that lives for one function call.

## Consequences

The lint is silenced for that module only. If a response ever becomes long-lived (stored, sent
through a channel) the allowance should be revisited.
