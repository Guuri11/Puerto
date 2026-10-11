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
