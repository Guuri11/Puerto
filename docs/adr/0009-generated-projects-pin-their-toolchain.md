# 0009 — Generated projects pin their toolchain and use edition 2024

**Status:** Accepted (2026-10-10)

## Context

Generated projects used edition 2021 with no `rust-toolchain.toml`. Their `make lint` runs clippy
with `-D warnings`, and clippy gains lints between releases — the fixes of 2026-10-10
(`manual_inspect`, `unnecessary_map_or`, `should_implement_trait`) were all version-dependent. So
whether a fresh project passed its own lint depended on whose machine ran it, and Puerto's test
matrix only proved it for the toolchain of whoever ran `make test/full`.

## Decision

Every project ships `rust-toolchain.toml` pinning the version Puerto's matrix lints with
(`1.95.0`, with `rustfmt` and `clippy`), all crates use edition 2024 and the workspace uses
resolver 3.

## Consequences

The matrix now proves lint-cleanliness for exactly the toolchain the project will use. Bumping the
pin is a deliberate change: update the template, run `make test/full`, fix what the new clippy
reports. Edition 2024 already required one such fix — `let` chains in the Pg repositories'
duplicate detection (`collapsible_if`).
