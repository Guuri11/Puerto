# Puerto

Puerto is a Rust CLI that scaffolds DDD backends following Clean Architecture (Hexagonal / Ports &
Adapters): a workspace with one crate per context and layer, a poem-openapi REST API, dependency
injection generated from a manifest, and tests for every layer. What it generates is written in
Spanish (`RepositorioProducto`, `crear_producto`, `/productos`). Website and docs:
<https://puerto-rs.vercel.app>.

## Install

```bash
cargo install puerto --locked
```

`--locked` builds with the dependency versions Puerto was tested with; without it, cargo may pick
newer transitive dependencies that require a newer Rust than yours.

## Quick start

```bash
puerto nuevo                                  # interactive: name, database, demo entity
puerto nuevo --nombre mi-app --db             # non-interactive, with SQLx/Postgres
cd mi-app
puerto generar scaffold Producto -- titulo:String precio:i64 sku:String!
make run                                      # http://localhost:8080 — Swagger UI
```

## Commands

```bash
puerto generar scaffold <Nombre> [--forzar] -- <campos>    # an entity across every layer
puerto generar caso-de-uso <Entidad> <accion>              # one more use case (snake_case)
puerto generar objeto-valor <Nombre> <tipo>                # a shared value object
puerto generar dominio|aplicacion|repositorio|presentacion <Nombre>   # one layer at a time
puerto generar migracion <nombre>                          # SQL migration (--db projects)
puerto generar bootstrap                                   # rewire DI from puerto.toml
puerto generar snippets [--ide zed|vscode]                 # IDE snippets
puerto validar                                             # check puerto.toml against the code
```

Field syntax: `nombre:String`, `precio:i64!` (unique), `nota:opt:String`, `etiquetas:vec:String`,
`nombre:Nombre:String` (value object), `estado:Estado:enum:Activo/Archivado`, `email:Email`
(shared VO), `cliente_id:ref:Cliente` (a reference).

## What every generated project gets

- One crate per context and layer: `contextos/<c>/{dominio,aplicacion,infraestructura,rest}`
  (`dominio_<c>`, `aplicacion_<c>`, `infra_<c>`, `rest_<c>`), `nucleo` for shared value objects,
  `presentacion/{cableado,api}` — Cargo enforces that dependencies point inward only
- `puerto.toml` as the source of truth; `presentacion/cableado` and
  `presentacion/api/src/generado/bootstrap.rs` wired from it
- Unit tests per use case, Object Mothers, `mockall` doubles; Postgres integration tests with `--db`
- `GET /health`, `GET /health/ready` (pings the database), graceful shutdown on SIGTERM
- Error bodies `{ codigo, mensaje }` with a machine-readable code (`producto.no_encontrado`);
  validation is `422`
- Logging through `tracing`; a pinned toolchain (`rust-toolchain.toml`), edition 2024
- `AGENTS.md` + `CLAUDE.md` instructions for coding agents
- A `make lint` (clippy `-D warnings`) that passes out of the box

## License

MIT
