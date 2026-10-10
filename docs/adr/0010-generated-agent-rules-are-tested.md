# 0010 — Generated agent rules have a root entry point and are tested

**Status:** Accepted (2026-10-10)

## Context

Generated projects shipped three per-crate `AGENTS.md` files and nothing at the root. Claude Code
loads `CLAUDE.md`, so for its users those rules were not loaded unless an agent happened to open
them. And they were wrong: they drew module trees with `mod.rs` files — the one thing the module
convention forbids — said DI was wired in `main.rs`, and showed `400` for validation errors.
Nothing tested them, so they drifted from the generator.

## Decision

- Every project ships a root `AGENTS.md` (architecture, `puerto` commands, conventions, the checks
  to run, one commit per piece, risk tiers for human review) and a one-line `CLAUDE.md`
  (`@AGENTS.md`) — the pattern Puerto uses for itself.
- Structural tests assert that no generated `AGENTS.md` describes a `mod.rs` file or the removed
  logger port, and that the root file covers the essentials.
- The template's static `bootstrap.rs` is tested to be byte-identical to what the generator
  writes — it had drifted the same way during the logger removal.

## Consequences

Agent instructions are now part of the generated output and break the build when they drift. They
still say nothing project-specific; that belongs in the project's own docs and ADRs.
