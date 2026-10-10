# Puerto

Puerto is a Rust CLI that scaffolds DDD backends following Clean Architecture (Hexagonal / Ports &
Adapters): a 3-crate workspace with a poem-openapi REST API, dependency injection generated from a
manifest, and tests for every layer. Website and docs: <https://puerto-rs.vercel.app>.

## Install

```bash
cargo install puerto --locked
```

`--locked` builds with the dependency versions Puerto was tested with; without it, cargo may pick
newer transitive dependencies that require a newer Rust than yours.

## Quick start

```bash
puerto new                                  # interactive: name, database, demo entity
puerto new --name my-app --db               # non-interactive, with SQLx/Postgres
cd my-app
puerto generate scaffold Product -- title:String price:i64 sku:String!
make run                                    # http://localhost:8080 — Swagger UI
```

## Commands

```bash
puerto generate scaffold <Name> [--force] -- <fields>   # an entity across every layer
puerto generate use-case <Entity> <action>              # one more use case (snake_case)
puerto generate value-object <Name> <type>              # a shared value object
puerto generate domain|application|repository|presentation <Name>   # one layer at a time
puerto generate migration <name>                        # SQL migration (--db projects)
puerto generate bootstrap                               # rewire DI from puerto.toml
puerto generate snippets [--ide zed|vscode]             # IDE snippets
puerto validate                                         # check puerto.toml against the code
```

Field syntax: `name:String`, `price:i64!` (unique), `note:opt:String`, `tags:vec:String`,
`name:Name:String` (value object), `status:Status:enum:Active/Archived`, `email:Email` (shared VO).

## What every generated project gets

- `business` (domain + application), `infrastructure` (in-memory or Postgres adapters),
  `presentation` (REST API) — dependencies point inward only
- `puerto.toml` as the source of truth; `presentation/src/generated/bootstrap.rs` wired from it
- Unit tests per use case, Object Mothers, `mockall` mocks; Postgres integration tests with `--db`
- `GET /health`, `GET /health/ready` (pings the database), graceful shutdown on SIGTERM
- Error bodies with a machine-readable code in `name` (`product.not_found`); validation is `422`
- Logging through `tracing`; a pinned toolchain (`rust-toolchain.toml`), edition 2024
- `AGENTS.md` + `CLAUDE.md` instructions for coding agents
- A `make lint` (clippy `-D warnings`) that passes out of the box

## License

MIT
