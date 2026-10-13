//! Name and shape validation shared by the CLI parser and `puerto validate`.
//!
//! These two used to diverge: `parse_field_arg` checked snake_case and nothing else, while
//! `run_validate` re-implemented its own predicates. Anything they both missed — a field called
//! `id`, a field called `type`, an entity called `Error` — reached the generators and produced
//! Rust that does not compile, with the error surfacing in the *generated* project rather than at
//! the command that caused it.
//!
//! Everything here is a pure name predicate. Type-registry checks live in `generators::types`.

use crate::puerto_toml::{Field, ValueObjectDefinition};

/// Fields every generated entity already has. A `[[entity.fields]]` entry with one of these names
/// emits the struct field twice (E0124).
pub const SYSTEM_FIELDS: &[&str] = &["id", "created_at", "updated_at", "deleted", "deleted_at"];

/// Rust keywords, strict and reserved. Field names become struct fields and entity names become
/// module names, so either can land on one.
pub const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "abstract", "become", "box", "do", "final", "gen",
    "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// PostgreSQL reserved key words (the "reserved" and "reserved (can be function or type)"
/// categories of the PostgreSQL key word table). Field names become unquoted column names in the
/// generated migration and queries, where any of these is a syntax error — `desc TEXT` broke
/// `sqlx migrate run`. Checked only in `--db` projects.
pub const SQL_RESERVED_WORDS: &[&str] = &[
    "all",
    "analyse",
    "analyze",
    "and",
    "any",
    "array",
    "as",
    "asc",
    "asymmetric",
    "authorization",
    "binary",
    "both",
    "case",
    "cast",
    "check",
    "collate",
    "collation",
    "column",
    "concurrently",
    "constraint",
    "create",
    "cross",
    "current_catalog",
    "current_date",
    "current_role",
    "current_schema",
    "current_time",
    "current_timestamp",
    "current_user",
    "default",
    "deferrable",
    "desc",
    "distinct",
    "do",
    "else",
    "end",
    "except",
    "false",
    "fetch",
    "for",
    "foreign",
    "freeze",
    "from",
    "full",
    "grant",
    "group",
    "having",
    "ilike",
    "in",
    "initially",
    "inner",
    "intersect",
    "into",
    "is",
    "isnull",
    "join",
    "lateral",
    "leading",
    "left",
    "like",
    "limit",
    "localtime",
    "localtimestamp",
    "natural",
    "not",
    "notnull",
    "null",
    "offset",
    "on",
    "only",
    "or",
    "order",
    "outer",
    "overlaps",
    "placing",
    "primary",
    "references",
    "returning",
    "right",
    "select",
    "session_user",
    "similar",
    "some",
    "symmetric",
    "system_user",
    "table",
    "tablesample",
    "then",
    "to",
    "trailing",
    "true",
    "union",
    "unique",
    "user",
    "using",
    "variadic",
    "verbose",
    "when",
    "where",
    "window",
    "with",
];

/// Inner types a shared `[[value_object]]` may wrap.
pub const SHARED_VO_ALLOWED_INNER_TYPES: &[&str] =
    &["String", "i64", "bool", "f64", "Uuid", "DateTime<Utc>"];

/// Type names in scope in generated code. An entity named after one of these produces a struct
/// that shadows it in the same file.
const RESERVED_TYPE_NAMES: &[&str] = &[
    "Self", "String", "Option", "Vec", "Box", "Result", "HashMap", "Uuid", "DateTime", "Utc",
    "Arc", "Error",
];

/// Module names the generated project already occupies. Entity names become module names
/// (`business/src/domain/{snake}/`, `presentation/src/api/{snake}/`).
const RESERVED_MODULE_NAMES: &[&str] = &[
    "shared",  // business/src/domain/shared — shared value objects
    "tracing", // infrastructure declares entity modules at its crate root: `tracing::` would
    // then be ambiguous with the crate every generated layer logs through (E0659)
    "db",        // infrastructure/src/db.rs (`--db` projects)
    "tags",      // presentation/src/api/tags.rs
    "error",     // presentation/src/api/error.rs
    "generated", // presentation/src/generated
    "api",       // presentation/src/api.rs
    "mocks",     // repository.rs mock module
    "tests",     // business/src/tests
    "mail",      // business/src/domain/mail, infrastructure/src/mail — the project's mails
];

/// snake_case: lowercase letters, digits and underscores, starting with a letter.
///
/// A leading `_` is rejected: the docs and the error message both say "not starting with a digit,
/// snake_case", and a leading underscore means "deliberately unused" to every Rust reader.
pub fn is_valid_field_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// PascalCase: an uppercase letter followed by alphanumerics.
pub fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// Full check for a `[[entity.fields]]` name.
pub fn validate_field_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("field name is empty".to_string());
    }
    if !is_valid_field_name(name) {
        return Err(format!(
            "invalid field name '{name}'. Must be snake_case (lowercase letters, digits and \
             underscores, starting with a lowercase letter)"
        ));
    }
    if SYSTEM_FIELDS.contains(&name) {
        return Err(format!(
            "field name '{name}' is reserved — every entity already has {}. Remove it from the \
             field list",
            SYSTEM_FIELDS.join(", ")
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "field name '{name}' is a Rust keyword and cannot be a struct field. Pick another name"
        ));
    }
    Ok(())
}

/// A field name that is also a PostgreSQL reserved word — only an error in a `--db` project.
pub fn validate_sql_field_name(name: &str) -> Result<(), String> {
    if SQL_RESERVED_WORDS.contains(&name) {
        return Err(format!(
            "field name '{name}' is a reserved SQL keyword and cannot be a column name. Pick \
             another name (e.g. '{name}_value')"
        ));
    }
    Ok(())
}

/// Two fields with the same name are one struct field defined twice (E0124).
pub fn validate_unique_field_names(fields: &[Field]) -> Result<(), String> {
    let mut seen: Vec<&str> = vec![];
    for f in fields {
        if seen.contains(&f.name.as_str()) {
            return Err(format!(
                "duplicate field name '{}' — every field must be distinct",
                f.name
            ));
        }
        seen.push(&f.name);
    }
    Ok(())
}

/// A use case action becomes a file and module name, and is PascalCased into type names.
pub fn validate_use_case_name(action: &str) -> Result<(), String> {
    if !is_valid_field_name(action) {
        return Err(format!(
            "invalid use case name '{action}'. Must be snake_case (lowercase letters, digits and \
             underscores, starting with a lowercase letter), e.g. 'archive_product'"
        ));
    }
    if RUST_KEYWORDS.contains(&action) {
        return Err(format!(
            "use case name '{action}' is a Rust keyword. Pick another name"
        ));
    }
    Ok(())
}

/// A `[[consumer]]` name becomes a module (`worker/src/consumers/<name>.rs`), a type
/// (`NotifyNewProduct`) and the `db` transport's consumer column.
pub fn validate_consumer_name(name: &str) -> Result<(), String> {
    if !is_valid_field_name(name) {
        return Err(format!(
            "invalid consumer name '{name}'. Must be snake_case (lowercase letters, digits and \
             underscores, starting with a lowercase letter), e.g. 'notify_new_product'"
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "consumer name '{name}' is a Rust keyword. Pick another name"
        ));
    }
    Ok(())
}

/// A `[[mail]]` name becomes two modules (`domain::mail::<name>`, `mail::<name>`), a template
/// folder and the types `{Name}Mail` / `{Name}Mailer`.
pub fn validate_mail_name(name: &str) -> Result<(), String> {
    if !is_valid_field_name(name) {
        return Err(format!(
            "invalid mail name '{name}'. Must be snake_case (lowercase letters, digits and \
             underscores, starting with a lowercase letter), e.g. 'welcome'"
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "mail name '{name}' is a Rust keyword. Pick another name"
        ));
    }
    Ok(())
}

/// The inner type of a shared `[[value_object]]`.
pub fn validate_shared_vo_inner_type(inner_type: &str) -> Result<(), String> {
    if !SHARED_VO_ALLOWED_INNER_TYPES.contains(&inner_type) {
        return Err(format!(
            "inner type '{inner_type}' is not allowed for a shared value object (allowed: {}; \
             'DateTime' is accepted as shorthand)",
            SHARED_VO_ALLOWED_INNER_TYPES.join(", ")
        ));
    }
    Ok(())
}

/// Full check for an entity name, in the PascalCase form the user typed.
pub fn validate_entity_name(pascal: &str) -> Result<(), String> {
    if pascal.is_empty() {
        return Err("entity name is empty".to_string());
    }
    if !is_pascal_case(pascal) {
        return Err(format!(
            "invalid entity name '{pascal}'. Must be PascalCase (an uppercase letter followed by \
             alphanumeric characters)"
        ));
    }
    if RESERVED_TYPE_NAMES.contains(&pascal) {
        return Err(format!(
            "entity name '{pascal}' collides with a type used in generated code. Pick another name"
        ));
    }
    let snake = crate::generators::naming::pascal_to_snake(pascal);
    if RUST_KEYWORDS.contains(&snake.as_str()) {
        return Err(format!(
            "entity name '{pascal}' becomes the module name '{snake}', which is a Rust keyword. \
             Pick another name"
        ));
    }
    if RESERVED_MODULE_NAMES.contains(&snake.as_str()) {
        return Err(format!(
            "entity name '{pascal}' becomes the module name '{snake}', which the generated project \
             already uses. Pick another name"
        ));
    }
    Ok(())
}

/// Full check for a value object name — local or shared.
pub fn validate_vo_name(vo_name: &str) -> Result<(), String> {
    if vo_name.is_empty() {
        return Err("value object name is empty".to_string());
    }
    if !is_pascal_case(vo_name) {
        return Err(format!(
            "invalid value object name '{vo_name}'. Must be PascalCase (an uppercase letter \
             followed by alphanumeric characters)"
        ));
    }
    if RESERVED_TYPE_NAMES.contains(&vo_name) {
        return Err(format!(
            "value object name '{vo_name}' collides with a type used in generated code. Pick \
             another name"
        ));
    }
    Ok(())
}

/// Enum VO variants: PascalCase and distinct. Two identical variants are `enum` arms defined
/// twice (E0428).
pub fn validate_enum_variants(variants: &[String]) -> Result<(), String> {
    if variants.is_empty() {
        return Err("enum value object has no variants".to_string());
    }
    let mut seen: Vec<&str> = vec![];
    for v in variants {
        if !is_pascal_case(v) {
            return Err(format!(
                "invalid enum variant '{v}'. Must be PascalCase (an uppercase letter followed by \
                 alphanumeric characters)"
            ));
        }
        if seen.contains(&v.as_str()) {
            return Err(format!(
                "duplicate enum variant '{v}' — every variant must be distinct"
            ));
        }
        seen.push(v);
    }
    Ok(())
}

/// Cross-field checks that need the whole list: one `value_objects.rs` is written per entity, so
/// two fields naming the same VO must agree on its inner type, and a local VO must not contradict
/// a shared one.
pub fn validate_vo_coherence(
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Vec<String> {
    let mut errors = vec![];
    let mut declared: Vec<(&str, &str)> = vec![];

    for field in fields {
        let Some(vo_name) = field.value_object.as_deref() else {
            continue;
        };
        // Enum VOs carry their shape in `enum_variants`, not in `field_type` (always "String").
        let inner = if field.value_object_kind.as_deref() == Some("enum") {
            "enum"
        } else {
            field.field_type.as_str()
        };

        if let Some((_, previous)) = declared.iter().find(|(n, _)| *n == vo_name) {
            if *previous != inner {
                errors.push(format!(
                    "value object '{vo_name}' is declared twice with different inner types \
                     ('{previous}' and '{inner}') — one value_objects.rs cannot define both"
                ));
            }
        } else {
            declared.push((vo_name, inner));
        }

        if let Some(shared) = shared_vos.iter().find(|v| v.name == vo_name) {
            if inner != "enum" && shared.inner_type != inner {
                errors.push(format!(
                    "value object '{vo_name}' is declared as [[value_object]] with type \
                     '{}' but field '{}' uses it as '{inner}'",
                    shared.inner_type, field.name
                ));
            }
        }
    }

    errors
}

/// Every per-field check that does not need the type registry, for one parsed field.
pub fn validate_field(field: &Field) -> Result<(), String> {
    validate_field_name(&field.name)?;
    if let Some(vo_name) = field.value_object.as_deref() {
        validate_vo_name(vo_name)?;
    }
    if let Some(variants) = &field.enum_variants {
        validate_enum_variants(variants)?;
    }
    if field.references.is_some() {
        validate_reference_shape(field)?;
    }
    Ok(())
}

/// A reference field on its own (ADR-0022): a PascalCase target, the id's type, not also a value
/// object, and `unique` (one-to-one) only when it is required.
fn validate_reference_shape(field: &Field) -> Result<(), String> {
    let name = &field.name;
    let target = field.references.as_deref().unwrap_or_default();
    if !is_pascal_case(target) {
        return Err(format!(
            "field '{name}' references '{target}': an entity name must be PascalCase"
        ));
    }
    if !matches!(field.field_type.as_str(), "Uuid" | "Option<Uuid>") {
        return Err(format!(
            "field '{name}' references {target} but its type is '{}': a reference holds an id, \
             Uuid or Option<Uuid>",
            field.field_type
        ));
    }
    if field.value_object.is_some() {
        return Err(format!(
            "field '{name}' references {target} and also names a value_object: a reference's \
             type is {target}'s id"
        ));
    }
    if field.unique && field.field_type.starts_with("Option<") {
        return Err(format!(
            "field '{name}' is an optional reference and cannot be marked unique"
        ));
    }
    Ok(())
}

/// The references of entity `pascal`, against the manifest (ADR-0022): each target is another
/// declared CRUD entity — not the entity itself (self references are a later spec), not the
/// auth projects' `User` (ownership is a later spec).
pub fn validate_references(
    pascal: &str,
    fields: &[Field],
    config: &crate::puerto_toml::PuertoConfig,
) -> Vec<String> {
    use crate::generators::naming::{is_crud_entity, pascal_to_snake};

    let mut errors = vec![];
    for field in fields {
        let Some(target) = field.references.as_deref() else {
            continue;
        };
        let name = &field.name;
        if target == pascal {
            errors.push(format!(
                "field '{name}' references {target} itself: references to the same entity are \
                 not generated yet"
            ));
            continue;
        }
        if config.project.has_auth() && validate_auth_entity_name(target).is_err() {
            errors.push(format!(
                "field '{name}' references {target}: references to the auth entities are not \
                 generated yet"
            ));
            continue;
        }
        match config.entity.iter().find(|e| e.name == target) {
            None => errors.push(format!(
                "field '{name}' references {target}, which is not declared in puerto.toml — \
                 `puerto generate scaffold {target}` first"
            )),
            Some(entity) if !is_crud_entity(&pascal_to_snake(target), &entity.use_cases) => errors
                .push(format!(
                    "field '{name}' references {target}, which is not a CRUD entity: only an \
                     entity with its five scaffolded use cases has an id type and a table"
                )),
            Some(_) => {}
        }
    }
    errors
}

/// [`validate_references`], then each target's code on disk: its model (the id type) and, in a
/// `--db` project, its `create_<snake>_table` migration — written earlier, so the foreign key
/// resolves when migrations run in order. Checked by the commands that write an entity.
pub fn validate_references_before_writing(
    pascal: &str,
    fields: &[Field],
    config: &crate::puerto_toml::PuertoConfig,
    base: &std::path::Path,
) -> Result<(), String> {
    use crate::generators::naming::pascal_to_snake;

    let mut errors = validate_references(pascal, fields, config);
    if errors.is_empty() {
        for target in fields.iter().filter_map(|f| f.references.as_deref()) {
            let snake = pascal_to_snake(target);
            let model = base.join(format!("business/src/domain/{snake}/model.rs"));
            let suffix = format!("_create_{snake}_table.sql");
            let migrated = !config.project.db
                || std::fs::read_dir(base.join("infrastructure/migrations"))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().ends_with(&suffix));
            if !model.exists() || !migrated {
                errors.push(format!(
                    "{pascal} references {target}, which has no code yet — run \
                     `puerto generate scaffold {target}` first"
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n  "))
    }
}

/// The transports `[project] events` can name (ADR-0018 § 4).
pub const EVENT_TRANSPORTS: &[&str] = &["db", "kafka"];

pub fn validate_events_transport(transport: &str) -> Result<(), String> {
    if EVENT_TRANSPORTS.contains(&transport) {
        return Ok(());
    }
    Err(format!(
        "unknown events transport '{transport}'. Use one of: {}",
        EVENT_TRANSPORTS.join(", ")
    ))
}

/// A custom event of entity `snake`, checked against the ones it already declares: snake_case,
/// none of the CRUD events, and a `{Entity}Event` variant of its own (`product_published` and
/// `published` would both be `Published`).
pub fn validate_event_name(snake: &str, event: &str, declared: &[String]) -> Result<(), String> {
    use crate::generators::naming::{crud_events, event_variant};

    if !is_valid_field_name(event) {
        return Err(format!(
            "invalid event name '{event}'. Must be snake_case, past tense, e.g. '{snake}_published'"
        ));
    }
    let variant = event_variant(snake, event);
    if variant == "Self" {
        return Err(format!("event name '{event}' would be the variant `Self`"));
    }
    if let Some((_, crud)) = crud_events(snake)
        .into_iter()
        .find(|(crud_variant, crud)| crud == event || *crud_variant == variant)
    {
        return Err(format!(
            "event '{event}' clashes with '{crud}', which every write of the entity already records"
        ));
    }
    if let Some(existing) = declared
        .iter()
        .find(|d| *d == event || event_variant(snake, d) == variant)
    {
        return Err(format!(
            "event '{event}' is already declared (as '{existing}')"
        ));
    }
    Ok(())
}

/// The modes `[project] auth` can name (ADR-0021 § 1).
pub const AUTH_MODES: &[&str] = &["oidc", "local"];

pub fn validate_auth_mode(mode: &str) -> Result<(), String> {
    if AUTH_MODES.contains(&mode) {
        return Ok(());
    }
    Err(format!(
        "unknown auth mode '{mode}'. Use one of: {}",
        AUTH_MODES.join(", ")
    ))
}

/// Module names an auth project already occupies: the `User` entity it is created with
/// (`domain/user`, `infrastructure/src/user`), `GET /me` (`api/me`) and `/auth/*` (`api/auth`).
const AUTH_RESERVED_MODULE_NAMES: &[&str] = &["user", "me", "auth"];

/// The entity names an auth project cannot scaffold, on top of [`validate_entity_name`].
pub fn validate_auth_entity_name(pascal: &str) -> Result<(), String> {
    let snake = crate::generators::naming::pascal_to_snake(pascal);
    if AUTH_RESERVED_MODULE_NAMES.contains(&snake.as_str()) {
        return Err(format!(
            "entity name '{pascal}' becomes the module name '{snake}', which an auth project \
             already uses (the User entity, GET /me, /auth). Pick another name"
        ));
    }
    Ok(())
}

/// `[a-z][a-z0-9_:-]*` — what `#[require_role]` accepts (`admin`, `billing:read`, `team-lead`).
pub fn validate_role_name(role: &str) -> Result<(), String> {
    let mut chars = role.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '-'));
    if !valid {
        return Err(format!(
            "invalid role '{role}'. Use lowercase letters, digits, '_', ':' and '-', starting \
             with a letter (e.g. 'admin', 'billing:read')"
        ));
    }
    Ok(())
}

/// An entity's `public` and `roles`: each names one of its use cases, a public route requires no
/// role, and every role is a role name.
pub fn validate_entity_access(entity: &crate::puerto_toml::Entity) -> Vec<String> {
    let mut errors = vec![];
    let names_use_case = |uc: &String| entity.use_cases.contains(uc);
    for (i, uc) in entity.public.iter().enumerate() {
        if !names_use_case(uc) {
            errors.push(format!(
                "public: '{uc}' is not one of the entity's use cases"
            ));
        }
        if entity.public[..i].contains(uc) {
            errors.push(format!("public: '{uc}' is listed twice"));
        }
    }
    for (uc, roles) in &entity.roles {
        if !names_use_case(uc) {
            errors.push(format!(
                "roles: '{uc}' is not one of the entity's use cases"
            ));
        }
        if entity.public.contains(uc) {
            errors.push(format!(
                "'{uc}' is both public and requires a role: a public route has no caller to check"
            ));
        }
        if roles.is_empty() {
            errors.push(format!(
                "roles: '{uc}' requires no role — remove it or name one"
            ));
        }
        for role in roles {
            if let Err(e) = validate_role_name(role) {
                errors.push(format!("roles: '{uc}': {e}"));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_system_field_names() {
        for name in SYSTEM_FIELDS {
            assert!(
                validate_field_name(name).is_err(),
                "'{name}' must be rejected"
            );
        }
    }

    #[test]
    fn rejects_rust_keyword_field_names() {
        assert!(validate_field_name("type").is_err());
        assert!(validate_field_name("match").is_err());
        assert!(validate_field_name("move").is_err());
    }

    #[test]
    fn rejects_leading_underscore_field_name() {
        assert!(validate_field_name("_hidden").is_err());
    }

    #[test]
    fn accepts_ordinary_field_names() {
        assert!(validate_field_name("title").is_ok());
        assert!(validate_field_name("unit_price_2").is_ok());
    }

    #[test]
    fn rejects_entity_names_that_collide_with_generated_code() {
        assert!(validate_entity_name("Error").is_err());
        assert!(validate_entity_name("String").is_err());
        assert!(validate_entity_name("Tracing").is_err()); // module would shadow `tracing`
        assert!(validate_entity_name("Db").is_err()); // infrastructure/src/db.rs
        assert!(validate_entity_name("Type").is_err()); // module would be `type`
        assert!(validate_entity_name("Product").is_ok());
    }

    #[test]
    fn rejects_duplicate_enum_variants() {
        let variants = vec!["Active".to_string(), "Active".to_string()];
        assert!(validate_enum_variants(&variants).is_err());
    }

    #[test]
    fn rejects_same_vo_name_with_two_inner_types() {
        let fields = vec![
            Field {
                name: "a".into(),
                field_type: "String".into(),
                value_object: Some("Code".into()),
                ..Default::default()
            },
            Field {
                name: "b".into(),
                field_type: "i64".into(),
                value_object: Some("Code".into()),
                ..Default::default()
            },
        ];
        assert_eq!(validate_vo_coherence(&fields, &[]).len(), 1);
    }

    #[test]
    fn accepts_same_vo_name_with_matching_inner_type() {
        let fields = vec![
            Field {
                name: "a".into(),
                field_type: "String".into(),
                value_object: Some("Code".into()),
                ..Default::default()
            },
            Field {
                name: "b".into(),
                field_type: "String".into(),
                value_object: Some("Code".into()),
                ..Default::default()
            },
        ];
        assert!(validate_vo_coherence(&fields, &[]).is_empty());
    }

    #[test]
    fn rejects_a_field_that_uses_a_shared_vo_with_another_inner_type() {
        let shared = vec![ValueObjectDefinition {
            name: "Email".into(),
            inner_type: "String".into(),
        }];
        let fields = vec![Field {
            name: "email".into(),
            field_type: "i64".into(),
            value_object: Some("Email".into()),
            ..Default::default()
        }];

        let errors = validate_vo_coherence(&fields, &shared);

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("Email"));
    }

    #[test]
    fn accepts_a_field_that_uses_a_shared_vo_with_its_inner_type() {
        let shared = vec![ValueObjectDefinition {
            name: "Email".into(),
            inner_type: "String".into(),
        }];
        let fields = vec![Field {
            name: "email".into(),
            field_type: "String".into(),
            value_object: Some("Email".into()),
            ..Default::default()
        }];

        assert!(validate_vo_coherence(&fields, &shared).is_empty());
    }

    #[test]
    fn accepts_an_enum_vo_named_like_a_shared_vo() {
        // Enum VOs are always `String`-typed in the manifest; their shape lives in the variants.
        let shared = vec![ValueObjectDefinition {
            name: "Status".into(),
            inner_type: "i64".into(),
        }];
        let fields = vec![Field {
            name: "status".into(),
            field_type: "String".into(),
            value_object: Some("Status".into()),
            value_object_kind: Some("enum".into()),
            enum_variants: Some(vec!["Active".into()]),
            ..Default::default()
        }];

        assert!(validate_vo_coherence(&fields, &shared).is_empty());
    }
    // ── Phase 16.4: auth ─────────────────────────────────────────────────────

    #[test]
    fn should_accept_the_auth_modes_and_reject_any_other() {
        assert!(validate_auth_mode("oidc").is_ok());
        assert!(validate_auth_mode("local").is_ok());
        for mode in ["", "OIDC", "auth0", "keycloak"] {
            assert!(validate_auth_mode(mode).is_err(), "{mode}");
        }
    }

    #[test]
    fn should_reserve_the_auth_modules_when_naming_an_entity_of_an_auth_project() {
        for name in ["User", "Me", "Auth"] {
            assert!(validate_auth_entity_name(name).is_err(), "{name}");
        }
        for name in ["Users", "UserProfile", "Author", "Product"] {
            assert!(validate_auth_entity_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn should_accept_role_names_the_macro_accepts() {
        for role in ["admin", "a", "billing:read", "team-lead", "level_2"] {
            assert!(validate_role_name(role).is_ok(), "{role}");
        }
        for role in ["", "Admin", "1admin", "ad min", "_admin", "admin!"] {
            assert!(validate_role_name(role).is_err(), "{role}");
        }
    }

    fn product(public: &[&str], roles: &[(&str, &[&str])]) -> crate::puerto_toml::Entity {
        crate::puerto_toml::Entity {
            name: "Product".into(),
            use_cases: vec!["create_product".into(), "list_products".into()],
            public: public.iter().map(|s| s.to_string()).collect(),
            roles: roles
                .iter()
                .map(|(uc, roles)| {
                    (
                        uc.to_string(),
                        roles.iter().map(|r| r.to_string()).collect(),
                    )
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn should_accept_public_and_roles_when_they_name_distinct_use_cases() {
        let entity = product(&["list_products"], &[("create_product", &["admin"])]);
        assert_eq!(validate_entity_access(&entity), Vec::<String>::new());
    }

    #[test]
    fn should_reject_public_and_roles_when_they_name_no_use_case_of_the_entity() {
        let entity = product(&["get_product"], &[("delete_product", &["admin"])]);
        let errors = validate_entity_access(&entity);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(
            errors[0].contains("'get_product' is not one of"),
            "{errors:?}"
        );
        assert!(
            errors[1].contains("'delete_product' is not one of"),
            "{errors:?}"
        );
    }

    #[test]
    fn should_reject_a_use_case_when_it_is_both_public_and_role_protected() {
        let entity = product(&["create_product"], &[("create_product", &["admin"])]);
        let errors = validate_entity_access(&entity);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("both public and requires a role"),
            "{errors:?}"
        );
    }

    #[test]
    fn should_reject_roles_when_they_are_empty_or_not_role_names() {
        let errors = validate_entity_access(&product(&[], &[("create_product", &[])]));
        assert!(errors[0].contains("requires no role"), "{errors:?}");
        let errors = validate_entity_access(&product(&[], &[("create_product", &["Admin"])]));
        assert!(errors[0].contains("invalid role 'Admin'"), "{errors:?}");
    }

    #[test]
    fn should_reject_a_public_use_case_when_it_is_listed_twice() {
        let errors = validate_entity_access(&product(&["list_products", "list_products"], &[]));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("listed twice"), "{errors:?}");
    }
}
