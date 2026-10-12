# Puerto — AI Agent Instructions

Puerto is a Rust CLI tool that scaffolds full-stack DDD projects. It uses `cargo-generate` to create 3-crate workspaces following Clean Architecture (Hexagonal / Ports & Adapters).

## Tech Stack

- Rust (edition 2024) · cargo-generate · poem-openapi · tokio
- mockall (testing) · thiserror (errors) · async-trait

## Puerto's Own Structure

```
crates/
  core/             — `puerto-core`, a runtime crate (ADR-0017): `RepositoryError`, `PageRequest`,
                      `Page<T>`; `events` (ADR-0018): `DomainEvent`, `EventEnvelope`, `EventHandler`,
                      the delivery policy and the transport ports; `mail` (ADR-0020): `Email`, `Mailer`,
                      `SentMails`; `auth` (ADR-0021): `Principal`, `Authenticator`, `AuthError`,
                      `mocks::MockAuthenticator` (`test-utils`). Generated projects depend on it at
                      the CLI's minor; same workspace version as `cli/`. The compile matrix builds
                      against this checkout
                      (`[patch.crates-io]`, added by the test harness only)
  macros/           — `puerto-macros` (proc-macro, ADR-0021 § 4): `#[require_role("<role>")]`, reached
                      as `puerto_web::require_role` — a 403 check at the top of a handler that takes
                      `auth: BearerAuth`; on a misuse it emits the method unchanged next to the error
  web/              — `puerto-web`, the presentation runtime: `App` (tracing, CORS, graceful
                      shutdown), `Config::from_env`, the health probes, `ErrorResponse`; `auth`:
                      `BearerAuth` (asks the `Arc<dyn Authenticator>` the routes carry as data;
                      401 `auth.unauthenticated`, 503 `auth.unavailable`), `forbidden()`
  sqlx/             — `puerto-sqlx`, the Postgres runtime (`--db` projects): the pool,
                      `run_migrations(pool, &MIGRATOR)`, `ping`, `map_sqlx_error`; for events projects
                      `uow::PgTransaction`, `events::{PgOutbox, PgDeliveries, PgSubscription,
                      PgDeadLetters}` and `migrations/outbox.sql` (the CLI's
                      `templates/infrastructure/outbox.sql` must match it byte for byte — tested);
                      for mail (ADR-0020) `mail::PgSentMails` and `migrations/sent_mails.sql` (same
                      byte-for-byte rule with `templates/infrastructure/sent_mails.sql`)
  events/           — `puerto-events`, the worker runtime (`--events` projects, ADR-0018): `Relay`
                      (outbox → `EventPublisher`), `Consumer` (subscription → `EventHandler`, `decide`,
                      dead letter before retiring), `Worker` + `Loops` (health on `WORKER_PORT`,
                      SIGTERM drain), `WorkerConfig::from_env`
  kafka/            — `puerto-kafka`, the Kafka transport (`--events kafka`, ADR-0018): `KafkaConfig`
                      (`KAFKA_BROKERS`, `KAFKA_TOPIC_PREFIX`, `ensure_topics`), `KafkaPublisher`,
                      `KafkaSubscription` (one record per batch, in-process retry, commit after
                      the outcome). `make test/kafka` runs it against Redpanda. Building it needs
                      `cmake` and the libcurl headers (librdkafka 2.12 includes `curl/curl.h`
                      even without curl support)
  mail/             — `puerto-mail`, mail (ADR-0020): `MailConfig` (`SMTP_URL`, `MAIL_FROM`),
                      `SmtpMailer` (lettre, rustls; 5xx → `Permanent`, the rest → `Transient`),
                      `Templates` (minijinja, strict, the subject on one line), `send_once` (the
                      sent log, then the send, then the mark) and, behind `test-utils`,
                      `RecordingMailer` / `InMemorySentMails`. The ports (`Email`, `Mailer`,
                      `SentMails`) are `puerto-core`'s. `make test/mail` runs it against Mailpit
  auth/             — `puerto-auth`, auth adapters (ADR-0021). Feature `oidc`: `OidcConfig`
                      (`AUTH_ISSUER`, `AUTH_AUDIENCE`, `AUTH_ROLES_CLAIM`), `RolesClaim` (a top-level
                      name as is, else a dotted path), `OidcAuthenticator::discover` (RS256/ES256
                      only, `iss`/`aud`/`exp`/`nbf` ± 60 s, an unknown `kid` refetches the JWKS at
                      most once a minute; the `Principal`'s `user_id` is nil). Its tests enable the
                      feature through a dev-dependency on itself. `make test/auth` runs it against
                      Keycloak
  cli/              — The puerto binary
    src/main.rs     — CLI definition + dispatch
    src/commands/   — Command handlers (new, list, validate)
    src/generators/ — Per-layer generators + bootstrap, conflict (overwrite guard), consistency (drift,
                      incl. the textual auth drift of routes.rs), auth (`new --auth`: User, provisioning,
                      `GET /me`, Keycloak),
                      web (the frontend slice + web/src/presentation/core/generated/*),
                      events (events.rs, the unit of work ports, `generate event`),
                      worker (the `worker/` crate, registry.rs, `generate consumer`)
    src/templates/  — .tera templates the generators render (embedded via include_dir)
    src/patchers/   — lib.rs / api.rs / routes.rs patchers
    src/runtime.rs  — Which runtime crates a project depends on, at which version (rendered from
                      the CLI's own; `puerto validate` warns on a minor mismatch)
    src/puerto_toml.rs — puerto.toml serde structs + read/write/add_entity + field parser
    src/validation.rs  — Name predicates shared by the field parser and `puerto validate`
    src/tests.rs    — Test root: shared helpers + one module per area in src/tests/
    src/tests/      — Structural tests + `#[ignore]` compile matrix (`make test/full`)
    template/       — The "basic" DDD template (edit directly here)
      Cargo.toml          — Workspace manifest (workspace only, no [package]; edition 2024, resolver 3)
      cargo-generate.toml — Template config (controls which files get substitution)
      rust-toolchain.toml — Pins the toolchain the matrix lints with (ADR-0009)
      AGENTS.md, CLAUDE.md — Agent rules shipped to every project (ADR-0010; tested)
      web/                — React + daisyUI frontend on Bun (ADR-0013); `--no-web` removes it.
                            Never `bun install` here: the template is embedded (tested)
      puerto.toml         — Source of truth for entities ({{project-name}} substituted)
      business/           — Domain + Application crate
      infrastructure/     — Adapters crate
      presentation/       — REST API crate
        src/
          main.rs.liquid        — Entry point: `puerto_web::Config::from_env()` + `App::new(config).try_serve(…)`
          app.rs                — The project's hook: `configure(routes)`, written once, never regenerated
          generated.rs          — pub mod bootstrap; (static, never changes)
          generated/
            bootstrap.rs        — AUTO-GENERATED, full DI wiring (the template copy is tested to
                                  match the generator byte for byte)
docs/adr/           — Architecture Decision Records: the *why* behind conventions (start here)
```

## puerto.toml — Source of Truth

Every generated project has a `puerto.toml` at the root:

```toml
[project]
name = "my-app"

[[entity]]
name = "Greeting"             # PascalCase
use_cases = ["get_greeting"]   # snake_case action names

[[entity]]
name = "Product"
use_cases = ["create_product", "list_products"]

[[entity.fields]]
name = "name"                  # snake_case field name
type = "String"                # Rust type from the type registry

[[entity.fields]]
name = "price"
type = "i64"

[[entity.fields]]
name = "sku"
type = "String"
unique = true                  # CREATE UNIQUE INDEX products_sku_key … WHERE deleted = false (SQL only)

[[entity.fields]]
name = "description"
type = "Option<String>"        # nullable field

[[entity.fields]]
name = "tags"
type = "Vec<String>"           # array field
```

**Rules:**

- `puerto generate scaffold <Name>` appends a new `[[entity]]` block automatically
- Re-scaffolding an entity that already has generated files is **refused** unless `--force`
  (interactively it asks). With `--force`, the `[[entity]]` block is updated in place so the
  manifest keeps describing the code on disk — use cases added via `generate use-case` survive
- `presentation/src/generated/bootstrap.rs` is regenerated from puerto.toml on every scaffold run
- **Never hand-edit `generated/bootstrap.rs`** — run `puerto generate bootstrap` instead
- All derived identifiers come from `name` (PascalCase) and `use_cases` entries (snake_case)
- If `entity.fields` is empty or absent, entities default to `name: String` (backward compatible)
- Field types must match the type registry — run `puerto validate` to check
- **Reserved names** (rejected by both the CLI and `puerto validate`): the system fields `id`,
  `created_at`, `updated_at`, `deleted`, `deleted_at`; Rust keywords (`type`, `match`, `move`, …);
  entity names that collide with generated code (`Error`, `String`, `Tracing`, `Db`, `Type`, …); in
  `--db` projects, PostgreSQL reserved words as field names (`desc`, `order`, `user`, …)
- Duplicate field names are rejected; use case actions must be snake_case
- `generate use-case` refuses an action that already exists (including the scaffolded CRUD ones)
  and adds the use case's field to `{Entity}Api` in `routes.rs`, so `bootstrap.rs` keeps compiling
- `generate value-object` validates the name (PascalCase), the inner type and duplicates before
  writing puerto.toml
- `puerto new --name` must be lowercase letters, digits, `-`, `_`, starting with a letter
- `puerto validate` also checks that every declared `[[entity]]` has code on disk — a declared but
  unscaffolded entity makes `bootstrap.rs` import modules that do not exist

### Type Registry

| Rust Type | SQL Type | OpenAPI | Default (tests) |
|-----------|----------|---------|------------------|
| `String` | `TEXT` | `string` | `"example"` |
| `i64` | `BIGINT` | `integer(int64)` | `42` |
| `bool` | `BOOLEAN` | `boolean` | `true` |
| `f64` | `DOUBLE` | `number(double)` | `1.5` |
| `Uuid` | `UUID` | `string(uuid)` | `Uuid::new_v4()` |
| `DateTime<Utc>` | `TIMESTAMPTZ` | `string(date-time)` | `Utc::now()` |
| `Option<T>` (`T` = `String`, `i64`, `bool`, `f64`, `DateTime<Utc>`) | nullable SQL | `T?` | `None` |
| `Vec<String>` | `TEXT[]` | `array[string]` | `vec![]` |
| `Vec<i64>` | `BIGINT[]` | `array[integer]` | `vec![]` |
| `HashMap<String, String>` | `JSONB` (row field `sqlx::types::Json<…>`) | `object` | `HashMap::new()` |

See `.claude/rules/puerto-toml.md` for the full derivation table and field rules.

## Generated Project Architecture

Every generated project is a Cargo workspace. Dependency rule (inward only):

```
Presentation → Infrastructure → Application → Domain
```

- **Domain** — pure Rust, no external crates beyond std/async-trait/thiserror and `puerto-core`
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
puerto new --no-web                        # Skip the web frontend (web/ is included by default)
puerto new --db --events db                # Domain events (ADR-0018): units of work + outbox (db | kafka)
puerto new --auth oidc                     # Auth (ADR-0021): an OIDC provider's tokens (Auth0, Keycloak); Keycloak in compose

puerto generate scaffold <Name>                                          # Add a new DDD entity — all layers at once (infers db from puerto.toml)
puerto generate scaffold <Name> --force                                  # Re-generate an existing entity, overwriting hand-written changes
puerto generate scaffold <Name> -- name:String price:i64!               # Primitive fields (! = unique)
puerto generate scaffold <Name> -- desc:opt:String tags:vec:String       # Option / Vec primitives
puerto generate scaffold <Name> -- name:Name:String age:Age:i64         # VO fields
puerto generate scaffold <Name> -- status:Status:enum:Active/Inactive   # Enum VO
puerto generate scaffold <Name> -- mid:MiddleName:opt:String            # Option VO
puerto generate scaffold <Name> -- tags:Tag:vec:String                  # Vec VO
puerto generate value-object <Name> <type>                    # Declare a shared VO in puerto.toml (e.g. Email String; DateTime shorthand ok)
puerto generate event <Entity> <name>      # events projects: a custom event (product_published) + regenerates events.rs
puerto generate consumer <name> --on product_created,product_published   # events projects: a worker consumer + regenerates registry.rs
puerto generate mail <name> [-- name:String seats:i64 plan:opt:String]  # a mail: port, template mailer, templates; the first adds Mailpit
puerto generate mail <name> --on customer_created --to email   # events projects: + the worker consumer mail_<name> that sends it once per event
puerto generate domain <Name>              # Domain layer only: model, errors, repo trait, use cases, Object Mother
puerto generate domain <Name> -- price:i64 status:Status:enum:Draft/Paid   # Same field syntax as scaffold — fields land in puerto.toml
puerto generate application <Name>         # Application layer only: use case impls
puerto generate repository <Name>          # Infrastructure layer only: InMemory or Pg repo (inferred from puerto.toml)
puerto generate presentation <Name>        # Presentation layer only: routes, dto, responses + regenerates bootstrap
puerto generate web <Name>                 # Web projects: the entity's frontend slice only (+ core/generated/*); --force overwrites
puerto generate use-case <Entity> <action> # Add a use case (refuses existing actions; adds the field to {Entity}Api)
puerto generate migration <name>           # Create a new SQLx migration file (--db projects only; versions never collide)
puerto generate bootstrap                  # Regenerate bootstrap.rs from puerto.toml (refuses on manifest/filesystem drift)
puerto generate bootstrap --allow-missing  # Write it anyway, even if a declared entity has no code
puerto generate snippets                   # Write IDE snippet files (Zed + VS Code)
puerto generate snippets --ide zed         # Zed only  (.zed/snippets/rust.json)
puerto generate snippets --ide vscode      # VS Code only (.vscode/puerto.code-snippets)
puerto validate                            # Validate puerto.toml (field types, names, reserved names, manifest/filesystem drift)
```

## Make Commands

```bash
make run           # Run puerto CLI (prompts for project name)
make test          # Fast structural tests (verifies generated file structure)
make test/full     # Slow: generates real projects, runs their `clippy --all-targets -D warnings` + tests
                   #       and the generated frontend's checks (needs `bun` on PATH)
make test/db       # Slow: a generated --db project against a throwaway Postgres + Mailpit (Docker + sqlx-cli)
make test/kafka    # Slow: the Kafka transport against a throwaway Redpanda (+ Postgres, Mailpit; Docker, cmake)
make test/mail     # Slow: the SMTP adapter against a throwaway Mailpit (Docker)
make test/auth     # Slow: the OIDC adapter against a throwaway Keycloak 26.8 (Docker)
make test/mutation # Slow: cargo-mutants on validation / migrations / types / patchers
make test/snapshots # Re-record generated-output snapshots after an intended output change
make lint          # cargo clippy --workspace --all-targets -- -D warnings
make check         # cargo check --workspace
make format        # cargo fmt --all -- --check
make format/fix    # cargo fmt --all
```

## Completion Checklist

- [ ] `make test` passes (structural tests — fast)
- [ ] `make test/full` passes (generated projects lint clean + internal tests pass — slow; needs
      `bun` on `PATH` for the web scenarios)
- [ ] `make test/db` passes when the change touches `--db` generation (Pg repository, migrations, SQL)
- [ ] Tier 1 change (templates, schema, bootstrap, migrations, patchers): diff read in full —
      see `.claude/rules/workflow.md` § Risk tiers
- [ ] One commit for this piece; a design decision with no trace in the code gets an ADR
- [ ] `make lint` clean (zero warnings)
- [ ] `make format/fix` run

## Generated project contract (what every project gets)

- Runtime crates instead of copies (ADR-0017): `puerto-core = "<cli minor>"` in the root
  `[workspace.dependencies]`, `{ workspace = true }` in each member; the domain imports
  `puerto_core::{RepositoryError, Page, PageRequest}`. `presentation` also depends on `puerto-web`:
  `main.rs` is `Config::from_env()` + `App::new(config).try_serve(build_app)`, `bootstrap.rs` hands its
  API and readiness check to `puerto_web::routes`, then to the user's `app::configure`. A `--db`
  project's `infrastructure` depends on `puerto-sqlx`: `db.rs` is `pub use puerto_sqlx::*;` plus
  the project's `MIGRATOR`
- Formatted: every command that writes Rust ends with `cargo fmt --all` in the project, so its
  `make format` passes; a failed format only warns (ADR-0019)
- A startup failure is a message, not a panic: `build_app` / `build_worker` return
  `Result<_, String>` (`?`, a hint per cause: `.env`, `make docker-compose/up`), `main` hands them
  to `try_serve` / `try_run`, which log the error and exit 1. `puerto_sqlx::DatabaseError` says
  `host:port/database` and the driver's cause, never the password
- Logging through `tracing` directly — no logger port (ADR-0006)
- Errors: `ErrorResponse { name, message }` with the code in `name`; validation → 422; one test per
  mapper arm (ADR-0007)
- `/health`, `/health/ready`, graceful shutdown on SIGTERM (ADR-0008)
- Pinned toolchain + edition 2024 (ADR-0009); root `AGENTS.md` + `CLAUDE.md` (ADR-0010)
- `web/` frontend unless `--no-web`: `[project] web = true`, `make web/*` targets (ADR-0013);
  `scaffold` writes a CRUD entity's screens (domain, HTTP repository, list/detail/form, tests) and
  regenerates `web/src/presentation/core/generated/{dependencies.ts,routes.tsx}`;
  `bootstrap.rs` exports the OpenAPI spec to `docs/api/openapi.json` from mocked use cases, and
  `make web/sync` turns it into the frontend's wire types
- Web forms say what went wrong next to the field: `domain/<entity>/messages.ts` (generated from
  the same codes as `errors.rs`) maps a code to a message and a field; `FieldError` shows it under
  that input, other codes go to the alert, unknown ones show as they are
- `--events db|kafka` (needs `--db`, ADR-0018): `[project] events`, the outbox migration (the
  CLI's `templates/infrastructure/outbox.sql`), per CRUD entity `events.rs` (AUTO-GENERATED) and
  `unit_of_work.rs`, a read-only repository port, writes through `Pg{Entity}UnitOfWorkFactory`,
  and a root `AGENTS.md` section saying `events.rs` is never hand-edited
- `--events` also writes the `worker/` member (ADR-0018 § 5): `main.rs` is
  `WorkerConfig::from_env()` + `Worker::new(config).try_run(build_worker)` (`puerto-events`),
  `generated/registry.rs` (AUTO-GENERATED from `[[consumer]]`), `consumers/<name>.rs` (the
  user's), `make worker`, `make events/dead-letters`, `WORKER_PORT`. On `db` the registry relays
  `PgOutbox` → `PgDeliveries`, one `PgSubscription` per consumer. On `kafka` it relays to
  `KafkaPublisher` (topics created at startup), one `KafkaSubscription` per consumer
  (`puerto-kafka`); the outbox migration has no `event_deliveries`, `docker-compose.yml` gains
  Redpanda and `.env` `KAFKA_BROKERS`
- `--auth oidc` (ADR-0021): `[project] auth`, written once: the `User` entity
  (`business/src/domain/user/`, in-memory or `PgUserRepository` over runtime-checked queries, the
  `users` migration with `UNIQUE (issuer, subject)`), `infrastructure/src/user/provisioning.rs`
  (`ProvisioningAuthenticator`: the provider, then find-or-create the user, a 60 s cache) and
  `GET /me` (`presentation/src/api/me/`). `bootstrap.rs` discovers the provider
  (`puerto_auth::OidcAuthenticator`, a startup error with a hint when it is down) and hands the
  routes `.data(Arc<dyn Authenticator>)`. Keycloak 26.8 in `docker-compose.yml` with
  `infrastructure/keycloak/realm.json` (client `web`, audience mapper `api`, user
  `admin@<project>.localhost`), `AUTH_*` in `.env`, `make auth/ui`, a root `AGENTS.md` section.
  The Greeting demo is `public`. `--auth local` is refused until 16.6
- `puerto generate mail <name>` (any project, ADR-0020): `business/src/domain/mail/<name>.rs`
  (`{Name}Mail` + the port `{Name}Mailer` + its mock), `infrastructure/src/mail/<name>.rs`
  (`Template{Name}Mailer` over `puerto-mail`, with render tests) and
  `infrastructure/mail/<name>/{subject.txt,body.html,body.txt}` — written once, the user's. The
  first mail adds `puerto-mail`, Mailpit to `docker-compose.yml` (created without `--db`),
  `SMTP_URL` / `MAIL_FROM`, `make mail/ui` and a root `AGENTS.md` section. An entity cannot be
  named `Mail`: mails own the `mail` modules. With `--on <event_type> --to <field>` (events
  projects) also `worker/src/consumers/mail_<name>.rs` (`Send{Name}Mail`: snapshot → mail →
  `puerto_mail::send_once`), `[[mail]] on`, the consumer `mail_<name>` in `registry.rs` (one
  `SmtpMailer` + one `PgSentMails`) and, the first time, the `sent_mails` migration (the CLI's
  `templates/infrastructure/sent_mails.sql`) and `puerto-mail` in `worker`

## Detailed Rules

- `.claude/rules/architecture.md` — Layer rules, naming conventions, error patterns
- `.claude/rules/puerto-toml.md` — puerto.toml schema + identifier derivation table
- `.claude/rules/testing.md` — TDD cycle, test naming, AAA pattern, DO/DON'T
- `.claude/rules/testing-conventions.md` — Object Mother pattern, mock setup, naming table
- `.claude/rules/workflow.md` — Puerto development workflow (Spec-Driven) + generated project TDD workflow
- `.claude/rules/release.md` — Versioning, changelog, release process, breaking changes policy, crates.io checklist
- `docs/adr/` — Architecture Decision Records
