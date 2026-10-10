# 0001 — Enum value objects implement `FromStr`

**Status:** Accepted (2026-10-10)

## Context

Enum value objects (`status:Status:enum:Active/Inactive`) were generated with an inherent
`pub fn from_str(s: &str) -> Result<Self, EntityError>`. Clippy's `should_implement_trait` rejects
an inherent method with the name and shape of a standard trait method, so every project with an
enum VO failed its own `make lint` (`-D warnings`).

## Decision

Generate `impl std::str::FromStr for Status { type Err = EntityError; … }` and construct with
`value.parse::<Status>()` everywhere (application, infrastructure `TryFrom`, tests). `as_str()`
stays inherent — `Display` would invite formatting a VO into user-facing text.

## Consequences

- `.parse::<Vo>()` needs no import, so generated call sites stay import-free.
- Hand-written code calling `Status::from_str(x)` needs `use std::str::FromStr;`.
