## [0.4.0] - 2026-04-27

### 🚀 Features

- Generated projects now print a Harbor ASCII banner on startup (set `HARBOR_BANNER=false` to suppress)
- HTTP request logging via poem's `Tracing` middleware — every request/response is traced automatically
- `InMemoryRepository` receives the project logger; repository-level operations are now logged at `debug`
- `GreetingApi` (and all generated `*Api` structs) receive the logger; errors are logged at `warn`
- `uuid` and `chrono` template dependencies now include `serde` features for JSON serialisation out of the box
- `poem-openapi` template dependency now includes `uuid` feature for typed UUID path/query params

### 💥 Breaking

- `InMemoryGreetingRepository` is no longer a unit struct — constructor is now `InMemoryGreetingRepository::new(logger: Arc<dyn LoggerTrait>)`. Update `bootstrap.rs` in existing projects (or run `harbor generate bootstrap`).
- `GreetingApi` (and all scaffolded `*Api` structs) now require a `logger: Arc<dyn LoggerTrait>` field. Update `bootstrap.rs` in existing projects.
- `build_app()` in `generated/bootstrap.rs` is now `async` and returns `impl poem::Endpoint` (was `Route`). Update `main.rs` to `generated::bootstrap::build_app().await`.

### 🐛 Bug Fixes

- `harbor new --db` now correctly writes `project.db = true` to `harbor.toml` — was silently omitted, causing `harbor generate scaffold --db` to not detect db mode correctly

### ⚙️ Miscellaneous Tasks

- Test coverage: add `db_project_harbor_toml_has_project_db_true` and `no_db_project_harbor_toml_omits_project_db` structural tests

## [0.3.0] - 2026-04-25

### 🚀 Features

- Add `harbor generate snippets [--ide zed|vscode]` command — writes TextMate-format snippet files for Zed (`.zed/snippets/rust.json`) and VS Code (`.vscode/harbor.code-snippets`), compatible with nvim+LuaSnip
- `harbor new` now automatically writes snippet files to both IDEs on project creation
- 23 Harbor-adapted snippets covering all DDD layers: domain model, errors, repository trait, use case trait/impl, persistence entity/repo, tokio test, sqlx integration test, Object Mother, and more

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
- Add `--no-db` flag to `harbor new` (explicit opt-out without prompting)
- Add `--destination` flag to `harbor new` (controls output directory)
- Generate `docker-compose.yml` and `.env` for `--db` projects
- Add db Makefile targets: `docker/up`, `sqlx/migrate`, `sqlx/prepare`, etc.
- Run migrations automatically on bootstrap startup (`run_migrations`)
- Add `uuid`, `chrono`, `serde`, `tracing`, `dotenvy` to template dependencies

### 💥 Breaking

- All generated `UseCaseImpl` structs now require a `logger: Arc<dyn LoggerTrait>` field. Existing projects must add this field manually and update `bootstrap.rs` (or run `harbor generate bootstrap`).

### 🐛 Bug Fixes

- Correct repo URL, install command, and Rust version in landing

### 🚜 Refactor

- Remove redundant crates/template, clean CLI output

### 📚 Documentation

- Update release.md crate name and AGENTS.md make commands
- Fix install command in README (harbor-framework)

### ⚙️ Miscellaneous Tasks

- Add CI and release GitHub Actions workflows

## [0.1.0] - 2026-04-12

### 🚀 Features

- Auto-wire DI via harbor.toml + generated bootstrap

### 🐛 Bug Fixes

- Rename template Cargo.toml to .liquid to fix crates.io packaging

### ⚙️ Miscellaneous Tasks

- Rename crate to harbor-framework for crates.io
