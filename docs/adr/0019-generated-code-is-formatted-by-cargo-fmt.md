# 0019 — Generated code is formatted by `cargo fmt`

**Status:** Accepted (2026-10-12).

## Context

A generated project's `make format` runs `cargo fmt --all -- --check`, and its rules tell agents to
keep it green. Puerto's own output failed it: a fresh `puerto new` had 3–7 diffs, a project with two
scaffolded entities about 200. The templates produce valid Rust, not rustfmt's layout — blank-line
runs left by optional blocks, imports in insertion order, `}).unwrap()`, and lines past 100 columns
that depend on the entity's name, so no template edit can rule them out. The compile matrix ran
clippy and the tests, never `fmt --check`, so nothing caught it.

Two ways to fix it:

1. **Make every template rustfmt-clean.** Line width depends on names the user picks: a template can
   be clean for `Product` and not for `OrderLineItemAdjustment`. Every new template is another place
   to get wrong.
2. **Format after generating.** One step, correct for any name, and it uses the project's own
   toolchain (`rust-toolchain.toml` lists `rustfmt`) and its `rustfmt.toml`, if any — the same
   formatter `make format` checks with.

## Decision

Every command that writes Rust — `new`, `generate scaffold | domain | application | repository |
presentation | use-case | event | consumer | bootstrap` — ends with `cargo fmt --all` in the project
root (`generators::format`). `generate migration`, `snippets`, `value-object` and `web` write no
Rust and do not run it.

- The whole workspace, not the files written: `rustfmt` follows `mod` declarations, so formatting a
  patched `lib.rs` formats its crate anyway. Hand-written code in that crate is formatted too — what
  its own `make format` asks for.
- Failure only warns. Without `rustfmt`, or with a declared module whose file a later layered
  command will write, the code is generated unformatted and the user is told to run
  `make format/fix`.
- The patchers run on formatted code from the second command on (`pub mod domain {}` on one line,
  4-space blocks). The compile matrix formats after every step, like the CLI, so they are tested on
  that input, and checks `cargo fmt --all -- --check` in every scenario.
- The template's static Rust is kept formatted, so `puerto new` changes nothing it does not
  generate. `presentation/src/generated/bootstrap.rs` is the exception: it must equal the
  generator's raw output byte for byte (tested), and `new` formats it.

## Consequences

- The snapshots record formatted output: they changed once, whitespace only (no identifier and no
  file added or removed — checked when they were re-recorded).
- Generating needs `cargo fmt` to be fast; it is (well under a second for a scaffolded project).
- A template's whitespace no longer matters for users. It still matters for readability of the
  templates themselves, and for the structural tests that assert on raw text before formatting.
