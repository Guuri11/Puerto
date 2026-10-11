# 0011 — Templates own the shape of generated code; `types.rs` owns the per-field decisions

**Status:** Accepted (2026-10-11)

## Context

After the move to Tera, the templates still received **pre-rendered Rust** as opaque strings
(`try_from_fields_str`, `validation_tests_str`, `seed_fn`, `all_bindings`, …). The generators were
`format!` factories with a template wrapped around the result: `domain.rs` was 931 lines, and the
same VO decision tree (enum → `Option` → `Vec` → plain VO → primitive) was copied wherever a
literal or conversion was needed — which is how `Option<VO>` / `Vec<VO>` shipped non-compiling
tests (T-01). Adding a field type meant finding every copy.

## Decision

- **Per-field decisions are made once, in `generators/types.rs`**, and handed to templates as a
  serialisable `FieldCtx`: flags (`is_vo`, `is_option_vo`, `required_string`, `string_vo`, …),
  names, and the small *expressions* a decision produces — `props_literal`, `seed_literal`,
  `accessor` (domain → primitive), `from_params` / `from_row` (primitive → domain), the Object
  Mother's storage / parameter / default. Each has a unit test over the field shapes, and
  `types.rs` is under mutation testing.
- **Templates own every statement, struct, test and their layout**, iterating over `fields` with
  `{% for %}` / `{% if %}`. Shapes used by several templates are Tera macros
  (`domain/value_object_macros.tera`, `application/macros.tera`, `presentation/macros.tera`);
  `render` loads all templates into one `Tera` so they can import them.
- Generators are context builders: they choose imports (dedup logic) and pass data — no
  `format!` producing Rust statements outside `types.rs`.
- **Exception:** SQL text with index-dependent `$n` numbering stays a Rust computation
  (`infrastructure.rs`), as does the `CREATE TABLE`.

The refactor (T-10) was done template by template against the generated-output snapshots
(T-18): every step left all of them byte-identical, including two cosmetic quirks of the old
string concatenation that were reproduced on purpose so the refactor changed nothing (the double
blank line before a model's first validation test; VO `let`s in use case impls at column 0).
Fixing those is a separate, deliberate change to generated output.

## Consequences

A new registry type touches `types.rs` (registry entry, and the literal/accessor rules if it
behaves differently) and templates only. Template whitespace is now the fragile part — Tera's
`{%-` / `-%}` trimming decides blank lines — and the snapshots are what catch a mistake there.
