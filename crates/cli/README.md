# Puerto

Puerto is a Rust CLI that scaffolds full-stack DDD projects following Clean Architecture (Hexagonal / Ports & Adapters). It generates a 3-crate workspace with auto-wired dependency injection, zero manual wiring.

## Install

```bash
cargo install puerto-framework
```

## Quick start

```bash
puerto new                          # interactive: prompts for name and db support
puerto new --name my-app            # skip name prompt
puerto new --name my-app --db       # fully non-interactive, includes SQLx/Postgres
```

## Commands

```bash
puerto new [--name <name>] [--db]

puerto generate scaffold <Name>            # add a DDD entity (InMemory repository)
puerto generate scaffold <Name> --db       # add a DDD entity (SQLx/Postgres repository)
puerto generate use-case <Entity> <action> # add a use case to an existing entity
puerto generate migration <name>           # create a SQLx migration file
puerto generate bootstrap                  # regenerate bootstrap.rs from puerto.toml
```

## Generated project structure

```
my-app/
├── puerto.toml                    # source of truth for entities + use cases
├── business/                      # domain + application layer (pure Rust)
│   └── src/
│       ├── domain/<entity>/
│       │   ├── model.rs
│       │   ├── errors.rs
│       │   ├── repository.rs      # trait (port) + mockall mock
│       │   └── use_cases/
│       └── application/<entity>/
│           └── <use_case>.rs      # use case impl + unit tests
├── infrastructure/                # adapters (InMemory or SQLx/Postgres)
│   └── src/<entity>/
│       └── repository.rs
└── presentation/                  # poem-openapi REST API
    └── src/
        ├── main.rs                # 5-line entry point — never changes
        ├── generated/
        │   └── bootstrap.rs       # auto-generated DI wiring
        └── api/<entity>/
            ├── routes.rs
            ├── dto.rs
            ├── responses.rs
            └── error_mapper.rs
```

## Architecture

Dependency rule (inward only):

```
Presentation → Infrastructure → Application → Domain
```

- **Domain** — pure Rust, no external dependencies beyond `std` / `async-trait` / `thiserror`
- **Application** — use case implementations, imports domain only
- **Infrastructure** — adapters (InMemory or SQLx); imports domain
- **Presentation** — poem-openapi REST API; imports all layers via auto-generated DI

## Tech stack

- [poem-openapi](https://github.com/poem-web/poem) — REST API + OpenAPI/Swagger
- [sqlx](https://github.com/launchbadge/sqlx) — async Postgres (optional, `--db`)
- [tokio](https://tokio.rs) — async runtime
- [thiserror](https://github.com/dtolnay/thiserror) — domain errors
- [mockall](https://github.com/asomers/mockall) — repository mocks for unit tests

## License

MIT
