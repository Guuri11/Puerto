# Puerto — AI Agent Instructions

Puerto is a Rust CLI tool that scaffolds full-stack DDD projects. It uses `cargo-generate` to create 3-crate workspaces following Clean Architecture (Hexagonal / Ports & Adapters).

## Tech Stack

- Rust (edition 2024) · cargo-generate · poem-openapi · tokio
- mockall (testing) · thiserror (errors) · async-trait

## Puerto's Own Structure

```
crates/
  cli/              — The puerto binary
    src/main.rs     — CLI definition, GenerateArgs setup, test suite
    src/scaffold.rs — File writer, lib.rs patcher, bootstrap generator
    src/puerto_toml.rs — puerto.toml serde structs + read/write/add_entity
    template/       — The "basic" DDD template (edit directly here)
      Cargo.toml          — Workspace manifest (workspace only, no [package])
      cargo-generate.toml — Template config (controls which files get substitution)
      puerto.toml         — Source of truth for entities ({{project-name}} substituted)
      business/           — Domain + Application crate
      infrastructure/     — Adapters crate
      presentation/       — REST API crate
        src/
          main.rs.liquid        — Minimal entry point (never changes after puerto new)
          generated.rs          — pub mod bootstrap; (static, never changes)
          generated/
            bootstrap.rs        — AUTO-GENERATED, full DI wiring
```

## puerto.toml — Source of Truth

Every generated project has a `puerto.toml` at the root:

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

- `puerto generate scaffold <Name>` appends a new `[[entity]]` block automatically
- `presentation/src/generated/bootstrap.rs` is regenerated from puerto.toml on every scaffold run
- **Never hand-edit `generated/bootstrap.rs`** — run `puerto generate bootstrap` instead
- All derived identifiers come from `name` (PascalCase) and `use_cases` entries (snake_case)

See `.claude/rules/puerto-toml.md` for the full derivation table.

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
- **`no_workspace: true`** must be set in `GenerateArgs` — prevents cargo-generate from trying to add the generated project as a member of Puerto's own workspace.
- **Template `.liquid` files**: cargo-generate strips the `.liquid` extension and processes Liquid syntax. Use for `main.rs.liquid` → `main.rs`.
- **Never add `[package]` to the template root `Cargo.toml`** — it is a workspace manifest only.
- **`generated/bootstrap.rs` is auto-generated** — never edit manually, never commit hand-edits to it in generated projects.

## CLI Commands

```bash
puerto new                                 # Interactive: prompts for name + db support
puerto new --name <name>                   # Skip name prompt
puerto new --db                            # Skip db prompt (enables SQLx/Postgres)
puerto new --name <name> --db              # Fully non-interactive, with database
puerto new --name <name> --no-db           # Fully non-interactive, no database
puerto new --name <name> --no-db --destination /tmp/projects  # Control output directory

puerto generate scaffold <Name>            # Add a new DDD entity — all layers at once (infers db from puerto.toml)
puerto generate domain <Name>              # Domain layer only: model, errors, repo trait, use cases, Object Mother
puerto generate application <Name>         # Application layer only: use case impls
puerto generate repository <Name>          # Infrastructure layer only: InMemory or Pg repo (inferred from puerto.toml)
puerto generate presentation <Name>        # Presentation layer only: routes, dto, responses + regenerates bootstrap
puerto generate use-case <Entity> <action> # Add a use case to an existing entity
puerto generate migration <name>           # Create a new SQLx migration file
puerto generate bootstrap                  # Regenerate bootstrap.rs from puerto.toml
puerto generate snippets                   # Write IDE snippet files (Zed + VS Code)
puerto generate snippets --ide zed         # Zed only  (.zed/snippets/rust.json)
puerto generate snippets --ide vscode      # VS Code only (.vscode/puerto.code-snippets)
```

## Make Commands

```bash
make run           # Run puerto CLI (prompts for project name)
make test          # Fast structural tests (verifies generated file structure)
make test/full     # Slow test: generates a real project, compiles it, runs its internal tests
make lint          # cargo clippy --workspace -- -D warnings
make check         # cargo check --workspace
make format        # cargo fmt --all
```

## Completion Checklist

- [ ] `make test` passes (structural tests — fast)
- [ ] `make test/full` passes (generated project compiles + internal tests pass — slow, ~20s)
- [ ] `make lint` clean (zero warnings)
- [ ] `make format` run

## Detailed Rules

- `.claude/rules/architecture.md` — Layer rules, naming conventions, error patterns
- `.claude/rules/puerto-toml.md` — puerto.toml schema + identifier derivation table
- `.claude/rules/testing.md` — TDD cycle, test naming, AAA pattern, DO/DON'T
- `.claude/rules/testing-conventions.md` — Object Mother pattern, mock setup, naming table
- `.claude/rules/workflow.md` — Puerto development workflow (Spec-Driven) + generated project TDD workflow
- `.claude/rules/release.md` — Versioning, changelog, release process, breaking changes policy, crates.io checklist
