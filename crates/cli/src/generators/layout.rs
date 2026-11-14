//! Where each piece of a generated project lives (Phase 19.3). Generators, patchers and checks ask
//! this module for every project path instead of spelling it, so Phase 19.6 (ADR-0023) moves the
//! structure here — and the snapshots show what moved. Paths are relative to the project root.

// ── business ──────────────────────────────────────────────────────────────────────────────────
pub const BUSINESS_MANIFEST: &str = "business/Cargo.toml";
pub const BUSINESS_SRC: &str = "business/src";
pub const BUSINESS_LIB: &str = "business/src/lib.rs";
pub const DOMAIN_DIR: &str = "business/src/domain";
pub const APPLICATION_DIR: &str = "business/src/application";
pub const SHARED_DIR: &str = "business/src/domain/shared";
pub const SHARED_RS: &str = "business/src/domain/shared.rs";

pub fn domain_dir(snake: &str) -> String {
    format!("{DOMAIN_DIR}/{snake}")
}

/// `module` without `.rs`: `domain_file("product", "model")`.
pub fn domain_file(snake: &str, module: &str) -> String {
    format!("{DOMAIN_DIR}/{snake}/{module}.rs")
}

pub fn use_case_contract(snake: &str, uc: &str) -> String {
    format!("{DOMAIN_DIR}/{snake}/use_cases/{uc}.rs")
}

pub fn application_dir(snake: &str) -> String {
    format!("{APPLICATION_DIR}/{snake}")
}

pub fn use_case_impl(snake: &str, uc: &str) -> String {
    format!("{APPLICATION_DIR}/{snake}/{uc}.rs")
}

pub fn mother(snake: &str) -> String {
    format!("{BUSINESS_SRC}/tests/mothers/{snake}_mother.rs")
}

pub fn mail_port(name: &str) -> String {
    format!("{DOMAIN_DIR}/mail/{name}.rs")
}

// ── infrastructure ────────────────────────────────────────────────────────────────────────────
pub const INFRA_MANIFEST: &str = "infrastructure/Cargo.toml";
pub const INFRA_SRC: &str = "infrastructure/src";
pub const INFRA_LIB: &str = "infrastructure/src/lib.rs";
pub const DB_RS: &str = "infrastructure/src/db.rs";
pub const MIGRATIONS_DIR: &str = "infrastructure/migrations";
pub const KEYCLOAK_REALM: &str = "infrastructure/keycloak/realm.json";

pub fn infra_dir(snake: &str) -> String {
    format!("{INFRA_SRC}/{snake}")
}

/// `module` without `.rs`: `infra_file("product", "repository")`.
pub fn infra_file(snake: &str, module: &str) -> String {
    format!("{INFRA_SRC}/{snake}/{module}.rs")
}

pub fn mail_adapter(name: &str) -> String {
    format!("{INFRA_SRC}/mail/{name}.rs")
}

/// `subject.txt`, `body.html` and `body.txt` of a mail.
pub fn mail_templates_dir(name: &str) -> String {
    format!("infrastructure/mail/{name}")
}

// ── presentation ──────────────────────────────────────────────────────────────────────────────
pub const PRESENTATION_MANIFEST: &str = "presentation/Cargo.toml";
pub const PRESENTATION_SRC: &str = "presentation/src";
pub const API_DIR: &str = "presentation/src/api";
pub const API_RS: &str = "presentation/src/api.rs";
pub const TAGS_RS: &str = "presentation/src/api/tags.rs";
pub const BOOTSTRAP_RS: &str = "presentation/src/generated/bootstrap.rs";

/// The module declaration file, `api/<snake>.rs`, next to the `api/<snake>/` directory.
pub fn api_module(snake: &str) -> String {
    format!("{API_DIR}/{snake}.rs")
}

pub fn api_dir(snake: &str) -> String {
    format!("{API_DIR}/{snake}")
}

/// `module` without `.rs`: `api_file("product", "routes")`.
pub fn api_file(snake: &str, module: &str) -> String {
    format!("{API_DIR}/{snake}/{module}.rs")
}

// ── worker ────────────────────────────────────────────────────────────────────────────────────
pub const WORKER_MANIFEST: &str = "worker/Cargo.toml";
pub const WORKER_SRC: &str = "worker/src";
pub const WORKER_GENERATED_DIR: &str = "worker/src/generated";
pub const REGISTRY_RS: &str = "worker/src/generated/registry.rs";
pub const CONSUMERS_RS: &str = "worker/src/consumers.rs";

pub fn consumer(name: &str) -> String {
    format!("{WORKER_SRC}/consumers/{name}.rs")
}
