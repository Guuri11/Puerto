# 0017 — Runtime crates next to the generator

**Status:** Accepted (2026-10-12). Spec: `PLAN.md` § Phase 12.

## Context

Puerto is a generator only: a generated project depends on no `puerto` crate. Everything it gets is
copied into it — including code that is identical in every project and that nobody is meant to
edit:

| Copied today | Lines | Varies per project |
|---|---|---|
| `presentation/src/main.rs` — tracing, server, graceful shutdown, port, banner | 97 | name only |
| `presentation/src/health.rs` — `/health`, `/health/ready` (ADR-0008) | 48 | no |
| `presentation/src/api/error.rs` — `ErrorResponse`, `IntoErrorResponse` (ADR-0007) | 31 | no |
| `business/src/domain/shared/pagination.rs` — `PageRequest`, `Page` (ADR-0016) | 107 | no |
| `business/src/domain/shared/repository_error.rs` (ADR-0015) | 17 | no |
| `infrastructure/src/db.rs` — pool, `ping`, `map_sqlx_error` | — | no |
| CORS and `OpenApiService` setup inside `bootstrap.rs` | — | no |

Three costs follow, and they grow with the roadmap:

1. **A fix never reaches an existing project.** A wrong Postgres code in `map_sqlx_error` stays
   wrong in every project generated before the fix. Rails and Loco ship the same fix through
   `bundle update` / `cargo update`.
2. **The missing production pieces are too large to generate.** Auth, background jobs, mailers,
   storage, typed config, security middleware: thousands of lines each. Generated, every project
   owns and maintains a private fork of them.
3. **The generator pays for code that is not generated.** Every template must stay green in the
   compile matrix and the snapshots; code in a crate is tested once, with ordinary tests.

There are no generated projects in use yet, so moving code now breaks nobody.

### How other frameworks split it

- **Loco** (closest: Rust, Rails-inspired) — one runtime crate, `loco-rs`, with cargo features
  (`auth`, `worker`, `worker_redis`…), plus a generator. The app implements a `Hooks` trait
  (`routes`, `boot`, `connect_workers`, `after_routes`, `initializers`, `seed`, `on_shutdown`…):
  the framework calls the app. Every minor has broken the app-facing API (the `boot` signature in
  0.14, `init_logger` in 0.16, `AppContext` and `Error` made `#[non_exhaustive]` only in 1.0 "so
  future fields are non-breaking"). Upgrades are a written checklist — since 1.1 one written *for
  a coding agent* — plus `cargo loco doctor`. Generated code (scaffolds) is not upgraded: "this only
  affects code you generate after upgrading".
- **Pavex** (Rust) — a runtime crate (`pavex`: request, response, extractors, `Blueprint`) and a
  transpiler that turns the `Blueprint` into a generated "server SDK" crate: the application
  state, the DI call graph and the router are plain generated Rust, no runtime container. It needs
  nightly `rustdoc --output-format json` to discover signatures.
- **Laravel** — `laravel/laravel` is a thin skeleton, `laravel/framework` is the versioned runtime.
  What the user may want to change is copied on request: `vendor:publish` (config, views),
  `stub:publish` (the generator's templates).
- **Rails** — gems (`railties`, `activerecord`…) for the runtime; generators for app code.
  `bin/rails app:update` replays the new skeleton over the app and asks file by file. Rails 8
  ships authentication as a **generator**, not a gem: the user model and controllers belong to the
  app, the primitives (`has_secure_password`, sessions) stay in the framework.
- **Spring Boot** — starters (dependency bundles) and auto-configuration that **backs away** when
  the app declares its own bean of the same type.

## Decision

### 1. The rule: what is generated, what is a crate

- **Generated** — anything derived from `puerto.toml` or meant to be edited: entities, value
  objects, errors and their `From<RepositoryError>`, use cases, repositories, DTOs, routes, error
  mappers, `bootstrap.rs`, the web slices. This is Puerto's value and does not move.
- **Runtime crate** — anything identical in every project and not meant to be edited.
- **Both, split along that line** — a feature with a domain part (auth: the `User` entity, the
  login use case are generated, as in Rails 8; password hashing, token signing, the middleware are
  in the runtime).

### 2. The runtime follows the Dependency Rule — one crate per layer, not one crate

A single `puerto` crate with features (Loco's shape) would make `business` depend on a crate that
also carries poem and sqlx; the domain's "no external crates" rule could only be kept by feature
discipline nobody checks. The split mirrors the layers instead:

| Crate | Content | Dependencies | Used by |
|---|---|---|---|
| `puerto-core` | `PageRequest`, `Page`, `RepositoryError`; later the **ports** `UnitOfWork`, `Mailer`, `JobQueue`, `Clock`, `FileStore` | `std`, `thiserror`, `async-trait` | `business` |
| `puerto-sqlx` | pool, `ping`, `map_sqlx_error`, migrations at startup; later the `UnitOfWork` adapter | `puerto-core`, `sqlx` | `infrastructure` |
| `puerto-web` | `App`: server, graceful shutdown, health, `ErrorResponse` / `IntoErrorResponse`, typed `Config` per environment, CORS, security headers, body limit, request id, serving `web/dist` | `puerto-core`, `poem`, `poem-openapi`, `tokio` | `presentation` |
| later: `puerto-jobs`, `puerto-mail`, `puerto-auth`, `puerto-storage` | adapters for the `puerto-core` ports | their own | `infrastructure`, `presentation` |

The framework is hexagonal too: `puerto-core` holds ports, the other crates are adapters. A user
replaces an adapter by implementing the port — the Spring "back away", without a container.

### 3. The app calls the runtime; the runtime does not call the app

Loco's `Hooks` trait inverts control: the app's shape is dictated by a trait whose signature
changes between minors, and the call graph is hidden inside the framework. Puerto keeps the
composition root visible, as Pavex does: `bootstrap.rs` stays generated, explicit Rust that
**calls** runtime building blocks. The runtime is a library, not a host:

```rust
// presentation/src/main.rs — generated once
#[tokio::main]
async fn main() -> Result<(), puerto_web::Error> {
    let config = puerto_web::Config::from_env()?;   // fails fast on a missing / invalid value
    puerto_web::App::new(config)
        .serve(generated::bootstrap::build_app)
        .await
}
```

Customisation happens in one hand-written file the generator creates once and never touches
(`presentation/src/app.rs`: extra middleware, an adapter swap), which `bootstrap.rs` calls. No
runtime DI container, no reflection, no proc macro that hides routes: an agent reading the project
can follow every call.

### 4. Versioning: one version for the CLI and every runtime crate

- All runtime crates live in Puerto's workspace (`crates/core`, `crates/web`, `crates/sqlx`) and are
  released **in lockstep** with the CLI: `puerto 0.14.0` generates `puerto-web = "0.14"`.
- `puerto validate` warns when the project's runtime minor differs from the CLI's.
- Every public enum and every struct a user may receive is `#[non_exhaustive]` from day one
  (Loco needed a 1.0 to learn this); constructors or builders instead of struct literals.
- Each breaking release keeps `release.md`'s Migration Guide and also ships it **as a prompt for a
  coding agent** (Loco 1.1's pattern): Puerto is AI-ready, so its upgrade path is too.
- `release.yml` publishes the runtime crates before `puerto`.

### 5. The compile matrix tests the unreleased runtime

Generated projects in `make test/full` / `make test/db` get a `[patch.crates-io]` pointing at the
workspace crates, so a template and the runtime it calls change in the same commit and are tested
together. The runtime crates get ordinary unit tests, and `make test/mutation` covers them.

### 6. Escape hatch: eject

`puerto eject <component>` (later, Laravel's `vendor:publish`) copies a runtime module's source
into the project and rewires the import, for the rare case where implementing the port is not
enough. Ejected code is the user's; it no longer receives fixes.

## Alternatives rejected

- **Stay a generator, add `puerto upgrade`** (a three-way merge of the new template over the
  project, as `rails app:update`). It handles the skeleton, not features: auth or jobs would still
  be thousands of generated lines per project, and every conflict lands on the user. It remains
  useful for the generated files themselves and may come later — it does not replace this.
- **One `puerto` crate with features** (Loco). See § 2: it breaks the domain's dependency rule in
  all but discipline.
- **A Pavex-style transpiler over rustdoc JSON.** Puerto already generates its DI graph, from
  `puerto.toml` rather than from reflection — no nightly toolchain needed. Nothing to adopt beyond
  the principle (generated composition root over a runtime container), kept in § 3.

## Consequences

- Fixes and security patches to infrastructure reach existing projects through `cargo update`.
- The missing production pieces (`PLAN.md`: auth, jobs, mailer, storage, config, security) can be
  built as tested library code instead of templates.
- Generated projects get smaller: the snapshots show exactly which files stop being generated.
- **Two public contracts** instead of one: the generated shape and the runtime crates' API, both
  under `release.md`'s SemVer policy.
- Less is visible in the project: an agent reads the runtime from docs.rs or `~/.cargo`, not from
  the repo. The generated `AGENTS.md` names the runtime crates and what each owns (ADR-0010 tests
  it); eject covers the case where the source must live in the project.
- The crate names (`puerto-core`, `puerto-web`, `puerto-sqlx`, `puerto-jobs`, `puerto-mail`,
  `puerto-auth`, `puerto-storage`) were free on crates.io on 2026-10-12.
- Rollout, one commit each, every one a pure move with no new behaviour:
  1. workspace crates + `puerto-core` (pagination, `RepositoryError`) — ADR-0015/0016 types move,
     their codes do not change;
  2. `puerto-web` (`main.rs`, health, `ErrorResponse`, CORS, `Config`) — ADR-0007/0008 hold;
  3. `puerto-sqlx` (pool, `ping`, `map_sqlx_error`).

  New features land in the runtime from then on.

## Amendment (12.1, 2026-10-12): traits the project implements for its own types stay in it

`IntoErrorResponse` did not move: every `error_mapper.rs` implements it for an error of
`business`, and with the trait in `puerto-web` both trait and type would be foreign to
`presentation` — the orphan rule (E0117) rejects it. Only the body, `ErrorResponse`, is the
runtime's. The rule for later pieces: a trait that one project crate implements for a type of
*another* project crate (here `presentation` for `business`) is generated into the implementing
crate. Runtime traits implemented for a type in the same crate — a repository adapter for a
`puerto-core` port — are fine.
