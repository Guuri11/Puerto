# Harbor — AI Agent Instructions

Harbor is a Rust CLI tool that scaffolds full-stack DDD projects. It uses `cargo-generate` to create 3-crate workspaces following Clean Architecture (Hexagonal / Ports & Adapters).

## Tech Stack

- Rust (edition 2024) · cargo-generate · poem-openapi · tokio
- mockall (testing) · thiserror (errors) · async-trait

## Harbor's Own Structure

```
crates/
  cli/              — The harbor binary
    src/main.rs     — CLI definition, GenerateArgs setup, test suite
    src/scaffold.rs — File writer, lib.rs patcher, bootstrap generator
    src/harbor_toml.rs — harbor.toml serde structs + read/write/add_entity
  template/
    basic/          — The "basic" DDD template
      Cargo.toml          — Workspace manifest (workspace only, no [package])
      cargo-generate.toml — Template config (controls which files get substitution)
      harbor.toml         — Source of truth for entities ({{project-name}} substituted)
      business/           — Domain + Application crate
      infrastructure/     — Adapters crate
      presentation/       — REST API crate
        src/
          main.rs.liquid        — Minimal entry point (never changes after harbor new)
          generated.rs          — pub mod bootstrap; (static, never changes)
          generated/
            bootstrap.rs        — AUTO-GENERATED, full DI wiring
```

## harbor.toml — Source of Truth

Every generated project has a `harbor.toml` at the root:

```toml
[project]
name = "my-app"

[[entity]]
name = "Greeting"           # PascalCase
use_cases = ["get_greeting"] # snake_case action names

[[entity]]
name = "Product"
use_cases = ["create_product"]
```

**Rules:**
- `harbor generate scaffold <Name>` appends a new `[[entity]]` block automatically
- `presentation/src/generated/bootstrap.rs` is regenerated from harbor.toml on every scaffold run
- **Never hand-edit `generated/bootstrap.rs`** — run `harbor generate bootstrap` instead
- All derived identifiers come from `name` (PascalCase) and `use_cases` entries (snake_case)

See `.claude/rules/harbor-toml.md` for the full derivation table.

## Generated Project Architecture

Every generated project is a Cargo workspace. Dependency rule (inward only):

```
Presentation → Infrastructure → Application → Domain
```

- **Domain** — pure Rust, no external crates beyond std/async-trait/thiserror
- **Application** — implements use case traits; imports domain only
- **Infrastructure** — adapters (DB, HTTP); imports domain + application
- **Presentation** — poem-openapi REST API; imports all layers

## Critical Rules

- **Template variables**: use `{{project-name}}` in `.toml` files (regex substitution), use `{{crate_name}}` in `.liquid` files (Liquid templating). Never mix them.
- **cargo-generate.toml include list**: must list every file that needs variable substitution. Files NOT listed are copied verbatim.
- **`no_workspace: true`** must be set in `GenerateArgs` — prevents cargo-generate from trying to add the generated project as a member of Harbor's own workspace.
- **Template `.liquid` files**: cargo-generate strips the `.liquid` extension and processes Liquid syntax. Use for `main.rs.liquid` → `main.rs`.
- **Never add `[package]` to the template root `Cargo.toml`** — it is a workspace manifest only.
- **`generated/bootstrap.rs` is auto-generated** — never edit manually, never commit hand-edits to it in generated projects.

## CLI Commands

```bash
harbor new                                 # Scaffold a new project
harbor generate scaffold <Name>            # Add a new DDD entity (all layers)
harbor generate use-case <Entity> <action> # Add a use case to an existing entity
harbor generate bootstrap                  # Regenerate bootstrap.rs from harbor.toml
```

## Make Commands

```bash
make run        # Run harbor CLI (prompts for project name)
make test       # Fast structural tests (verifies generated file structure)
make test/full  # Slow test: generates a real project, compiles it, runs its internal tests
make lint       # cargo clippy --workspace -- -D warnings
make check      # cargo check --workspace
make format     # cargo fmt --all
```

## Completion Checklist

- [ ] `make test` passes (structural tests — fast)
- [ ] `make test/full` passes (generated project compiles + internal tests pass — slow, ~20s)
- [ ] `make lint` clean (zero warnings)
- [ ] `make format` run

## Detailed Rules

- `.claude/rules/architecture.md` — Layer rules, naming conventions, error patterns
- `.claude/rules/harbor-toml.md` — harbor.toml schema + identifier derivation table
- `.claude/rules/testing.md` — TDD cycle, test naming, AAA pattern, DO/DON'T
- `.claude/rules/testing-conventions.md` — Object Mother pattern, mock setup, naming table
- `.claude/rules/workflow.md` — Harbor development workflow (Spec-Driven) + generated project TDD workflow
