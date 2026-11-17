<p align="center">
  <img src="assets/brand/github-social.svg" alt="Puerto — Scaffold. Structure. Ship." width="100%"/>
</p>

<p align="center">
  <a href="https://puerto-rs.vercel.app/docs/"><img src="https://img.shields.io/badge/docs-puerto--rs.vercel.app-F5B700?labelColor=15171A" alt="Documentation"/></a>
  <a href="https://crates.io/crates/puerto"><img src="https://img.shields.io/crates/v/puerto?color=15171A&labelColor=5A574F" alt="crates.io"/></a>
  <img src="https://img.shields.io/badge/rust-2024-15171A?logo=rust&logoColor=white&labelColor=5A574F" alt="Rust 2024"/>
  <img src="https://img.shields.io/badge/architecture-DDD%20%2B%20Clean%20Arch-15171A?labelColor=5A574F" alt="DDD + Clean Architecture"/>
  <img src="https://img.shields.io/badge/license-MIT-15171A?labelColor=5A574F" alt="MIT license"/>
</p>

<p align="center">
  <b><a href="https://puerto-rs.vercel.app">Website</a></b> ·
  <b><a href="https://puerto-rs.vercel.app/docs/start/quick-start/">Quick start</a></b> ·
  <b><a href="https://puerto-rs.vercel.app/docs/">Documentation</a></b> ·
  <b><a href="https://puerto-rs.vercel.app/docs/decisions/">Decisions</a></b>
</p>

---

A Rust full-stack framework that brings the **delightful developer experience** of Laravel and Ruby on Rails to a **Domain-Driven Design** architecture.

---

## Core Principles

### 1. Delightful Coding Experience

Getting started should take seconds, not hours. `puerto nuevo --nombre mi-app` gives you a production-ready workspace with the right structure, dependencies, and conventions already in place. No boilerplate, no decision fatigue.

### 2. Truly AI-Ready

Puerto is built for the age of AI-assisted development. Rust's strict compiler acts as an **instant feedback loop** — when an AI agent generates wrong code, the compiler catches it with a precise, actionable error. No silent failures, no runtime surprises. The stronger the types, the smarter the agent.

### 3. Convention over Configuration

Puerto makes the right choices for you. Directory layout, error patterns, dependency injection, testing conventions — all standardized. When every Puerto project looks the same, AI agents and human developers can navigate any codebase from day one.

---

## Why DDD Instead of MVC

MVC is a great starting point, but it doesn't scale well. As applications grow, business logic leaks into controllers and models, making code harder to test and reason about.

Puerto generates projects around **Domain-Driven Design** and **Clean Architecture**:

- **Domain** is pure Rust — no framework dependencies, no infrastructure concerns. Business rules live here.
- **Application** orchestrates the domain — use cases are explicit, testable units of behavior.
- **Infrastructure** adapts external systems (databases, HTTP clients) to domain contracts.
- **Presentation** exposes the application via HTTP — it's just another adapter.

The result: a codebase where business logic is isolated, always testable, and never coupled to the framework.

---

## Architecture

Every Puerto project is a Cargo workspace with one crate per context and layer
([ADR-0023](docs/adr/0023-crates-por-contexto-y-capa.md)), plus a React frontend (`--sin-web`
skips it). What Puerto generates is written in Spanish — types, functions, routes, JSON, error
codes, tests ([ADR-0024](docs/adr/0024-lo-generado-en-castellano.md)); every name comes from
[the glossary](docs/glosario.md).

```
mi-app/
├── nucleo/                 # Shared value objects — no I/O
├── contextos/
│   └── producto/           # One context per entity
│       ├── dominio/        # dominio_producto — model, errors, repository port, events
│       ├── aplicacion/     # aplicacion_producto — one file per use case, with its tests
│       ├── infraestructura/ # infra_producto — in-memory or Postgres adapters
│       └── rest/           # rest_producto — poem-openapi routes, DTOs, error mapping
├── presentacion/
│   ├── cableado/           # The composition root — generated from puerto.toml
│   └── api/                # The binary (+ worker/ with domain events)
├── migraciones/ + migrador/ # Every SQL migration and the one MIGRADOR (--db)
└── web/                    # React 19 + daisyUI on Bun — typed from the API's OpenAPI spec
```

**Dependency rule (inward only), enforced by Cargo:**

```
rest_<c> → aplicacion_<c> → dominio_<c> → nucleo, puerto-nucleo
infra_<c> → dominio_<c>
cableado, api, worker → everything
```

The domain depends on nothing in the workspace. A reference between entities
(`cliente_id:ref:Cliente`) is a dependency between their domain crates, so Cargo refuses a cycle.
`web/` follows the same rule, checked by dependency-cruiser.

Every generated entity also gets, out of the box:

- **Its own id type** (`IdProducto`), so another entity's id cannot be passed by mistake
- **One repository error** (`ErrorRepositorio`: `Duplicado`, `DatosInvalidos`, `NoDisponible`,
  `ReferenciaInexistente`) that its own error converts — a unique violation in Postgres answers
  `409`
- **Paged lists** — `GET /productos?limite=&desplazamiento=` answers
  `{ elementos, total, limite, desplazamiento }`, 20 per page by default and capped at 100 in the
  domain

---

## Getting Started

```bash
# Install Puerto (--locked: build with the tested dependency versions)
cargo install puerto --locked

# Scaffold a new project
puerto nuevo --nombre mi-app

# Enter the project and run
cd mi-app
make run
```

Visit `http://localhost:8080` for the Swagger UI, or call the API directly:

```bash
curl http://localhost:8080/api/saludos/Mundo
```

### Web frontend

`puerto generar scaffold <Nombre>` also writes the entity's screens — list, detail and a
create/edit form: containers that call the typed API client, views that only take props, and
`bun test` tests against a fake API — and every field type gets its input. A rejected form says
what went wrong next to the field (`contextos/<entidad>/mensajes.ts`, from
the same codes as `errores.rs`), and list pages live in the URL. The OpenAPI spec is the contract
between the two halves:

```bash
make web/sync                    # after a scaffold or a DTO change: export the spec, regenerate the web types
make web/install && make web/dev # http://localhost:5173, /api proxied to :8080
```

`puerto generar web <Nombre>` writes the screens alone, for an entity already in `puerto.toml`.
The frontend keeps its English internals for now; it follows the API's Spanish contract.

### Entity Fields

Scaffold entities with typed fields — the type system flows from `puerto.toml` through all layers:

```bash
puerto generar scaffold Producto -- nombre:String precio:i64 sku:String!
```

This creates a `Producto` entity with custom fields in `puerto.toml`, and generates typed structs in every layer (domain model, DTOs, repository rows, SQL migrations). Supported types include `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`, `Option<T>`, `Vec<T>`, and `HashMap<String, String>`. Append `!` to mark a field as unique (e.g., `sku:String!`); `--unico jugador_id,partido_id` makes a unique over several fields, and `--orden=-puntos,nombre` sets the order of the list.

### Value Objects

Wrap primitive fields in strongly-typed Value Objects — generated across all layers automatically:

```bash
# Regular VO
puerto generar scaffold Pedido -- importe:Importe:f64 estado:Estado:enum:Pendiente/Confirmado/Cancelado

# Nullable / array VOs
puerto generar scaffold Cliente -- email:Email:String apodo:Apodo:opt:String etiquetas:Etiqueta:vec:String

# Shared VOs — declared once, reusable across entities
puerto generar objeto-valor Email String
puerto generar scaffold Cliente -- email:Email   # type inferred from the shared VO declaration
```

Each VO generates a domain struct with private inner value, `new()` returning `Result`, and a `valor()` / `as_str()` accessor. Enum VOs implement `FromStr` (`"Activo".parse::<Estado>()`) and `as_str()`. Infrastructure and REST use primitives; VOs are constructed and extracted at the boundary automatically.

### References

An entity that belongs to another holds its id — never the aggregate
([ADR-0022](docs/adr/0022-references-are-typed-ids-checked-by-foreign-keys.md)):

```bash
puerto generar scaffold Cliente -- nombre:String
puerto generar scaffold Pedido -- cliente_id:ref:Cliente revisor_id:ref:opt:Cliente
```

`Pedido.cliente_id` is an `IdCliente`, so ids of two entities cannot be swapped. With `--db` the
column has a foreign key: an order for a missing customer is a `422` on `cliente_id`. The list
filters by it (`GET /pedidos?clienteId=`), and the web form picks the customer from a `<select>`.

### Domain events

A project with a database can record an event with every write — saved with the entity in one
transaction, in an outbox table ([ADR-0018](docs/adr/0018-domain-events-outbox-pluggable-transport.md)):

```bash
puerto nuevo --nombre tienda --db --eventos db   # or --eventos kafka
puerto generar scaffold Producto -- nombre:String sku:String!
puerto generar evento Producto producto_publicado
puerto generar consumidor avisar_producto_nuevo --tras producto_creado,producto_publicado
make worker                                      # relays the outbox, runs the consumers (:8081)
```

Each CRUD entity gets a generated `eventos.rs` (`EventoProducto::Creado(SnapshotProducto)`, …) and
a unit of work; its repository only reads, so a write that skips the outbox does not compile. A
second binary, `presentacion/worker/`, relays the outbox and runs the consumers: each is a
`ManejadorEvento`, built once with the database pool (`construir(pool)`, where it takes the use
cases it calls), that answers `Ack`, `Reintentar` (redelivered with backoff) or `Descartar` —
nothing is dropped without a row in `descartados` (`make eventos/descartados`). Delivery is
at-least-once, so handlers must be idempotent. The `db` transport needs no broker. On Kafka
(`--eventos kafka`) the worker publishes each event to its aggregate's topic, keyed by the
aggregate id, and each consumer is a consumer group — `docker-compose.yml` runs Redpanda for
development. Building a Kafka project compiles librdkafka from source: it needs `cmake` and the
libcurl headers (`libcurl4-openssl-dev` on Debian/Ubuntu, `curl` on Arch, preinstalled with Xcode
on macOS).

### Mail

A mail is a port in the domain, a template implementation in infrastructure and three template
files — written once, then yours ([ADR-0020](docs/adr/0020-mail-port-per-mail-sent-from-the-worker.md)):

```bash
puerto generar correo bienvenida -- nombre:String plan:opt:String       # any project
puerto generar correo bienvenida --tras cliente_creado --para email     # events projects: the worker sends it
make correo/ui                                                          # Mailpit: every mail sent in development
```

`contextos/correo/dominio/src/bienvenida.rs` holds `CorreoBienvenida` and the port
`EnviadorCorreoBienvenida`; `contextos/correo/infraestructura/src/bienvenida.rs` renders
`contextos/correo/infraestructura/plantillas/bienvenida/{asunto.txt, cuerpo.html, cuerpo.txt}`
(minijinja, strict) and sends through SMTP — its tests render every template, so a typo fails
`make test`. With `--tras`, `presentacion/worker/src/consumidores/correo_bienvenida.rs` builds the
mail from the event's snapshot and sends it once per event: a `correos_enviados` row per event
keeps a redelivery from sending it again, a 5xx is discarded, a 4xx or a server that is down is
retried. In development Mailpit (`docker-compose.yml`) catches every mail; production sets
`SMTP_URL`.

### Auth

`puerto nuevo --auth oidc` verifies an OpenID Connect provider's tokens (Auth0, Keycloak — Keycloak
runs in `docker-compose.yml` for development); `--auth local` (with `--db`) keeps accounts and
sessions in the project's database, with `POST /auth/registro`, `/auth/login` and `/auth/logout`
([ADR-0021](docs/adr/0021-auth-one-authenticator-port-oidc-or-local.md)). Routes need a token by
default; `scaffold --publico listar_productos` opens one, `--rol borrar_producto=admin` adds
`#[requiere_rol("admin")]` to its handler, and `--privado email` keeps a field out of what a public
route answers a caller without a session.

Validate your `puerto.toml` at any time:

```bash
puerto validar
```

---

## Project Structure (Puerto itself)

Puerto is also a Cargo workspace:

```
puerto/
└── crates/
    ├── cli/                # The puerto binary (cargo-generate wrapper)
    │   ├── src/            # CLI source
    │   └── template/       # The project template (edit directly here)
    ├── nucleo/             # puerto-nucleo — runtime: ErrorRepositorio, PeticionPagina / Pagina<T>,
    │                       #   domain events (envelope, delivery policy, transport ports), the
    │                       #   mail ports and the auth port
    ├── macros/             # puerto-macros — #[requiere_rol], reached through puerto-web
    ├── web/                # puerto-web — runtime: server, Config, health probes, RespuestaError,
    │                       #   BearerAuth
    ├── sqlx/               # puerto-sqlx — runtime: Postgres pool, migrations, ping, mapear_error_sqlx,
    │                       #   unit of work, outbox, the db event transport, sessions and credentials
    ├── eventos/            # puerto-eventos — runtime: the worker's relay, consume loops, health
    │                       #   probes and graceful shutdown
    ├── kafka/              # puerto-kafka — runtime: the Kafka event transport
    ├── correo/             # puerto-correo — runtime: SMTP sender, templates, send-once
    └── auth/               # puerto-auth — runtime: the OIDC authenticator, argon2 password hashing
```

What every generated project shares is not copied into it: it depends on the runtime crates, on the
same minor as the CLI that generated it, and receives their fixes through `cargo update`
([ADR-0017](docs/adr/0017-runtime-crates-next-to-the-generator.md)).

| Command          | Description                                                |
| ---------------- | ---------------------------------------------------------- |
| `make run`       | Run the puerto CLI                                         |
| `make test`      | Fast structural tests                                      |
| `make test/full` | Full test: generates projects + runs their clippy and tests     |
| `make test/db`   | Same for a `--db` project, against a throwaway Postgres + Mailpit |
| `make test/kafka` | The Kafka transport against a throwaway Redpanda (+ Postgres, Mailpit) |
| `make test/mail` | The SMTP adapter against a throwaway Mailpit               |
| `make test/auth` | The OIDC adapter and an OIDC project against a throwaway Keycloak + Postgres |
| `make test/mutation` | Mutation testing (cargo-mutants) on Puerto's pure logic |
| `make lint`      | Clippy with `-D warnings`                                  |
| `make format`    | rustfmt                                                    |
| `make check`     | cargo check                                                |
