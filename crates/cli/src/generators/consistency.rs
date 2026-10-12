//! Does `puerto.toml` describe the code that is actually on disk?
//!
//! `generated/bootstrap.rs` is written from the manifest alone: one `use` per entity, one per use
//! case, with no check that any of it exists. An entity declared but never scaffolded produced a
//! bootstrap full of E0433s — in the *generated* project, at `cargo build`, far from the command
//! that caused it.
//!
//! The list below is deliberately not "every file scaffold writes" (that is `conflict::entity_files`)
//! but exactly the files `bootstrap.rs` imports. Those are the ones whose absence does not compile.

use std::path::{Path, PathBuf};

use crate::generators::naming::pascal_to_snake;
use crate::puerto_toml::{Entity, PuertoConfig};

/// One entity whose declared code is not on disk.
#[derive(Debug)]
pub struct EntityDrift {
    pub entity: String,
    pub missing: Vec<PathBuf>,
}

/// The files `bootstrap.rs` will import for this entity.
pub fn bootstrap_required_files(entity: &Entity) -> Vec<PathBuf> {
    let snake = pascal_to_snake(&entity.name);
    let mut files = vec![
        PathBuf::from(format!("business/src/domain/{snake}/repository.rs")),
        PathBuf::from(format!("infrastructure/src/{snake}/repository.rs")),
        PathBuf::from(format!("presentation/src/api/{snake}/routes.rs")),
    ];
    files.extend(
        entity
            .use_cases
            .iter()
            .map(|uc| PathBuf::from(format!("business/src/application/{snake}/{uc}.rs"))),
    );
    files
}

/// Whether `base` looks like a generated project at all.
///
/// Without this, every unit test that writes a bare `puerto.toml` into a temp dir would report
/// every entity as drifted. A project with no `business/src/domain` has nothing to be
/// inconsistent *with*.
fn has_generated_tree(base: &Path) -> bool {
    base.join("business/src/domain").is_dir()
}

/// The files the generated code imports for this entity: `bootstrap.rs`'s, plus — in a web
/// project, for an entity with screens — those `web/src/presentation/core/generated/*` import.
pub fn required_files(entity: &Entity, web: bool) -> Vec<PathBuf> {
    let mut files = bootstrap_required_files(entity);
    if web && crate::generators::web::has_web_slice(entity) {
        files.extend(crate::generators::web::web_required_files(
            &pascal_to_snake(&entity.name),
        ));
    }
    files
}

/// Entities declared in the manifest whose code is missing.
pub fn missing_entity_files(base: &Path, config: &PuertoConfig) -> Vec<EntityDrift> {
    if !has_generated_tree(base) {
        return vec![];
    }
    config
        .entity
        .iter()
        .filter_map(|entity| {
            let missing: Vec<PathBuf> = required_files(entity, config.project.web)
                .into_iter()
                .filter(|f| !base.join(f).exists())
                .collect();
            (!missing.is_empty()).then(|| EntityDrift {
                entity: entity.name.clone(),
                missing,
            })
        })
        .collect()
}

/// One line per `[[consumer]]` whose handler is not on disk: the worker's `registry.rs` imports
/// it. Empty in a project without `worker/`.
pub fn missing_consumer_files(base: &Path, config: &PuertoConfig) -> Vec<String> {
    if !base.join("worker/src").is_dir() {
        return vec![];
    }
    config
        .consumer
        .iter()
        .filter(|c| {
            !base
                .join(format!("worker/src/consumers/{}.rs", c.name))
                .exists()
        })
        .map(|c| {
            format!(
                "consumer '{0}': worker/src/consumers/{0}.rs is missing — remove its [[consumer]] \
                 block and run `puerto generate consumer {0} --on {1}`",
                c.name,
                c.events.join(",")
            )
        })
        .collect()
}

/// One line per file of a `[[mail]]` that is not on disk: the mail does not compile without it.
pub fn missing_mail_files(base: &Path, config: &PuertoConfig) -> Vec<String> {
    let mut lines = vec![];
    for mail in &config.mail {
        let name = &mail.name;
        let mut paths = vec![
            format!("business/src/domain/mail/{name}.rs"),
            format!("infrastructure/src/mail/{name}.rs"),
            format!("infrastructure/mail/{name}/subject.txt"),
            format!("infrastructure/mail/{name}/body.html"),
        ];
        // Sent on events: the worker's registry imports its consumer.
        if !mail.on.is_empty() {
            paths.push(format!("worker/src/consumers/mail_{name}.rs"));
        }
        for path in paths {
            if !base.join(&path).exists() {
                lines.push(format!(
                    "mail '{name}': {path} is missing — restore it, or remove the [[mail]] block \
                     and the mail's other files, then run `puerto generate mail {name}`"
                ));
            }
        }
    }
    lines
}

/// Domain modules on disk with no `[[entity]]` block — the inverse drift. Not an error: the code
/// compiles, it is just invisible to every Puerto command.
pub fn orphan_entity_dirs(base: &Path, config: &PuertoConfig) -> Vec<String> {
    if !has_generated_tree(base) {
        return vec![];
    }
    let declared: Vec<String> = config
        .entity
        .iter()
        .map(|e| pascal_to_snake(&e.name))
        .collect();

    let Ok(entries) = std::fs::read_dir(base.join("business/src/domain")) else {
        return vec![];
    };

    let mut orphans: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        // `shared` holds the shared value objects and `mail` the mails: neither is an entity. An
        // auth project's `user` is written once by `puerto new --auth`, outside the manifest.
        .filter(|name| name != "shared" && name != "mail" && !declared.contains(name))
        .filter(|name| !(config.project.has_auth() && name == "user"))
        .collect();
    orphans.sort();
    orphans
}

/// The command that fixes this drift.
///
/// Mid-way through the domain-first workflow the domain layer is already written and only the
/// later ones are missing — telling the user to run `scaffold` there is wrong twice over: it would
/// refuse (the files exist) and, forced, it would overwrite the domain they just wrote.
fn suggested_command(entity: &str, missing: &[PathBuf]) -> String {
    let is_missing = |needle: &str| missing.iter().any(|p| p.to_string_lossy().contains(needle));

    if is_missing("business/src/domain") {
        format!("puerto generate scaffold {entity}")
    } else if is_missing("business/src/application") {
        format!("puerto generate application {entity}")
    } else if is_missing("infrastructure/src") {
        format!("puerto generate repository {entity}")
    } else if is_missing("presentation/src") {
        format!("puerto generate presentation {entity}")
    } else {
        format!("puerto generate web {entity}")
    }
}

/// One human-readable line per drifted entity, for error and warning output.
pub fn drift_lines(drifts: &[EntityDrift]) -> Vec<String> {
    drifts
        .iter()
        .map(|d| {
            let paths: Vec<String> = d.missing.iter().map(|p| p.display().to_string()).collect();
            format!(
                "entity '{}' is in puerto.toml but its code is missing ({}). Run `{}`",
                d.entity,
                paths.join(", "),
                suggested_command(&d.entity, &d.missing)
            )
        })
        .collect()
}

/// One handler of an `#[OpenApi] impl`, as text: its doc comments and attributes, then its
/// signature and body.
struct Handler<'a> {
    attributes: Vec<&'a str>,
    signature: String,
    body: Vec<&'a str>,
}

/// The handlers of the `#[OpenApi] impl` in a rustfmt-formatted `routes.rs`: items indented four
/// spaces, from `#[OpenApi]` to the `}` that closes the impl.
fn handlers(routes: &str) -> Vec<Handler<'_>> {
    let mut lines = routes
        .lines()
        .skip_while(|line| line.trim() != "#[OpenApi]");
    lines.next();
    let mut handlers = vec![];
    let mut attributes = vec![];
    let mut current: Option<Handler> = None;
    for line in lines.take_while(|line| *line != "}") {
        let item = line
            .strip_prefix("    ")
            .filter(|rest| !rest.starts_with(' '));
        match item {
            Some(rest) if rest.starts_with("///") || rest.starts_with("#[") => {
                handlers.extend(current.take());
                attributes.push(rest);
            }
            Some(rest) if rest.starts_with("async fn ") || rest.starts_with("fn ") => {
                handlers.extend(current.take());
                current = Some(Handler {
                    attributes: std::mem::take(&mut attributes),
                    signature: rest.to_string(),
                    body: vec![],
                });
            }
            _ => {
                if let Some(handler) = current.as_mut() {
                    // A signature rustfmt split over lines ends at the line that opens the body.
                    if !handler.signature.trim_end().ends_with('{') {
                        handler.signature.push_str(line.trim());
                    } else {
                        handler.body.push(line);
                    }
                }
            }
        }
    }
    handlers.extend(current);
    handlers
}

impl Handler<'_> {
    /// Whether the body calls use case `uc`: `self.{uc}.execute(…)`, on one line or split by
    /// rustfmt (`self\n.{uc}\n.execute(`).
    fn calls(&self, uc: &str) -> bool {
        let joined: String = self.body.iter().map(|line| line.trim()).collect();
        joined.contains(&format!(".{uc}.execute("))
    }

    fn takes_bearer_auth(&self) -> bool {
        self.signature.contains("BearerAuth")
    }

    fn requires_role(&self, role: &str) -> bool {
        let attribute = format!("#[require_role(\"{role}\")]");
        self.attributes.iter().any(|line| line.trim() == attribute)
    }
}

/// Auth projects: one line per use case whose handler in `routes.rs` does not match what
/// `puerto.toml` says of it — a `public` route that takes `BearerAuth`, a protected one that does
/// not, a role in `roles` with no `#[require_role]`. Textual: a use case whose handler is not
/// found (renamed, moved) is not checked.
pub fn auth_drift(base: &Path, config: &PuertoConfig) -> Vec<String> {
    if !config.project.has_auth() {
        return vec![];
    }
    let mut lines = vec![];
    for entity in &config.entity {
        let path = format!(
            "presentation/src/api/{}/routes.rs",
            pascal_to_snake(&entity.name)
        );
        let Ok(routes) = std::fs::read_to_string(base.join(&path)) else {
            continue;
        };
        let handlers = handlers(&routes);
        for uc in &entity.use_cases {
            let Some(handler) = handlers.iter().find(|h| h.calls(uc)) else {
                continue;
            };
            let public = entity.public.contains(uc);
            if public && handler.takes_bearer_auth() {
                lines.push(format!(
                    "entity '{}': '{uc}' is public in puerto.toml, but its handler in {path} \
                     takes BearerAuth",
                    entity.name
                ));
            }
            if !public && !handler.takes_bearer_auth() {
                lines.push(format!(
                    "entity '{}': '{uc}' is protected in puerto.toml, but its handler in {path} \
                     takes no BearerAuth — anyone can call it. Add `auth: BearerAuth`, or list it \
                     in `public`",
                    entity.name
                ));
            }
            for role in entity.roles.get(uc).into_iter().flatten() {
                if !handler.requires_role(role) {
                    lines.push(format!(
                        "entity '{}': '{uc}' requires the role '{role}' in puerto.toml, but its \
                         handler in {path} has no #[require_role(\"{role}\")]",
                        entity.name
                    ));
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_files_cover_every_use_case() {
        let entity = Entity {
            name: "OrderItem".into(),
            use_cases: vec!["create_order_item".into(), "list_order_item".into()],
            db: false,
            fields: vec![],
            ..Default::default()
        };
        let files = bootstrap_required_files(&entity);

        assert!(files.contains(&PathBuf::from(
            "business/src/application/order_item/create_order_item.rs"
        )));
        assert!(files.contains(&PathBuf::from(
            "business/src/application/order_item/list_order_item.rs"
        )));
        assert!(files.contains(&PathBuf::from("presentation/src/api/order_item/routes.rs")));
    }

    #[test]
    fn suggests_scaffold_when_nothing_exists_and_the_next_layer_otherwise() {
        let all = vec![
            PathBuf::from("business/src/domain/invoice/repository.rs"),
            PathBuf::from("infrastructure/src/invoice/repository.rs"),
        ];
        assert_eq!(
            suggested_command("Invoice", &all),
            "puerto generate scaffold Invoice"
        );

        // Domain already written (not in the missing list) — the domain-first flow's next step.
        let mid_flow = vec![
            PathBuf::from("business/src/application/invoice/create_invoice.rs"),
            PathBuf::from("infrastructure/src/invoice/repository.rs"),
        ];
        assert_eq!(
            suggested_command("Invoice", &mid_flow),
            "puerto generate application Invoice"
        );

        let only_presentation = vec![PathBuf::from("presentation/src/api/invoice/routes.rs")];
        assert_eq!(
            suggested_command("Invoice", &only_presentation),
            "puerto generate presentation Invoice"
        );

        let only_web = vec![PathBuf::from("web/src/domain/invoice/repository.ts")];
        assert_eq!(
            suggested_command("Invoice", &only_web),
            "puerto generate web Invoice"
        );
    }

    #[test]
    fn a_directory_without_a_generated_tree_never_drifts() {
        let config = PuertoConfig {
            project: crate::puerto_toml::Project {
                name: "x".into(),
                db: false,
                web: false,
                events: None,
                auth: None,
            },
            entity: vec![Entity {
                name: "Ghost".into(),
                use_cases: vec![],
                db: false,
                fields: vec![],
                ..Default::default()
            }],
            value_object: vec![],
            consumer: vec![],
            mail: vec![],
        };
        let dir = std::env::temp_dir();
        assert!(missing_entity_files(&dir, &config).is_empty());
    }
    const ROUTES: &str = r#"use std::sync::Arc;

#[OpenApi]
impl ProductApi {
    /// Create a new Product
    #[oai(path = "/products", method = "post", tag = "ApiTags::Product")]
    #[require_role("admin")]
    async fn create(
        &self,
        auth: BearerAuth,
        body: Json<CreateProductRequest>,
    ) -> CreateProductResponse {
        match self
            .create_product
            .execute(CreateProductParams {
                actor: auth.0,
            })
            .await
        {
            Ok(entity) => todo!(),
        }
    }

    /// List Products
    #[oai(path = "/products", method = "get", tag = "ApiTags::Product")]
    async fn list(&self, limit: Query<Option<u32>>) -> ListProductsResponse {
        match self.list_products.execute(ListProductsParams { page }).await {
            Ok(page) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn should_create() {
        let response = api.create_product.execute(params).await;
    }
}
"#;

    #[test]
    fn should_read_each_handler_with_its_attributes_and_signature() {
        let handlers = handlers(ROUTES);

        assert_eq!(handlers.len(), 2);
        assert!(handlers[0].calls("create_product"));
        assert!(handlers[0].takes_bearer_auth());
        assert!(handlers[0].requires_role("admin"));
        assert!(!handlers[0].requires_role("editor"));
        assert!(handlers[1].calls("list_products"));
        assert!(!handlers[1].calls("create_product"));
        assert!(!handlers[1].takes_bearer_auth());
    }

    fn auth_project(dir: &Path, entity: Entity) -> PuertoConfig {
        std::fs::create_dir_all(dir.join("presentation/src/api/product")).unwrap();
        std::fs::write(dir.join("presentation/src/api/product/routes.rs"), ROUTES).unwrap();
        PuertoConfig {
            project: crate::puerto_toml::Project {
                name: "x".into(),
                auth: Some("oidc".into()),
                ..Default::default()
            },
            entity: vec![entity],
            value_object: vec![],
            consumer: vec![],
            mail: vec![],
        }
    }

    fn product(public: &[&str], roles: &[(&str, &str)]) -> Entity {
        Entity {
            name: "Product".into(),
            use_cases: vec!["create_product".into(), "list_products".into()],
            public: public.iter().map(|s| s.to_string()).collect(),
            roles: roles
                .iter()
                .map(|(uc, role)| (uc.to_string(), vec![role.to_string()]))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn should_find_no_auth_drift_when_routes_match_the_manifest() {
        let dir = std::env::temp_dir().join("puerto_auth_drift_ok");
        let config = auth_project(
            &dir,
            product(&["list_products"], &[("create_product", "admin")]),
        );

        assert_eq!(auth_drift(&dir, &config), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn should_report_auth_drift_when_routes_disagree_with_the_manifest() {
        let dir = std::env::temp_dir().join("puerto_auth_drift_bad");
        // create_product listed public (but takes BearerAuth), list_products protected (but takes
        // none), and a role the handler does not require.
        let config = auth_project(
            &dir,
            product(&["create_product"], &[("list_products", "editor")]),
        );

        let lines = auth_drift(&dir, &config);

        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines[0].contains("'create_product' is public in puerto.toml"),
            "{lines:?}"
        );
        assert!(
            lines[1].contains("'list_products' is protected"),
            "{lines:?}"
        );
        assert!(
            lines[2].contains("#[require_role(\"editor\")]"),
            "{lines:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn should_not_check_auth_drift_when_the_project_has_no_auth() {
        let dir = std::env::temp_dir().join("puerto_auth_drift_off");
        let mut config = auth_project(&dir, product(&["create_product"], &[]));
        config.project.auth = None;

        assert_eq!(auth_drift(&dir, &config), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
