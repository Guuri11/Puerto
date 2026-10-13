## [0.19.0] - 2026-10-13

Auth (PLAN Phase 16, [ADR-0021](docs/adr/0021-auth-one-authenticator-port-oidc-or-local.md)): one
`Authenticator` port, answered by an OpenID Connect provider (Auth0, Keycloak) or by accounts and
sessions in the project's own database. A project without `--auth` is generated exactly as in 0.18.

### 🚀 Features

- **`puerto new --auth oidc`**: the API verifies the provider's bearer tokens (RS256/ES256, `iss`,
  `aud`, `exp`/`nbf`, JWKS refetched on an unknown `kid` at most once a minute). Written once and
  yours: the `User` entity (in memory, or `PgUserRepository` with a `users` migration in `--db`),
  `ProvisioningAuthenticator` (a caller becomes a `User` on their first request) and `GET /me`.
  Keycloak 26.8 in `docker-compose.yml` with a realm of its own (client `web`, PKCE; a dev-only
  password-grant client `dev-cli`; `admin@<project>.localhost` with `admin`, `user@` without),
  `AUTH_*` in `.env`, `make auth/ui`, `make -s auth/token [EMAIL=…]`, an Auth section in `AGENTS.md`
- **`puerto new --db --auth local`**: accounts in Postgres — `POST /auth/register`, `/auth/login`
  (an opaque session token; the row keeps its SHA-256), `/auth/logout`; argon2id passwords; an
  unknown email and a wrong password get the same 401 in the same time. `AUTH_SESSION_TTL`,
  `make auth/grant EMAIL=… ROLE=…`
- **Protected scaffolds**: in an auth project every CRUD handler takes `auth: BearerAuth`
  (401 `auth.unauthenticated`, 503 `auth.unavailable`) except `--public <uc>,…`;
  `--role <uc>=<role>` adds `#[require_role("<role>")]` (403 `auth.forbidden`; two roles: both
  required). Protected params carry `actor: Principal`; route tests sign in through a
  `MockAuthenticator` and add a 401 per protected route and a 403 per role. `generate use-case`
  protects the new use case too
- **The frontend signs in**: OIDC through `oidc-client-ts` (token in memory, `/auth/callback`),
  local through `/login` and `/register` (token in `localStorage`). `AuthProvider` / `useAuth` /
  `RequireAuth`, a bearer middleware, a user menu; generated routes wrap protected screens in
  `<RequireAuth>` and screens hide the actions a role guards (`can(role)`)
- `puerto.toml` gains `[project] auth = "oidc" | "local"` and `[[entity]] public` / `roles`;
  `puerto validate` checks them and that `routes.rs` still says the same (`BearerAuth`,
  `require_role`)
- New runtime crates **`puerto-macros`** (`#[require_role]`, reached as `puerto_web::require_role`)
  and **`puerto-auth`** (`OidcAuthenticator`, `Argon2Hasher`). `puerto-core::auth` (`Principal`,
  `Authenticator`, the local ports and their mocks), `puerto-web::auth` (`BearerAuth`,
  `forbidden()`, `bearer_token`), `puerto-sqlx::auth` (`PgSessions`, `PgCredentials`, the `users`
  and `auth` migrations)
- `make test/auth` runs the OIDC adapter and a generated OIDC project's API against Keycloak +
  Postgres; `make test/db` runs a local-auth project's API end to end

### 🐛 Bug Fixes

- `puerto new --auth oidc --no-demo` compiles: auth runs after the demo strip

### Migration Guide

Nothing to migrate: 0.19 changes no project generated without `--auth`. Bump the runtime crates to
`0.19`. In an auth project, `User`, `Me` and `Auth` are reserved entity names.

### Release

- Publish order: `puerto-core` → `puerto-macros` → `puerto-web` → `puerto-sqlx` →
  `puerto-events` → `puerto-kafka` → `puerto-mail` → `puerto-auth` → `puerto`

## [0.18.0] - 2026-10-12

Mail (PLAN Phase 14, [ADR-0020](docs/adr/0020-mail-port-per-mail-sent-from-the-worker.md)): a
port per mail, its templates in infrastructure, sent by the worker once per event.

### 🚀 Features

- **`puerto generate mail <name> [-- field:Type …]`** (any project) writes, once and yours from
  then on: `business/src/domain/mail/<name>.rs` (`{Name}Mail` + the port `{Name}Mailer` + its
  mock), `infrastructure/src/mail/<name>.rs` (`Template{Name}Mailer`, whose tests render every
  template — a typo fails `make test`) and `infrastructure/mail/<name>/{subject.txt, body.html,
  body.txt}` (minijinja, strict). The first mail adds `puerto-mail`, Mailpit to
  `docker-compose.yml` (created in a project without `--db`), `SMTP_URL` / `MAIL_FROM` in `.env`,
  `make mail/ui` and a Mail section in `AGENTS.md`
- **`--on <event_type> --to <field>`** (events projects): the worker sends the mail.
  `worker/src/consumers/mail_<name>.rs` builds it from the event's snapshot and sends it through
  `send_once`, which keeps a `sent_mails` row per event — a redelivered event does not send it
  again. 5xx → dead letter, 4xx / refused / timeout → retried with the delivery policy. Wired in
  `registry.rs` on both transports with one `SmtpMailer` and one `PgSentMails`
- **`puerto-mail`**, a new runtime crate: `MailConfig::from_env` (never prints the password in
  `SMTP_URL`), `SmtpMailer` (lettre over rustls), `Templates`, `send_once`, and behind
  `test-utils` `RecordingMailer` / `InMemorySentMails`. `puerto-core` gains `Email`, `Mailer`,
  `MailError`, `SentMails`; `puerto-sqlx` gains `mail::PgSentMails` and `SENT_MAILS_MIGRATION`
- `puerto.toml` gains `[[mail]] { name, on }` (wiring only); `puerto validate` checks names,
  events and the mail's files; `puerto list` shows the mails
- `make test/mail` (the SMTP adapter against Mailpit); `make test/db` and `make test/kafka` run the
  mail end to end against Mailpit: sent once, skipped when redelivered, retried while SMTP is down

### Breaking

- An entity can no longer be named `Mail`: mails own the `mail` modules of `business` and
  `infrastructure`

### Migration Guide

No projects generated by 0.17 are known to be in use. To bring one forward: bump the runtime crates
to `0.18`, and rename an entity called `Mail` before running `puerto generate mail`.

### Release

- Publish order: `puerto-core` → `puerto-web` → `puerto-sqlx` → `puerto-events` →
  `puerto-kafka` → `puerto-mail` → `puerto`

## [0.17.1] - 2026-10-12

### 🐛 Bug Fixes

- `puerto new --db` declares `pub mod db;` in `infrastructure/src/lib.rs` right away. It used to
  wait for the first scaffold, so a project from `puerto new --db --events db|kafka` did not
  compile (the worker names `infrastructure::db`). Broken since 0.15; an existing project only
  needs the line added

### Release

- Publish order: `puerto-core` → `puerto-web` → `puerto-sqlx` → `puerto-events` →
  `puerto-kafka` → `puerto` (runtime crates unchanged, republished at the CLI's version)

## [0.17.0] - 2026-10-12

Developer experience: what a walkthrough of a fresh project found (PLAN Phase 15). The default
project now keeps what it is given, a startup failure explains itself, and the web form says what
went wrong next to the field.

### 🚀 Features

- **The in-memory repository keeps what it is given** — without `--db`, `POST` answered 201 and
  `GET /:id` then 404. `InMemory{Entity}Repository` now behaves like the Postgres adapter:
  deleted rows hidden, lists newest first and paged, a unique value held by another live row
  answers `Duplicate`, with generated unit tests for each. `make run` says the data is lost on
  restart
- **A startup failure is an error message, not a panic** — `build_app` / `build_worker` return
  `Result<_, String>` with a hint per cause (`DATABASE_URL` in `.env`, `make docker-compose/up`,
  the Kafka broker). `puerto-web` gains `App::try_serve` and `puerto-events` `Worker::try_run`:
  the error is logged and the process exits 1. `puerto_sqlx::DatabaseError` names
  `host:port/database` and the driver's cause, never the password, and a refused connection is
  reported at once instead of after the 30 s pool timeout
- **The web form says what went wrong, next to the field** — `domain/<entity>/messages.ts`,
  generated from the same codes as `errors.rs`, maps a code to a message and a field;
  `FieldError` shows it under the input (`aria-invalid`), other codes go to the alert
- **Snippets** for events projects (`uow-write`, `app-use-case-uow`, `consumer-arm`), custom
  endpoints (`poem-endpoint-action`, `poem-route-test`) and the frontend (`web-container`,
  `web-container-test`, `web-repo-method`, `web-form-field`, `web-route`) — Zed and VS Code
- `web/src/presentation/core/routes.tsx` (`customRoutes`): your own screens, rendered by the
  generated route table and never overwritten
- The web list keeps its page in the URL (`?page=N`): reload and back keep it
- `puerto list` shows the events transport, custom events, shared value objects and consumers

### 🐛 Bug Fixes

- `generate use-case` puts the custom use case's mock in the route tests' `Mocks`
- A custom use case no longer logs its params (user data); its generated test is
  `#[ignore = "write this test"]` ending in `todo!()` instead of an empty body that passed

### Breaking

- `puerto_sqlx::DatabaseError::ConnectionError` and `MigrationError` are now struct variants
  (`{ target, cause }`, `{ cause }`); `puerto_web::Error` gains `Startup`
- Generated `bootstrap.rs` / `registry.rs` return `Result`, and `main.rs` calls `try_serve` /
  `try_run`. `serve` and `run` are unchanged: a 0.16 `main.rs` compiles against the 0.17 runtime
  until `bootstrap.rs` is regenerated, then it needs `try_serve`
- The generated web form takes a `problem` prop; the generated list reads its page from the URL

### Migration Guide

No projects generated by 0.16 are known to be in use. To bring one forward: bump the runtime
crates to `0.17`, run `puerto generate bootstrap` (it also rewrites `worker/src/generated/registry.rs`),
replace `.serve(` with `.try_serve(` in
`presentation/src/main.rs` and `.run(` with `.try_run(` in `worker/src/main.rs`, and re-run
`puerto generate web <Entity> --force` for each entity's screens.

### Release

- Publish order: `puerto-core` → `puerto-web` → `puerto-sqlx` → `puerto-events` →
  `puerto-kafka` → `puerto`

## [0.16.0] - 2026-10-12

Fixes from a walkthrough of every command on five generated projects. Generated code is now
formatted as it is written, so a project's own `make format` passes.

### 🐛 Bug Fixes

- **Generated code passes `cargo fmt --check`** (ADR-0019) — every command that writes Rust ends
  with `cargo fmt --all` in the project; a failed format only warns. A fresh project had 3–7
  diffs, one with two scaffolded entities about 200
- `generate scaffold --force` on an entity that no longer has a local value object removes
  `value_objects.rs` and its `pub mod` — the stale file did not compile
- `generate presentation` before `application` / `repository` (API first) writes the routes and
  `tags.rs` but leaves `bootstrap.rs` alone, and names the next step — it used to write a
  bootstrap that did not compile and print "All layers wired"
- Generated Makefile: `docker compose up -d --wait` (the first `make sqlx/migrate` failed while
  Postgres started); `make run`, `make worker` and `make test/infrastructure` start the database;
  `test/infrastructure` defined once; colors print where `sh` is bash; `web/*` targets ask for
  `make web/install` instead of failing with exit 127
- The migration hint after a scaffold adds `make sqlx/prepare`; `generate consumer --help` says
  `--events`

### Breaking

- Generated output is reformatted (whitespace and import order only). No migration needed:
  `cargo fmt --all` brings an existing project to the same layout

### Release

- Publish order: `puerto-core` → `puerto-web` → `puerto-sqlx` → `puerto-events` →
  `puerto-kafka` → `puerto` (runtime crates unchanged, republished at the CLI's minor)

## [0.15.0] - 2026-10-12

Domain events (ADR-0018). In a `--db` project created with `--events db|kafka`, every write of a
CRUD entity saves the entity and records its event in an `outbox` table in one transaction, and a
second process, `worker/`, relays the outbox to a transport and runs the consumers. Projects
created without `--events` are generated exactly as in 0.14.

### 🚀 Features

- **`puerto new --db --events db|kafka`** — `[project] events`, the outbox migration, and per
  CRUD entity a generated `events.rs` (`{Entity}Snapshot`, `{Entity}Event`, `{ENTITY}_CREATED`…
  constants) and `unit_of_work.rs`. The repository port only reads: writes go
  `uow_factory.begin()` → `save` → `record` → `commit`, tested with `RecordingUnitOfWorkFactory`
- **`puerto generate event <Entity> <name>`** — a custom event beyond created / updated / deleted
- **`worker/`** — a second binary on the new runtime crate **`puerto-events`**: the outbox relay,
  one consume loop per consumer with the delivery policy (`Ack` / `Retry` with a 1 s → 300 s
  backoff / dead letter after `MAX_DELIVERIES`), `/health` + `/health/ready` on `WORKER_PORT`,
  graceful shutdown. `generated/registry.rs` is regenerated from `puerto.toml`; `make worker`,
  `make events/dead-letters`
- **`puerto generate consumer <name> --on <event_type>[,…]`** — `[[consumer]]` in `puerto.toml`,
  an idempotent `EventHandler` stub with its tests in `worker/src/consumers/<name>.rs`
- **`db` transport** (`puerto-sqlx`): deliveries are rows in Postgres (`event_deliveries`), no
  broker
- **`kafka` transport** — the new runtime crate **`puerto-kafka`** (rdkafka, needs `cmake`): one
  topic per aggregate keyed by its id (the events of one aggregate stay in order), one consumer
  group per consumer, offsets committed after the outcome, a `Retry` retried in process without
  reordering. `docker-compose.yml` gains Redpanda and `.env` `KAFKA_BROKERS`
- `puerto-core` gains the event envelope, `DomainEvent`, `EventHandler`, the delivery policy and
  the transport ports; `puerto validate` checks events and consumers

### 🐛 Bug Fixes

- Web import specifiers are sorted as Biome does (the upper-case letter first), so
  `bun run lint` passes on entities whose types and helpers share a letter

### Release

- Publish order: `puerto-core` → `puerto-web` → `puerto-sqlx` → `puerto-events` →
  `puerto-kafka` → `puerto`

## [0.14.0] - 2026-10-12

The code every generated project shared as a copy now lives in three runtime crates, released in
lockstep with the CLI (ADR-0017): a fix to the server, the health probes or the Postgres error
mapping reaches existing projects through `cargo update` instead of staying wrong in every
project generated before it. New infrastructure (auth, jobs, mailer, storage) lands there next.

### Breaking

- **`puerto-core`** — `RepositoryError` (now `#[non_exhaustive]`), `PageRequest`, `Page<T>`.
  `business/src/domain/shared/{pagination,repository_error}.rs` are no longer generated; imports
  move from `crate::domain::shared::…` to `puerto_core::…`. Every entity's
  `From<RepositoryError>` gains a `_` arm (→ `RepositoryError`, 500)
- **`puerto-web`** — the server (`App::serve`: tracing, CORS, banner, graceful shutdown),
  `Config::from_env`, the `/health` and `/health/ready` probes and `ErrorResponse`.
  `presentation/src/main.rs` is `Config::from_env()?` + `App::new(config).serve(build_app)`;
  `health.rs` is gone; `api/error.rs` keeps only the `IntoErrorResponse` trait (the orphan rule
  keeps it in the project); `build_app(config)` ends with `crate::app::configure(…)`
- **`puerto-sqlx`** (`--db` projects) — `create_postgres_pool`, `run_migrations`, `ping`,
  `map_sqlx_error`, `DatabaseError` (now `#[non_exhaustive]`). `infrastructure/src/db.rs` is
  `pub use puerto_sqlx::*;` plus the project's `MIGRATOR`, and `bootstrap.rs` calls
  `run_migrations(&pool, &infrastructure::db::MIGRATOR)`
- `HARBOR_BANNER` is now `PUERTO_BANNER`
- A set but invalid `SERVICE_PORT` or `PUERTO_BANNER` stops the server at startup instead of
  silently falling back to the default

### 🚀 Features

- Runtime requirements are declared once, in the root `Cargo.toml`'s `[workspace.dependencies]`
  on the CLI's minor (`puerto-core = "0.14"`), and inherited by each member with
  `{ workspace = true }`
- **`presentation/src/app.rs`**: the project's own hook, `configure(routes) -> impl Endpoint`,
  for middleware and extra routes. Written once by `puerto new`, never touched by a generator
- `puerto validate` warns when a runtime crate is on another minor than the CLI
- The generated root `AGENTS.md` names each runtime crate and what it owns

### Migration Guide

No project generated before 0.14.0 is in use, so the simplest path is to regenerate. For a project
you keep, hand this to your coding agent:

> Upgrade this Puerto project from 0.13 to 0.14. In the root `Cargo.toml` add
> `[workspace.dependencies]` with `puerto-core = "0.14"`, `puerto-web = "0.14"` and, if
> `infrastructure` uses sqlx, `puerto-sqlx = "0.14"`; add `<crate> = { workspace = true }` to
> `business`, `infrastructure` and `presentation` (`puerto-web` only in `presentation`,
> `puerto-sqlx` only in `infrastructure`). Delete `business/src/domain/shared/pagination.rs`,
> `shared/repository_error.rs` and `presentation/src/health.rs`, and replace every import of
> them with `puerto_core::{Page, PageRequest, RepositoryError}` or `puerto_web::ErrorResponse`;
> add a `_ => Self::RepositoryError` arm to each `From<RepositoryError>`. Copy
> `presentation/src/main.rs`, `presentation/src/app.rs`, `presentation/src/api/error.rs` and
> (with a database) `infrastructure/src/db.rs` from a fresh `puerto new` of the same kind, move
> any middleware you had in `main.rs` into `app.rs`, rename `HARBOR_BANNER` to `PUERTO_BANNER`
> in `.env`, then run `puerto generate bootstrap`, `puerto validate` and `make test`.

## [0.13.0] - 2026-10-11

Three backlog items that a production service needs on day one and cannot cheaply add on day 100:
ids that cannot be mixed up, one error type for every repository, and lists that never read a whole
table. All three change generated code, hence the minor bump.

### Breaking

- **Entity ids are newtypes.** `Product.id`, the get / update / delete params and
  `find_by_id` take `ProductId` (declared in `model.rs`) instead of `Uuid`. The wire format, the
  SQL column and `web/` are unchanged (ADR-0014)
- **Repositories return `RepositoryError`** (`business/src/domain/shared/repository_error.rs`)
  instead of the entity's error; every entity error gets `From<RepositoryError>`. The API's error
  codes are unchanged (ADR-0015)
- **`list_*` is paged.** `find_all(page: PageRequest) -> Page<Entity>`,
  `ListEntitiesParams { page }`, and `GET /entities?limit=&offset=` answers
  `{ items, total, limit, offset }` instead of a bare array (ADR-0016)
- Shared value objects are declared in `business/src/domain/shared.rs`; `shared/mod.rs` is no
  longer generated

### 🚀 Features

- **Typed ids:** `{Entity}Id::generate()`, `from_uuid`, `value()` and `Display`; passing another
  entity's id is a compile error
- **One driver-error mapping:** `infrastructure::db::map_sqlx_error` turns a `23505` unique
  violation into `Duplicate` (→ `409`) and anything else into `Unavailable`; a row the domain
  rejects is reported as `InvalidData` and logged with the reason. Pg repositories with a
  `unique` field ship an integration test for the duplicate
- **Paged lists:** `PageRequest` (default 20, capped at 100 in the domain — no route can bypass
  it) and `Page<T>`; Postgres reads `ORDER BY created_at DESC, id LIMIT/OFFSET` plus a `COUNT(*)`
- **Paged screens:** the web list screen asks for one page at a time and renders a new
  `Pagination` control ("1–20 of 45", Previous / Next)
- New generated tests: the `From<RepositoryError>` arms, a duplicate on create, an unavailable
  store on get, the page passed through the list use case and route, and the Pg page of a larger
  table

### Migration Guide

No project generated before 0.13.0 is in use, so the simplest path is to regenerate. For a project
you keep: copy `business/src/domain/shared.rs` and `shared/{pagination,repository_error}.rs` from a
fresh `puerto new`, declare `pub mod shared;` in the `domain` block of `business/src/lib.rs`
(replacing `shared/mod.rs` with `shared.rs` if you have shared value objects), then re-scaffold
each entity with `puerto generate scaffold <Name> --force` and `make web/sync`. Hand-written code
that called `find_all()` or `find_by_id(uuid)` needs `PageRequest::default()` and
`{Entity}Id::from_uuid(uuid)`; a mock returning `Err(EntityError::RepositoryError)` from a
repository returns `Err(RepositoryError::Unavailable)`.

## [0.12.0] - 2026-10-11

Full-stack: every project now ships a React frontend, and `scaffold` writes each CRUD entity's
screens next to its Rust layers — Rails-style, but with the same Dependency Rule on both sides and
the OpenAPI spec as the contract between them.

### 🚀 Features

- **`web/` in every new project** (`puerto new --no-web` skips it; the wizard asks): React 19 +
  daisyUI 5 on Bun, wouter, Vite with `/api` proxied to `:8080`, `bun test` with happy-dom and
  Testing Library, Biome, and dependency-cruiser checking `domain` ← `infrastructure` ←
  `presentation`. `[project] web = true` in puerto.toml; `make web/install`, `web/dev`,
  `web/test`, `web/lint`; `make test` and `make lint` include the frontend (ADR-0013)
- **The OpenAPI spec is exported** to `docs/api/openapi.json` by a test in `bootstrap.rs`, built
  from mocked use cases — no server, no database. `make web/sync` re-exports it, regenerates the
  frontend's wire types (openapi-typescript) and formats `web/`; until then `bun run typecheck`
  fails, which is the drift check
- **Screens per entity:** in a web project, `generate scaffold` writes the domain types, the
  repository port, an HTTP repository over openapi-fetch, list / detail / form (create + edit)
  containers and views, an Object Mother and an in-memory fake, all tested. The route table and
  the composition root (`web/src/presentation/core/generated/`) are regenerated from puerto.toml
  like `bootstrap.rs`
- **Every field type has its input:** text, number (`step="any"` for `f64`), checkbox,
  `datetime-local`, a select for enum value objects (a union type narrowed by `parse<Vo>()`), a
  —/Yes/No select for `Option<bool>`, comma-separated lists, a `key=value` textarea for maps; an
  empty optional input is `null`. Server validation is the authority: the form shows the code the
  API answers
- **`puerto generate web <Name> [--force]`**: the screens alone, for an entity already in
  puerto.toml — the last step of the layer-by-layer flow
- The overwrite guard covers the frontend files; `puerto validate` and `generate bootstrap` report
  a CRUD entity whose screens are missing

### 🐛 Fixed

- The `domain-use-case` IDE snippet ships the use case mock (`mocks::Mock{Uc}UseCase`)

### Changed

- A project created without `--no-web` needs [Bun](https://bun.sh) for `make test` and `make lint`

### 🧪 Internal

- The compile matrix installs, typechecks, lints (Biome before any formatting), tests and builds
  the generated frontend of every web scenario; new scenarios `web_crud` and `web_every_type`
- Generated TypeScript comes out Biome-clean: imports sorted and wrapped like Biome, routes wrapped
  by width; the per-field frontend decisions live in `types::web_field_ctx`

## [0.11.0] - 2026-10-11

The last batch of shape changes before the first projects are generated for real: the presentation
layer now depends on use case ports and every CRUD route ships with tests, tables and paths use real
plurals, and the list use case is named after the plural. Underneath, the generators finished their
move to Tera templates — a refactor proven byte-identical by new snapshots of every generated file.

### 🐛 Fixed

- **A failed generation step still printed `✓ Done. Zero manual wiring.`** — `scaffold` ignored the
  results of the `lib.rs` patchers, the Mother and shared VO writers and the puerto.toml / bootstrap
  update. Every step now propagates its error; render errors name the template, write errors name
  the path, and no generator panics on a render failure
- **Plurals:** `Category` was the table and path `categorys`

### 🚀 Features

- **Route tests in every project:** the CRUD `routes.rs` is generated with tests that drive the real
  `OpenApiService` through `poem::test::TestClient`, one mocked use case per test, one test per
  status path (create 201 / 422, get 200 / 404, list 200 / 500, update 200 / 404, delete 204 / 404);
  the demo `GreetingApi` gets 200 / 422
- **Use case mocks:** every use case trait ships `mocks::Mock{Uc}UseCase` behind
  `cfg(any(test, feature = "test-utils"))`, like the repository mocks; Object Mothers compile under
  the same gate, so other crates' tests can build entities
- **Real plurals** for SQL tables and REST paths: `categories`, `boxes`, `statuses`, `people`,
  `order_items`; uncountables (`data`, `series`, …) stay as they are

### 🧪 Internal

- **Snapshot of every generated file**, one per compile-matrix scenario (Postgres included), checked
  by `make test` on every push; `make test/snapshots` re-records them after an intended change
- **Generators render through Tera templates over a per-field `FieldCtx`** (ADR-0011): templates own
  the shape of the code, `types.rs` owns every per-field decision; no type name is hard-coded
  outside it. Snapshots unchanged throughout
- `make lint` covers test targets; the 6.6k-line `tests.rs` is split into one module per area
- Mutation testing: 0 survivors

### Breaking

Generated code changes shape:

- **`{Entity}Api` holds ports:** fields are `Arc<dyn {Uc}UseCaseTrait>` (were
  `Arc<{Uc}UseCaseImpl>`); `routes.rs` no longer imports `business::application::*` (ADR-0012).
  `presentation` dev-depends on `business` with `test-utils` and on `poem` with `test`
- **Tables and REST paths use real plurals** — `categorys` → `categories`, `/categorys` →
  `/categories`
- **The list use case is `list_{plural}`:** `list_product` → `list_products` (file, module,
  `ListProductsUseCaseTrait`, `ListProductsParams`, `ListProductsUseCaseImpl`,
  `ListProductsResponse`, `ProductApi.list_products`, puerto.toml)

### Migration Guide

No project generated with an earlier Puerto is known to be in use. For one that is:

1. In every `routes.rs`, change `Arc<{Uc}UseCaseImpl>` fields to `Arc<dyn {Uc}UseCaseTrait>` and
   import the trait from `business::domain::{entity}::use_cases::{uc}` instead of the impl. Add
   `business = { path = "../business", features = ["test-utils"] }` and
   `poem = { version = "3", features = ["test"] }` to `presentation`'s `[dev-dependencies]` if you
   want to add route tests.
2. For each entity, rename `list_{snake}` to `list_{plural}`: the use case files in
   `domain/{entity}/use_cases/` and `application/{entity}/`, their `pub mod` lines in
   `business/src/lib.rs`, the type names, the `{Entity}Api` field and the entry in puerto.toml.
   Then run `puerto generate bootstrap`.
3. Entities whose plural is irregular (`-y`, `-s`, `-x`, `-ch`, `-sh`, `person`, `child`, …):
   either rename the table with a migration (`ALTER TABLE categorys RENAME TO categories;`) and tell
   API clients about the new path, or keep the old name by hand in the repository queries and
   routes.

## [0.10.1] - 2026-10-10

### 🐛 Fixed

- **IDE snippets had drifted from the generated code**, with nothing tying them to it:
  `NaiveDateTime` where projects use `DateTime<Utc>` (`domain-model`, `persistence-entity`), DTOs
  without camelCase, a Pg repository that swallowed errors and never mapped `23505` → `Duplicate`,
  endpoints without tags or failure logs, an Object Mother with a different API, a `cfg-test` block
  whose `Arc` import failed `make lint`, a repository trait without `find_all`

### 🚀 Features

- **New snippets:** `inmemory-repo`, `poem-endpoint-get`, `app-use-case-error-test`,
  `poem-error-mapper-test`, `migration-unique-index` (partial), `sql-select-active`
- **Snippets are compiled in Puerto's test suite:** an entity is built from snippets alone and must
  pass clippy `-D warnings` and its tests — in memory (`make test/full`) and against Postgres,
  migration and integration tests included (`make test/db`)

### 📚 Docs

- **The crates.io README told people to `cargo install puerto-framework`** — the crate's old name —
  and documented a `scaffold --db` flag that does not exist. Rewritten for 0.10.
- **Install with `cargo install puerto --locked`** (README, crate README, website). Without
  `--locked`, cargo resolves the newest transitive dependencies, and `kstring 2.0.5` (via
  cargo-generate → liquid) now requires Rust 1.96 — so a plain `cargo install puerto` fails on 1.95.
- `puerto new my-app` examples corrected to `puerto new --name my-app`.

## [0.10.0] - 2026-10-10

An end-to-end review generated real projects with every option — `--db` against a live Postgres
included — and found output that did not compile or did not pass the project's own `make lint`.
This release fixes all of it, then adopts the conventions of a hand-written 60k-LOC backend built
on the same architecture (`document-api`). The compile matrix now runs clippy `-D warnings` in every
generated project and has a Postgres scenario (`make test/db`). 0.9.1 and 0.9.2 were tagged but never
reached crates.io; their changes ship here.

### 🐛 Fixed

- **A fresh `puerto new` failed its own `make lint`** — ~57 clippy errors in a project with value
  objects (`map_err` that returns its input, redundant field names, argument-less `format!`, an
  inherent `from_str` on enum VOs, unused test imports, Mothers without `Default`)
- **`generate use-case` on a scaffolded entity broke compilation** (bootstrap wired a field
  `routes.rs` did not have) and **silently replaced** a working implementation with a `todo!()`
  stub when the action already existed — including the CRUD ones
- **`--db` projects:** `HashMap` columns did not compile (now `sqlx::types::Json`); Pg repository
  tests missed imports, compared nanosecond timestamps with microsecond ones and seeded `unique`
  fields with constants; two scaffolds in the same second produced the same migration version;
  a field named `desc`/`order`/`user` broke the migration
- **Entity whose only `String` field is an enum VO:** its create test did not compile
- **Create use case logged the literal text `params.x`** instead of a value
- `puerto new` suggested `cargo run -p <name>` (the package is `presentation`)
- `.sqlx/` was git-ignored while builds run with `SQLX_OFFLINE=true`
- The AI rules shipped in every project described `mod.rs` files — the one thing the module
  convention forbids — and were not loaded by Claude Code (no root `CLAUDE.md`)

### 🚀 Features

- **Health and shutdown:** `GET /health` (liveness), `GET /health/ready` (readiness — `503` until
  the database answers), graceful shutdown on SIGTERM / Ctrl-C
- **Agent rules:** every project ships a root `AGENTS.md` + `CLAUDE.md` — architecture, commands,
  conventions, one commit per piece, risk tiers for human review — kept in sync by tests
- **Pinned toolchain:** `rust-toolchain.toml` (1.95.0), edition 2024, resolver 3
- **Validation at the command that writes:** `generate value-object` (name, type, duplicates),
  `generate use-case` (snake_case, existing actions), duplicate field names, SQL reserved words in
  `--db` projects, project names in `puerto new --name`
- `generate migration` writes the file itself (no `sqlx` CLI needed) and is refused without `--db`
- `make test/db` (Postgres scenario) and `make test/mutation` (cargo-mutants: 335 mutants, 0
  survivors)
- `docs/adr/` — ten Architecture Decision Records

### Breaking

Generated code changes shape:

- **No logger port.** `LoggerTrait`, `TracingLogger`, `business::domain::logger`, every `logger`
  field and `silent_logger()` mock are gone; code logs with `tracing` macros. Repository
  constructors take no logger (`InMemoryXRepository::new()`, `PgXRepository::new(pool)`)
- **Error contract.** `ErrorResponse.name` is the error code (`product.not_found`) — it was a
  generic `product_error` with the code in `message`. Domain validation and invalid value objects
  answer **422** (was 400); response enums carry `UnprocessableEntity` instead of `BadRequest`
- **Enum value objects implement `FromStr`** instead of an inherent `from_str`
- **Unique indexes are partial** (`WHERE deleted = false`)
- Entity names `Tracing` and `Db` are rejected; `generate use-case` is no longer idempotent (a
  second run is refused)

### Migration Guide

No project generated with an earlier Puerto is known to be in use. For one that is:

1. Delete `business/src/domain/logger.rs` and `infrastructure/src/logger.rs` and their `pub mod`
   lines; remove `logger` fields and arguments; replace `self.logger.info(&x)` with
   `tracing::info!(…)`; add `tracing = "0.1"` to `business` and `presentation`.
2. Run `puerto generate bootstrap`.
3. In each `error_mapper.rs`, build bodies with `ErrorResponse::from_code(self.to_string())` and map
   `ValidationError` / `Invalid*` to `StatusCode::UNPROCESSABLE_ENTITY`; in `responses.rs` replace
   the `400 BadRequest` variant with `422 UnprocessableEntity`. Tell API clients that the code is
   now in `name`.
4. Replace `Vo::from_str(&x)` with `x.parse::<Vo>()` (or import `std::str::FromStr`).
5. Recreate unique indexes as partial: `DROP INDEX t_f_key; CREATE UNIQUE INDEX t_f_key ON t (f)
   WHERE deleted = false;`

## [0.9.2] - 2026-08-15

Two rounds of work: making generated projects actually compile, and making the CLI honour what the
docs already promised. A compile matrix now gates both — it generates real projects, scaffolds into
them and runs `cargo test --workspace` inside, which is how the bugs below were found.

### 🐛 Fixed

- **`Option<VO>` and `Vec<VO>` emitted model tests that did not compile** — the test-props builders
  branched only on enum/plain VOs, so `Option<VO>` got `VO::new(None)` and `Vec<VO>` got
  `VO::new(vec![])`, both with the wrong inner type
- **`puerto new --no-demo` dropped `pub mod tags;`** from `presentation/src/api.rs`, breaking the
  first scaffold into an empty project
- **An entity with no use cases emitted syntactically invalid `bootstrap.rs`** (`EntityApi { , logger: … }`)
- **`DateTime<Utc>` / `Uuid` fields re-imported chrono and uuid in the model** (E0252)
- **`get` / `list` / `delete` test modules missed field-type imports** (E0433)
- **DTOs used `DateTime` / `HashMap` with no import**, no chrono dependency and no poem-openapi
  chrono feature
- **`unique = true` never reached the SQL migration.** It is documented in `AGENTS.md`, exposed as
  the `!` CLI suffix, and did nothing. Now emits `CREATE UNIQUE INDEX {table}_{field}_key` — a named
  index rather than an inline column `UNIQUE`, so a later migration can drop it by a predictable
  name. This also makes the `23505` → `EntityError::Duplicate` mapping in the Pg repository
  reachable for the first time
- **`puerto generate domain` could never receive typed fields.** A dead `find` guaranteed an empty
  field list, and the entity was registered with `fields: []` — so `application`, `repository` and
  `presentation`, which all read fields back from `puerto.toml`, were field-blind. The entire
  documented domain-first workflow could only ever produce the default `name: String` entity

### 🚀 Features

- **`puerto generate scaffold` no longer overwrites silently.** Re-scaffolding an entity that has
  generated files is refused: interactively it lists the files and asks, non-interactively it aborts
  without writing a byte. `--force` overwrites and updates the `[[entity]]` block in place, so the
  manifest keeps describing the code on disk — use cases added via `generate use-case` survive
- **`puerto generate domain <Name> -- fields`** takes the same field syntax as `scaffold` and
  persists the fields to `puerto.toml`, which is what makes the later layered commands field-aware
- **Manifest / filesystem drift detection.** `puerto validate` now errors when a declared
  `[[entity]]` has no code on disk (naming the missing paths and the command that fixes it) and
  warns on the inverse — a domain module with no `[[entity]]` block. `puerto generate bootstrap`
  refuses to write a bootstrap that will not compile; `--allow-missing` writes it anyway
- **`puerto --version`.** The flag was documented in the install instructions and did not exist —
  clap never generated it because `version` was absent from `#[command(...)]`
- **Stricter name validation**, applied identically by the CLI parser and `puerto validate`:
  system field names (`id`, `created_at`, `updated_at`, `deleted`, `deleted_at`), Rust keywords,
  entity names that collide with generated code (`Error`, `String`, `Logger`, `Type`, …), duplicate
  enum variants, and two fields declaring the same value object with different inner types

### 🚜 Refactor

- New `generators/conflict.rs` — the overwrite guard, checked before the first byte is written
- New `generators/consistency.rs` — the manifest/filesystem pass, covering exactly the files
  `bootstrap.rs` imports
- New `validation.rs` — one home for `SYSTEM_FIELDS`, `RUST_KEYWORDS`, reserved type and module
  names, and the name predicates. Called from `parse_field_arg`, from `run_validate` and before any
  scaffold write, so the two can no longer diverge
- The `-- fields` plumbing is a single `parse_cli_fields()` shared by `scaffold` and `domain`

### ⚙️ CI

- `make test/full` runs on every push and pull request, not only on tag push. It used to be
  tag-only, which is how three separate bugs that produced non-compiling projects reached `main`
- The compile matrix covers primitives, `Option`/`Vec` primitives, plain and unique value objects,
  `Option`/`Vec`/enum/shared value objects, multiple entities, `--no-demo`, and the domain-first
  layered flow with fields

### 📚 Documentation

- Swept the drift between the docs and the CLI: the `scaffold --db` flag that no longer exists, the
  old `name:Name[vo:String]` value object syntax, `crates/template/basic/`, `scaffold.rs` described
  as the home of the writers, and tests described as living in `main.rs`. Every `crates/`-relative
  path in `AGENTS.md` and `.claude/rules/*` is now verified to exist
- The landing docs now document that re-running `scaffold` is refused, and what `--force` does

### 💥 Breaking

- **Field names with a leading underscore are rejected.** `_private:String` used to parse. The
  predicate allowed it while its own error message said otherwise, and a leading underscore reads as
  "deliberately unused" to every Rust reader.
- **Re-running `puerto generate scaffold <Name>` on an existing entity is refused** instead of
  overwriting. Non-interactive callers (CI, scripts) now get a non-zero exit.
- **`puerto generate bootstrap` refuses** when `puerto.toml` declares an entity whose code is not on
  disk, instead of writing a file that does not compile.
- **`puerto validate` fails on inputs it used to accept**: system field names, Rust keywords,
  reserved entity names, duplicate enum variants, incoherent value objects, and declared-but-
  unscaffolded entities.

### Migration Guide

Existing projects need no file changes. The commands are stricter, so adjust any automation:

1. **Scripts that re-run `scaffold` to regenerate an entity** must pass `--force`:

   ```bash
   puerto generate scaffold Product --force -- name:String price:i64
   ```

   Without it the command exits non-zero and writes nothing.

2. **Run `puerto validate` once** and fix what it reports. The new checks catch manifests that were
   already producing code that does not compile:

   - a field named `id` / `created_at` / `updated_at` / `deleted` / `deleted_at` → rename it; every
     entity already has those
   - a field named after a Rust keyword (`type`, `match`, `move`, …) → rename it
   - a field name starting with `_` → rename it
   - an entity named `Error`, `String`, `Logger`, `Type`, … → rename it
   - an entity declared in `puerto.toml` but never scaffolded → run the command `validate` suggests,
     or delete the `[[entity]]` block

3. **If `puerto generate bootstrap` now refuses**, the manifest and the filesystem disagree. Fix the
   drift it names. `--allow-missing` restores the old behaviour if you genuinely want the file
   written anyway.

4. **SQL projects with `unique = true` fields**: the constraint was never generated, so existing
   tables do not have it. New scaffolds emit it. To add it to an existing table:

   ```sql
   CREATE UNIQUE INDEX products_sku_key ON products (sku);
   ```

## [0.8.0] - 2026-05-04

### 🚀 Features

- **Entity fields in `puerto.toml`** — define typed fields on entities via `[[entity.fields]]` blocks. Generated model structs, Props, Params, DTOs, repository rows, and SQL migrations all derive from the field list automatically
- **Type registry** — 13 Rust types supported out of the box: `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`, `Option<T>`, `Vec<T>`, `HashMap<String, String>`, and their nullable/array variants. Each type maps to SQL column types, OpenAPI formats, and default test values
- **Scaffold with fields** — `puerto generate scaffold Product name:String price:i64! sku:String` passes typed fields via CLI. Fields are persisted to `puerto.toml` and used across all layers
- **`puerto validate`** — new command that validates `puerto.toml`: entity names (PascalCase), field names (snake_case), field types (against the type registry), duplicate entities/fields, and warns about `Option` + `unique` combinations
- **Dynamic generators** — all layer generators now produce typed code from the field list. When `fields` is empty, the previous `name: String` default is preserved for backward compatibility
- **Field-aware Object Mother** — `ProductMother` generates builder methods for each custom field (`with_price()`, `with_empty_name()`, etc.) instead of just `with_name()`

### 🚜 Refactor

- Replaced string-constant generators with dynamic functions across all layers (`generate_model()`, `generate_mother()`, `generate_crud_dto()`, `generate_crud_routes()`, `generate_infra_entity()`, `generate_crud_infra_db_repository()`)
- Added `generators/types.rs` module with `TypeMapping` registry, `resolve_type()`, `validate_fields()`, `collect_imports()`
- Added `commands/validate.rs` module with full puerto.toml validation

## [0.7.2] - 2026-05-04

### Changed

- Refactor: split `main.rs` and `scaffold.rs` into dedicated modules (`commands`, `generators`, `patchers`, `tests`)

## [0.7.0] - 2026-05-03

### 💥 Rebranding — `puerto-framework` → `puerto`

This release marks the rename of the crate from `puerto-framework` to `puerto`.

The name Puerto was taken by many existing tools, making SEO and discoverability difficult. **Puerto** (Spanish for _port_) keeps the original nautical metaphor, is easy to pronounce in English, and has a unique presence on crates.io.

**What changed:**

- Crate name on crates.io: `puerto-framework` → `puerto`
- GitHub repo: `Guuri11/puerto` → `Guuri11/Puerto`
- Everything else is identical — CLI commands, generated project structure, `puerto.toml` schema

**Migrating:**

```toml
# Cargo.toml
[dependencies]
# before
puerto-framework = "0.6"
# after
puerto = "0.7"
```

The `puerto-framework` crate on crates.io is now deprecated. No further versions will be published under that name.

---

## [0.6.0] - 2026-05-03

### 🚀 Features

- Add `puerto generate domain <Name>` — scaffolds domain layer only (model, errors, repository trait, use cases, Object Mother) and adds entity to `puerto.toml`
- Add `puerto generate application <Name>` — scaffolds application layer only (use case impls) for an entity already in `puerto.toml`
- Add `puerto generate repository <Name>` — scaffolds infrastructure layer only (InMemory or Pg repository, inferred from `puerto.toml`)
- Add `puerto generate presentation <Name>` — scaffolds presentation layer only (routes, dto, responses, error_mapper) and regenerates `bootstrap.rs`
- Object Mother test factory (`{Entity}Mother`) is now generated by `puerto generate domain` and `puerto generate scaffold` — provides `random()`, `random_vec(n)`, builder chain, and `build_props()` helpers
- Generated project Makefiles overhauled: ANSI colour output, 20+ targets, split test targets (`test/domain`, `test/application`), `check`, `clean`, `audit`, `audit/fix`, `format/fix`, `generate/entity`, `generate/use-case`, `generate/bootstrap`
- DB project Makefiles now include a full SQLx suite: `sqlx/online`, `sqlx/offline`, `sqlx/migrate`, `sqlx/prepare`, `sqlx/check`, `generate/migration`; Docker helpers `docker-compose/up`, `docker-compose/down`, `db-up`, `db-down`, and a `reset-db` target with destroy confirmation
- Puerto root `Makefile` updated with ANSI colours, `format/fix`, `audit`, `audit/fix` targets
- Generated project `AGENTS.md` files updated with Object Mother docs and layer-by-layer command guidance

### 📚 Documentation

- `/docs` page: added "Layer Generators" section documenting all four new per-layer commands
- `AGENTS.md` (puerto root): CLI commands table updated with four new `puerto generate` commands
- Template `AGENTS.md` files updated to reference per-layer generators for AI-assisted development

## [0.5.0] - 2026-04-27

### 🚀 Features

- `puerto generate scaffold` now infers `--db` from `puerto.toml` (`project.db = true`) — no need to pass the flag when the project already uses SQLx
- Interactive `puerto new` demo prompt: shows a real project preview (`my-app`) before asking for the actual project name
- Bump to v0.5.0

## [0.4.0] - 2026-04-27

### 🚀 Features

- Generated projects now print a Puerto ASCII banner on startup (set `HARBOR_BANNER=false` to suppress)
- HTTP request logging via poem's `Tracing` middleware — every request/response is traced automatically
- `InMemoryRepository` receives the project logger; repository-level operations are now logged at `debug`
- `GreetingApi` (and all generated `*Api` structs) receive the logger; errors are logged at `warn`
- `uuid` and `chrono` template dependencies now include `serde` features for JSON serialisation out of the box
- `poem-openapi` template dependency now includes `uuid` feature for typed UUID path/query params

### 💥 Breaking

- `InMemoryGreetingRepository` is no longer a unit struct — constructor is now `InMemoryGreetingRepository::new(logger: Arc<dyn LoggerTrait>)`. Update `bootstrap.rs` in existing projects (or run `puerto generate bootstrap`).
- `GreetingApi` (and all scaffolded `*Api` structs) now require a `logger: Arc<dyn LoggerTrait>` field. Update `bootstrap.rs` in existing projects.
- `build_app()` in `generated/bootstrap.rs` is now `async` and returns `impl poem::Endpoint` (was `Route`). Update `main.rs` to `generated::bootstrap::build_app().await`.

### 🐛 Bug Fixes

- `puerto new --db` now correctly writes `project.db = true` to `puerto.toml` — was silently omitted, causing `puerto generate scaffold --db` to not detect db mode correctly

### ⚙️ Miscellaneous Tasks

- Test coverage: add `db_project_puerto_toml_has_project_db_true` and `no_db_project_puerto_toml_omits_project_db` structural tests

## [0.3.0] - 2026-04-25

### 🚀 Features

- Add `puerto generate snippets [--ide zed|vscode]` command — writes TextMate-format snippet files for Zed (`.zed/snippets/rust.json`) and VS Code (`.vscode/puerto.code-snippets`), compatible with nvim+LuaSnip
- `puerto new` now automatically writes snippet files to both IDEs on project creation
- 23 Puerto-adapted snippets covering all DDD layers: domain model, errors, repository trait, use case trait/impl, persistence entity/repo, tokio test, sqlx integration test, Object Mother, and more

### 🐛 Bug Fixes

- Fix `cargo fmt` violation in `scaffold.rs` (`apply_db_to_new_project` method chain formatting)

### 📚 Documentation

- Add IDE Snippets section to `/docs` page with snippet inventory, IDE setup instructions, and regeneration commands

## [0.2.0] - 2026-04-25

### 🚀 Features

- Add `LoggerTrait` domain port with `noop()` helper and mockall mock
- Add `TracingLogger` infrastructure adapter backed by the `tracing` crate
- Wire logger into all generated `UseCaseImpl` structs and `bootstrap.rs`
- Initialize `tracing_subscriber` with env-filter in generated `main.rs`
- Add `--no-db` flag to `puerto new` (explicit opt-out without prompting)
- Add `--destination` flag to `puerto new` (controls output directory)
- Generate `docker-compose.yml` and `.env` for `--db` projects
- Add db Makefile targets: `docker/up`, `sqlx/migrate`, `sqlx/prepare`, etc.
- Run migrations automatically on bootstrap startup (`run_migrations`)
- Add `uuid`, `chrono`, `serde`, `tracing`, `dotenvy` to template dependencies

### 💥 Breaking

- All generated `UseCaseImpl` structs now require a `logger: Arc<dyn LoggerTrait>` field. Existing projects must add this field manually and update `bootstrap.rs` (or run `puerto generate bootstrap`).

### 🐛 Bug Fixes

- Correct repo URL, install command, and Rust version in landing

### 🚜 Refactor

- Remove redundant crates/template, clean CLI output

### 📚 Documentation

- Update release.md crate name and AGENTS.md make commands
- Fix install command in README (puerto-framework)

### ⚙️ Miscellaneous Tasks

- Add CI and release GitHub Actions workflows

## [0.1.0] - 2026-04-12

### 🚀 Features

- Auto-wire DI via puerto.toml + generated bootstrap

### 🐛 Bug Fixes

- Rename template Cargo.toml to .liquid to fix crates.io packaging

### ⚙️ Miscellaneous Tasks

- Rename crate to puerto-framework for crates.io
