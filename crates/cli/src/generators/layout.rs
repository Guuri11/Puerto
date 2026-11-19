//! Where each piece of a generated project lives. Generators, patchers and checks ask this module
//! for every project path instead of spelling it. Paths are relative to the project root.
//!
//! One crate per context and layer (ADR-0023): `contextos/<c>/{dominio,aplicacion,
//! infraestructura,rest}`, plus `nucleo`, `migrador` and the binaries under `presentacion/`. In v1
//! a context is one entity, named after its snake case name.

// ── workspace ─────────────────────────────────────────────────────────────────────────────────
pub const WORKSPACE_MANIFEST: &str = "Cargo.toml";
pub const CONTEXTS_DIR: &str = "contextos";

// ── nucleo: the shared value objects ──────────────────────────────────────────────────────────
pub const NUCLEO_DIR: &str = "nucleo";
pub const NUCLEO_SRC: &str = "nucleo/src";

// ── the frontend (`[proyecto] web`) ───────────────────────────────────────────────────────────
pub const WEB_DIR: &str = "web";

// ── migrations (`--db`) ───────────────────────────────────────────────────────────────────────
pub const MIGRATIONS_DIR: &str = "migraciones";
pub const MIGRADOR_DIR: &str = "migrador";

// ── the four crates of a context ──────────────────────────────────────────────────────────────

/// A layer of a context, and the crate it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Domain,
    Application,
    Infrastructure,
    Rest,
}

impl Layer {
    pub const ALL: [Layer; 4] = [
        Layer::Domain,
        Layer::Application,
        Layer::Infrastructure,
        Layer::Rest,
    ];

    /// The directory under `contextos/<c>/`.
    pub fn dir(self) -> &'static str {
        match self {
            Layer::Domain => "dominio",
            Layer::Application => "aplicacion",
            Layer::Infrastructure => "infraestructura",
            Layer::Rest => "rest",
        }
    }

    /// The crate name's prefix: `dominio_product`, `infra_product`.
    pub fn prefix(self) -> &'static str {
        match self {
            Layer::Domain => "dominio",
            Layer::Application => "aplicacion",
            Layer::Infrastructure => "infra",
            Layer::Rest => "rest",
        }
    }
}

pub fn context_dir(c: &str) -> String {
    format!("{CONTEXTS_DIR}/{c}")
}

/// `dominio_product`: the crate's name, also its path in Rust code.
pub fn crate_name(c: &str, layer: Layer) -> String {
    format!("{}_{c}", layer.prefix())
}

pub fn crate_dir(c: &str, layer: Layer) -> String {
    format!("{CONTEXTS_DIR}/{c}/{}", layer.dir())
}

pub fn crate_manifest(c: &str, layer: Layer) -> String {
    format!("{}/Cargo.toml", crate_dir(c, layer))
}

pub fn crate_lib(c: &str, layer: Layer) -> String {
    format!("{}/src/lib.rs", crate_dir(c, layer))
}

/// `module` without `.rs`: `crate_file("product", Layer::Domain, "modelo")`.
pub fn crate_file(c: &str, layer: Layer, module: &str) -> String {
    format!("{}/src/{module}.rs", crate_dir(c, layer))
}

/// `modelo`, `errores`, `repositorio`, `valores`, `eventos`, `unit_of_work`.
pub fn domain_file(c: &str, module: &str) -> String {
    crate_file(c, Layer::Domain, module)
}

/// The use case's whole file: params, trait, double, implementation and tests (ADR-0023 § 4).
pub fn use_case(c: &str, uc: &str) -> String {
    crate_file(c, Layer::Application, uc)
}

/// The Object Mother, behind the domain crate's `pruebas` feature.
pub fn mother(c: &str) -> String {
    format!("{}/src/pruebas/madres.rs", crate_dir(c, Layer::Domain))
}

/// `repositorio`, `fila`, `unit_of_work`.
pub fn infra_file(c: &str, module: &str) -> String {
    crate_file(c, Layer::Infrastructure, module)
}

/// `rutas`, `dto`, `respuestas`, `errores`.
pub fn rest_file(c: &str, module: &str) -> String {
    crate_file(c, Layer::Rest, module)
}

// ── mail: the `correo` context (domain and infrastructure, no application) ────────────────────
pub const MAIL_CONTEXT: &str = "correo";

pub fn mail_port(name: &str) -> String {
    crate_file(MAIL_CONTEXT, Layer::Domain, name)
}

pub fn mail_adapter(name: &str) -> String {
    crate_file(MAIL_CONTEXT, Layer::Infrastructure, name)
}

/// `asunto.txt`, `cuerpo.html` and `cuerpo.txt` of a mail, next to its adapter.
pub fn mail_templates_dir(name: &str) -> String {
    format!(
        "{}/plantillas/{name}",
        crate_dir(MAIL_CONTEXT, Layer::Infrastructure)
    )
}

// ── auth: the `usuario` context ───────────────────────────────────────────────────────────────
pub const USER_CONTEXT: &str = "usuario";
pub const KEYCLOAK_REALM: &str = "keycloak/realm.json";

// ── presentacion ──────────────────────────────────────────────────────────────────────────────
/// The generated composition crate: repositories, units of work and use cases (ADR-0023 § 7).
pub const CABLEADO_MANIFEST: &str = "presentacion/cableado/Cargo.toml";
pub const CABLEADO_LIB: &str = "presentacion/cableado/src/lib.rs";

pub const API_MANIFEST: &str = "presentacion/api/Cargo.toml";
pub const BOOTSTRAP_RS: &str = "presentacion/api/src/generado/bootstrap.rs";

// ── worker ────────────────────────────────────────────────────────────────────────────────────
pub const WORKER_DIR: &str = "presentacion/worker";
pub const WORKER_MANIFEST: &str = "presentacion/worker/Cargo.toml";
pub const WORKER_SRC: &str = "presentacion/worker/src";
pub const WORKER_GENERATED_DIR: &str = "presentacion/worker/src/generado";
pub const REGISTRY_RS: &str = "presentacion/worker/src/generado/registro.rs";
pub const CONSUMERS_RS: &str = "presentacion/worker/src/consumidores.rs";

pub fn consumer(name: &str) -> String {
    format!("{WORKER_SRC}/consumidores/{name}.rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_each_crate_after_its_layer_and_context() {
        let names: Vec<String> = Layer::ALL
            .iter()
            .map(|l| crate_name("order_item", *l))
            .collect();
        assert_eq!(
            names,
            [
                "dominio_order_item",
                "aplicacion_order_item",
                "infra_order_item",
                "rest_order_item"
            ]
        );
        assert_eq!(
            crate_dir("order_item", Layer::Infrastructure),
            "contextos/order_item/infraestructura"
        );
    }
}
