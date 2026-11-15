# Contexts — AI Agent Notes

One directory per context, four crates each (ADR-0023 of Puerto). A context is one entity of
`puerto.toml`, named after it in snake case: `contextos/product/`.

## Layout

```
contextos/<c>/
├── dominio/          → dominio_<c>       pure Rust: no web framework, database driver or HTTP client
│   └── src/
│       ├── lib.rs                # one `pub mod` per file, written whole — no `mod.rs`
│       ├── modelo.rs             # Entity + Props + its id type + constructors
│       ├── valores.rs            # value objects of this entity, if any
│       ├── errores.rs            # EntityError enum (thiserror, machine-readable codes)
│       ├── repositorio.rs        # Repository trait (port) + `mocks::Mock<Entity>Repository`
│       ├── eventos.rs            # AUTO-GENERATED snapshot + events — `events` projects, never hand-edit
│       ├── unit_of_work.rs       # The write port: begin → save → record → commit — `events` projects
│       └── pruebas/madres.rs     # The Object Mother, behind the `pruebas` feature
├── aplicacion/       → aplicacion_<c>    one file per use case
│   └── src/<action>.rs           # Params + UseCaseTrait + `mocks::Mock<Action>UseCase` + UseCaseImpl + tests
├── infraestructura/  → infra_<c>         the adapters of the domain's ports
│   └── src/
│       ├── repositorio.rs        # InMemory<Entity>Repository, or Pg<Entity>Repository (--db)
│       ├── fila.rs               # `<Entity>Db` row + conversions (--db)
│       ├── unit_of_work.rs       # Pg<Entity>UnitOfWorkFactory (`events` projects)
│       └── pruebas/siembra.rs    # `build` / `seed`: the rows the Postgres tests need (--db)
└── rest/             → rest_<c>          HTTP: poem-openapi
    └── src/
        ├── rutas.rs              # <Entity>Api: #[OpenApi] impl with endpoints + route tests
        ├── dto.rs                # Request/Response structs (#[derive(Object)], camelCase JSON)
        ├── respuestas.rs         # #[derive(ApiResponse)] enums
        └── errores.rs            # `a_respuesta_error(err)`: status + body (+ one test per arm)
```

`contextos/correo/` holds the mails (`puerto generar correo`); in an auth project
`contextos/usuario/` holds the caller as a `User`.

## Dependencies

```
rest_<c> → aplicacion_<c> → dominio_<c> → nucleo, puerto-nucleo
infra_<c> → dominio_<c>
```

Cargo enforces it: a crate only sees what its `Cargo.toml` names. No domain crate depends on an
application crate; only `presentacion/cableado` and the binaries depend on `infra_*`. A reference
(`customer_id:ref:Customer`) makes `dominio_order` depend on `dominio_customer`, owner of
`CustomerId`: references cannot form a cycle, and `puerto validar` names one if they do.

Every crate of the project is declared in the root `Cargo.toml` (`[workspace.dependencies]`) and
named with `{ workspace = true }`. Test helpers — the doubles (`mocks`), the Object Mothers, the
Postgres seeds — sit behind each crate's `pruebas` feature, which other crates enable in their
`[dev-dependencies]`: `dominio_product = { workspace = true, features = ["pruebas"] }`.

## Critical Rules

- **Two constructors**: `new(props)` validates and returns `Result<Self, Error>`.
  `from_repository(data)` bypasses validation — only infrastructure calls it.
- **Typed ids**: an entity's id is its own type, `ProductId` (`modelo.rs`): `ProductId::generate()`
  for a new one, `from_uuid` / `.value()` only at the edges (routes, DTOs, rows).
- **Repositories return `ErrorRepositorio`** (`puerto_nucleo`): `Duplicado`, `DatosInvalidos`,
  `NoDisponible`, `ReferenciaInexistente`. Each entity error has `From<ErrorRepositorio>`, so a use
  case uses `?`. It is `#[non_exhaustive]`: the `From` impl keeps a `_` arm.
- **Paged lists** take a `puerto_nucleo::PeticionPagina` and answer a `puerto_nucleo::Pagina<T>`.
- **A use case is one file** of the application crate: its `Params`, its `UseCaseTrait`, the
  trait's double and the `Impl` with its tests. The routes hold `Arc<dyn …UseCaseTrait>`.
- **Events projects**: a repository port only reads. A write goes through the entity's unit of
  work — `uow_factory.begin()`, `save`, `record(<Entity>Event::…)`, `commit` — and its tests use
  `unit_of_work::mocks::RecordingUnitOfWorkFactory`.
- **Dependencies injected as `Arc<dyn Trait>`** — never concrete types.
- **No `unwrap()`/`expect()`** outside tests (the `pruebas` modules are test code).
- **Logging with `tracing` directly** — no logger port. A use case logs `<use_case>: start` and
  `<use_case>: done`; never the payload.
- **Value objects**: private inner value, `new()` returning `Result`; enum value objects implement
  `std::str::FromStr` (`"Active".parse::<Status>()`) and `as_str()`.
- **Error codes are machine-readable**: `"product.not_found"`,
  `"product.validation_error.name_empty"` — never sentences.
- **Infrastructure has no business logic**: `from_repository()` is the only door from storage into
  the domain. Log the driver error with context, then convert it with
  `puerto_sqlx::mapear_error_sqlx` (`23505` → `Duplicado`); a row the domain rejects is
  `DatosInvalidos`. An entity with references writes through `mapear_error_sqlx_con(&e, REFERENCES)`.
- **REST**: DTOs never expose domain models (`EntityDto::from_domain(&entity)`); error mappers are
  exhaustive and every arm has a test; every `ApiResponse` enum has `from_status`.

## Tests

- Tests live **inside the file they test**, in `#[cfg(test)] mod tests { ... }`, written before
  the implementation. Names: `should_<expectation>_when_<scenario>`, with `// Arrange`, `// Act`,
  `// Assert`.
- A use case test mocks the repository: `dominio_<c>::repositorio::mocks::Mock<Entity>Repository`.
- Route tests drive the real `OpenApiService` through `poem::test::TestClient`, every use case
  mocked (`aplicacion_<c>::<action>::mocks::Mock<Action>UseCase`). One test per status path.
- Postgres tests are `#[sqlx::test(migrator = "migrador::MIGRADOR")]`: a throwaway database per
  test, migrated by the project's one migrator. They seed rows with `pruebas::siembra::seed`, and
  the rows an entity references with the target's (`infra_customer::pruebas::siembra::seed`).
- `cargo test -p dominio_product`, or every domain at once: `make test/domain`.

## Object Mother

`dominio_<c>::pruebas::madres::<Entity>Mother`:

```rust
let widget = WidgetMother::random();
let invalid = WidgetMother::new().with_empty_name().build_props();
let batch = WidgetMother::random_vec(5);
```

Each field gets `.with_<field>(value)`; `String` fields `.with_empty_<field>()` too.

## Adding an entity

`puerto generar scaffold <Name> [-- fields…]` writes the four crates at once. One layer at a time:

```bash
puerto generar dominio <Name>        # dominio_<c> + Object Mother → Next: aplicacion
puerto generar aplicacion <Name>     # aplicacion_<c>              → Next: repositorio
puerto generar repositorio <Name>    # infra_<c>                   → Next: presentacion
puerto generar presentacion <Name>   # rest_<c> + the wiring
```
