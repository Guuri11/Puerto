# 0006 — Logging through `tracing`, not a `Logger` port

**Status:** Accepted (2026-10-10). Supersedes the `LoggerTrait` port of Phase 5 (`PLAN.md`).

## Context

Generated projects injected a `LoggerTrait` (`business::domain::logger`) with a `TracingLogger`
adapter. It cost a `logger` field in every use case, repository and API struct, an extra argument
in every repository constructor, a `silent_logger()` mock in every test module and
`Arc::clone(&logger)` throughout `bootstrap.rs`.

`document-api` — a 60k-LOC hand-written backend following the same conventions — made the opposite
choice at scale: `tracing::info!` directly, no logger port. `tracing` is already a facade: the
subscriber is chosen once at startup and can be swapped, filtered or captured in tests without
touching call sites.

## Decision

Remove the port. Application, infrastructure and presentation log with `tracing` macros directly;
`main.rs` installs the subscriber. Use cases log `<use_case>: start` / `done` (with the id when
there is one) and never the request payload. Entity names that would become a module called
`tracing` are rejected — infrastructure declares entity modules at its crate root, where one would
make `tracing::` ambiguous (E0659).

## Consequences

- Less generated code and simpler tests; nothing to mock.
- Log assertions in tests, if ever needed, use a `tracing` test subscriber instead of a mock.
- `business` depends on `tracing`: a facade crate, accepted in the application layer like
  `async-trait` and `thiserror`.
