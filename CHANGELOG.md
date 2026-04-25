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
