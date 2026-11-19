//! Name and shape validation shared by the CLI parser and `puerto validar`.
//!
//! These two used to diverge: `parse_field_arg` checked snake_case and nothing else, while
//! `run_validate` re-implemented its own predicates. Anything they both missed — a field called
//! `id`, a field called `type`, an entity called `Error` — reached the generators and produced
//! Rust that does not compile, with the error surfacing in the *generated* project rather than at
//! the command that caused it.
//!
//! Everything here is a pure name predicate. Type-registry checks live in `generators::types`.

use crate::generators::layout;
use crate::puerto_toml::{Field, ValueObjectDefinition};

/// Fields every generated entity already has. A `[[entidad.campos]]` entry with one of these names
/// emits the struct field twice (E0124).
pub const SYSTEM_FIELDS: &[&str] = &["id", "creado_en", "actualizado_en", "borrado", "borrado_en"];

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

/// Inner types a shared `[[objeto_valor]]` may wrap.
pub const SHARED_VO_ALLOWED_INNER_TYPES: &[&str] =
    &["String", "i64", "bool", "f64", "Uuid", "DateTime<Utc>"];

/// Type names in scope in generated code. An entity named after one of these produces a struct
/// that shadows it in the same file.
const RESERVED_TYPE_NAMES: &[&str] = &[
    "Self", "String", "Option", "Vec", "Box", "Result", "HashMap", "Uuid", "DateTime", "Utc",
    "Arc", "Error",
];

/// Context names the generated project already occupies: an entity's snake case name is its
/// context, `contextos/{snake}/` (ADR-0023).
const RESERVED_MODULE_NAMES: &[&str] = &[
    "correo", // contextos/correo — the project's mails
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

/// Full check for a `[[entidad.campos]]` name.
pub fn validate_field_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("el nombre del campo está vacío".to_string());
    }
    if !is_valid_field_name(name) {
        return Err(format!(
            "nombre de campo inválido '{name}'. Va en snake_case (minúsculas, dígitos y guiones \
             bajos, y empieza por una minúscula)"
        ));
    }
    if SYSTEM_FIELDS.contains(&name) {
        return Err(format!(
            "el nombre de campo '{name}' está reservado: toda entidad ya tiene {}. Quítalo de la \
             lista de campos",
            SYSTEM_FIELDS.join(", ")
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "el nombre de campo '{name}' es una palabra clave de Rust y no puede ser un campo de un \
             struct. Elige otro nombre"
        ));
    }
    Ok(())
}

/// A field name that is also a PostgreSQL reserved word — only an error in a `--db` project.
pub fn validate_sql_field_name(name: &str) -> Result<(), String> {
    if SQL_RESERVED_WORDS.contains(&name) {
        return Err(format!(
            "el nombre de campo '{name}' es una palabra reservada de SQL y no puede ser el nombre \
             de una columna. Elige otro nombre (p. ej. '{name}_valor')"
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
                "el campo '{}' está repetido: cada campo tiene su propio nombre",
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
            "nombre de caso de uso inválido '{action}'. Va en snake_case (minúsculas, dígitos y \
             guiones bajos, y empieza por una minúscula), p. ej. 'archivar_producto'"
        ));
    }
    if RUST_KEYWORDS.contains(&action) {
        return Err(format!(
            "el caso de uso '{action}' es una palabra clave de Rust. Elige otro nombre"
        ));
    }
    Ok(())
}

/// The modules of a domain crate and the methods of a CRUD entity: a transition's verb is both a
/// module (`dominio/src/<verb>.rs`) and a method (`impl <Entity> { fn <verb> }`).
const TRANSITION_TAKEN: &[&str] = &[
    "crear",
    "obtener",
    "listar",
    "actualizar",
    "borrar",
    "validar",
    "new",
    "from_repositorio",
    "modelo",
    "errores",
    "repositorio",
    "valores",
    "eventos",
    "unit_of_work",
    "pruebas",
];

/// A transition's verb (27.12): snake_case, and none of the names its entity already uses.
pub fn validate_transition_verb(verb: &str) -> Result<(), String> {
    if !is_valid_field_name(verb) {
        return Err(format!(
            "verbo inválido '{verb}'. Va en snake_case e infinitivo, p. ej. 'cancelar' o \
             'poner_a_la_venta'"
        ));
    }
    if RUST_KEYWORDS.contains(&verb) {
        return Err(format!(
            "el verbo '{verb}' es una palabra clave de Rust. Elige otro"
        ));
    }
    if TRANSITION_TAKEN.contains(&verb) {
        return Err(format!(
            "el verbo '{verb}' ya es un caso de uso CRUD, un método o un módulo del dominio de \
             la entidad. Elige otro"
        ));
    }
    Ok(())
}

/// A `[[consumidor]]` name becomes a module (`presentacion/worker/src/consumidores/<name>.rs`), a type
/// (`NotifyNewProduct`) and the `db` transport's consumer column.
pub fn validate_consumer_name(name: &str) -> Result<(), String> {
    if !is_valid_field_name(name) {
        return Err(format!(
            "nombre de consumidor inválido '{name}'. Va en snake_case (minúsculas, dígitos y \
             guiones bajos, y empieza por una minúscula), p. ej. 'avisar_producto_nuevo'"
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "el consumidor '{name}' es una palabra clave de Rust. Elige otro nombre"
        ));
    }
    Ok(())
}

/// A `[[correo]]` name becomes two modules (`domain::mail::<name>`, `mail::<name>`), a template
/// folder and the types `{Name}Mail` / `{Name}Mailer`.
pub fn validate_mail_name(name: &str) -> Result<(), String> {
    if !is_valid_field_name(name) {
        return Err(format!(
            "nombre de correo inválido '{name}'. Va en snake_case (minúsculas, dígitos y guiones \
             bajos, y empieza por una minúscula), p. ej. 'bienvenida'"
        ));
    }
    if RUST_KEYWORDS.contains(&name) {
        return Err(format!(
            "el correo '{name}' es una palabra clave de Rust. Elige otro nombre"
        ));
    }
    Ok(())
}

/// The inner type of a shared `[[objeto_valor]]`.
pub fn validate_shared_vo_inner_type(inner_type: &str) -> Result<(), String> {
    if !SHARED_VO_ALLOWED_INNER_TYPES.contains(&inner_type) {
        return Err(format!(
            "el tipo interno '{inner_type}' no vale para un objeto valor compartido (valen: {}; \
             'DateTime' sirve de atajo)",
            SHARED_VO_ALLOWED_INNER_TYPES.join(", ")
        ));
    }
    Ok(())
}

/// A value object's class (27.16): `enum`, or one with a rule, on the primitive that rule reads.
pub fn validate_vo_class(class: &str, inner_type: &str) -> Result<(), String> {
    if class == "enum" {
        return Ok(());
    }
    match crate::puerto_toml::VO_CLASSES
        .iter()
        .find(|(c, _)| *c == class)
    {
        None => Err(format!(
            "la clase '{class}' no existe (valen: enum, {})",
            crate::puerto_toml::VO_CLASSES
                .iter()
                .map(|(c, _)| *c)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Some((_, ty)) if *ty != inner_type => Err(format!(
            "la clase '{class}' va sobre un {ty}, no sobre '{inner_type}'"
        )),
        Some(_) => Ok(()),
    }
}

/// A shared value object's `ejemplo` (27.15): only for `String` (not blank), `i64`, `f64`
/// (finite) and `bool`, and read as that type — it becomes a literal in the generated tests.
pub fn validate_vo_example(inner_type: &str, example: &str) -> Result<(), String> {
    let valid = match inner_type {
        "String" => !example.trim().is_empty(),
        "i64" => example.parse::<i64>().is_ok(),
        "f64" => example.parse::<f64>().is_ok_and(f64::is_finite),
        "bool" => matches!(example, "true" | "false"),
        _ => {
            return Err(format!(
                "un ejemplo solo vale en un objeto valor String, i64, f64 o bool, no en \
                 '{inner_type}'"
            ));
        }
    };
    if !valid {
        return Err(format!(
            "el ejemplo '{example}' no es un valor de tipo '{inner_type}'"
        ));
    }
    Ok(())
}

/// Full check for an entity name, in the PascalCase form the user typed.
pub fn validate_entity_name(pascal: &str) -> Result<(), String> {
    if pascal.is_empty() {
        return Err("el nombre de la entidad está vacío".to_string());
    }
    if !is_pascal_case(pascal) {
        return Err(format!(
            "nombre de entidad inválido '{pascal}'. Va en PascalCase (una mayúscula seguida de \
             letras y dígitos)"
        ));
    }
    if RESERVED_TYPE_NAMES.contains(&pascal) {
        return Err(format!(
            "la entidad '{pascal}' choca con un tipo que usa el código generado. Elige otro nombre"
        ));
    }
    let snake = crate::generators::naming::pascal_to_snake(pascal);
    if RUST_KEYWORDS.contains(&snake.as_str()) {
        return Err(format!(
            "la entidad '{pascal}' da el módulo '{snake}', que es una palabra clave de Rust. Elige \
             otro nombre"
        ));
    }
    if RESERVED_MODULE_NAMES.contains(&snake.as_str()) {
        return Err(format!(
            "la entidad '{pascal}' da el contexto '{snake}', que el proyecto generado ya usa. \
             Elige otro nombre"
        ));
    }
    Ok(())
}

/// Full check for a value object name — local or shared.
pub fn validate_vo_name(vo_name: &str) -> Result<(), String> {
    if vo_name.is_empty() {
        return Err("el nombre del objeto valor está vacío".to_string());
    }
    if !is_pascal_case(vo_name) {
        return Err(format!(
            "nombre de objeto valor inválido '{vo_name}'. Va en PascalCase (una mayúscula seguida \
             de letras y dígitos)"
        ));
    }
    if RESERVED_TYPE_NAMES.contains(&vo_name) {
        return Err(format!(
            "el objeto valor '{vo_name}' choca con un tipo que usa el código generado. Elige otro \
             nombre"
        ));
    }
    Ok(())
}

/// Enum VO variants: PascalCase and distinct. Two identical variants are `enum` arms defined
/// twice (E0428).
pub fn validate_enum_variants(variants: &[String]) -> Result<(), String> {
    if variants.is_empty() {
        return Err("el objeto valor enum no tiene variantes".to_string());
    }
    let mut seen: Vec<&str> = vec![];
    for v in variants {
        if !is_pascal_case(v) {
            return Err(format!(
                "variante de enum inválida '{v}'. Va en PascalCase (una mayúscula seguida de letras y \
                 dígitos)"
            ));
        }
        if seen.contains(&v.as_str()) {
            return Err(format!(
                "la variante '{v}' está repetida: cada variante tiene su propio nombre"
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
                    "el objeto valor '{vo_name}' se declara dos veces con tipos internos distintos \
                     ('{previous}' y '{inner}'): un mismo valores.rs no puede definir los dos"
                ));
            }
        } else {
            declared.push((vo_name, inner));
        }

        if let Some(shared) = shared_vos.iter().find(|v| v.name == vo_name) {
            if inner != "enum" && shared.inner_type != inner {
                errors.push(format!(
                    "el objeto valor '{vo_name}' está declarado en [[objeto_valor]] con el tipo \
                     '{}', pero el campo '{}' lo usa como '{inner}'",
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
    if let Some(label) = &field.label {
        if !is_screen_text(label) {
            return Err(format!(
                "la etiqueta del campo '{}' no puede estar vacía ni llevar comillas, `\\` o saltos \
                 de línea: {label:?}",
                field.name
            ));
        }
    }
    if let Some(texts) = &field.texts {
        let variants = field.enum_variants.as_deref().unwrap_or_default();
        for (variant, text) in texts {
            if !variants.contains(variant) {
                return Err(format!(
                    "los textos del campo '{}' nombran '{variant}', que no es una de sus variantes \
                     ({})",
                    field.name,
                    variants.join(", ")
                ));
            }
            if !is_screen_text(text) {
                return Err(format!(
                    "los textos del campo '{}': el de '{variant}' no puede estar vacío ni llevar \
                     comillas, `\\` o saltos de línea: {text:?}",
                    field.name
                ));
            }
        }
    }
    Ok(())
}

/// A text the screens write into a string literal: not blank, nothing that would end it.
fn is_screen_text(text: &str) -> bool {
    !text.trim().is_empty() && !text.contains(['"', '\\', '\n', '\r'])
}

/// A reference field on its own (ADR-0022): a PascalCase target, the id's type, not also a value
/// object, and `unique` (one-to-one) only when it is required.
fn validate_reference_shape(field: &Field) -> Result<(), String> {
    let name = &field.name;
    let target = field.references.as_deref().unwrap_or_default();
    if !is_pascal_case(target) {
        return Err(format!(
            "el campo '{name}' referencia '{target}': un nombre de entidad va en PascalCase"
        ));
    }
    if !matches!(field.field_type.as_str(), "Uuid" | "Option<Uuid>") {
        return Err(format!(
            "el campo '{name}' referencia {target}, pero su tipo es '{}': una referencia guarda un \
             id, Uuid u Option<Uuid>",
            field.field_type
        ));
    }
    if field.value_object.is_some() {
        return Err(format!(
            "el campo '{name}' referencia {target} y además nombra un objeto_valor: el tipo de una \
             referencia es el id de {target}"
        ));
    }
    if field.unique && field.field_type.starts_with("Option<") {
        return Err(format!(
            "el campo '{name}' es una referencia opcional y no puede ser único"
        ));
    }
    Ok(())
}

/// The references of entity `pascal`, against the manifest (ADR-0022): each target is another
/// declared CRUD entity — not the entity itself (self references are a later spec), not the
/// auth projects' `Usuario` (ownership is a later spec).
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
                "el campo '{name}' referencia a su propia entidad, {target}: todavía no se generan \
                 autorreferencias"
            ));
            continue;
        }
        if config.project.has_auth() && validate_auth_entity_name(target).is_err() {
            errors.push(format!(
                "el campo '{name}' referencia {target}: todavía no se generan referencias a las \
                 entidades de auth"
            ));
            continue;
        }
        match config.entity.iter().find(|e| e.name == target) {
            None => errors.push(format!(
                "el campo '{name}' referencia {target}, que no está declarada en puerto.toml: antes, \
                 `puerto generar scaffold {target}`"
            )),
            Some(entity) if !is_crud_entity(&pascal_to_snake(target), &entity.use_cases) => errors
                .push(format!(
                    "el campo '{name}' referencia {target}, que no es una entidad CRUD: solo una \
                     entidad con sus cinco casos de uso del scaffold tiene un tipo de id y una tabla"
                )),
            Some(_) => {}
        }
    }
    if errors.is_empty()
        && let Some(cycle) = reference_cycle(pascal, fields, config)
    {
        errors.push(format!(
            "{}: esta referencia cierra un ciclo. Cada contexto tiene sus propios crates, y Cargo \
             no compila crates que dependen unos de otros en círculo (ADR-0023 § 3): quita una de \
             las referencias",
            cycle.join(" → ")
        ));
    }
    errors
}

/// The references, followed from `pascal` (with `fields`, its fields to be) through the other
/// entities' declared ones, back to `pascal`: `["Order", "Customer", "Order"]`, or none.
fn reference_cycle(
    pascal: &str,
    fields: &[Field],
    config: &crate::puerto_toml::PuertoConfig,
) -> Option<Vec<String>> {
    let targets = |entity: &str| -> Vec<String> {
        let fields = if entity == pascal {
            fields
        } else {
            match config.entity.iter().find(|e| e.name == entity) {
                Some(e) => e.fields.as_slice(),
                None => &[],
            }
        };
        fields.iter().filter_map(|f| f.references.clone()).collect()
    };
    // Depth first, one path at a time; `seen` stops at an entity already explored.
    let mut seen: Vec<String> = vec![];
    let mut stack: Vec<Vec<String>> = vec![vec![pascal.to_string()]];
    while let Some(path) = stack.pop() {
        let last = path.last().cloned().unwrap_or_default();
        for target in targets(&last) {
            if target == pascal {
                let mut cycle = path.clone();
                cycle.push(target);
                return Some(cycle);
            }
            if !seen.contains(&target) {
                seen.push(target.clone());
                let mut next = path.clone();
                next.push(target);
                stack.push(next);
            }
        }
    }
    None
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
            let model = base.join(layout::domain_file(&snake, "modelo"));
            let suffix = format!("_crear_tabla_{snake}.sql");
            let migrated = !config.project.db
                || std::fs::read_dir(base.join(layout::MIGRATIONS_DIR))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().ends_with(&suffix));
            if !model.exists() || !migrated {
                errors.push(format!(
                    "{pascal} referencia {target}, que aún no tiene código: ejecuta antes \
                     `puerto generar scaffold {target}`"
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

/// The transports `[proyecto] eventos` can name (ADR-0018 § 4).
pub const EVENT_TRANSPORTS: &[&str] = &["db", "kafka"];

/// The route of the app under a portada: `/` and lowercase segments (`/panel`, `/app/v2`), never
/// `/` alone: that is the portada's.
pub fn validate_app_path(path: &str) -> Result<(), String> {
    let segments: Vec<&str> = match path.strip_prefix('/') {
        Some(rest) => rest.split('/').collect(),
        None => vec![],
    };
    let valid = !segments.is_empty()
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        });
    if valid {
        return Ok(());
    }
    Err(format!(
        "la ruta de la app '{path}' no vale: empieza por `/` y sigue con minúsculas, dígitos y `-` \
         (p. ej. /panel); `/` a secas es de la portada"
    ))
}

pub fn validate_events_transport(transport: &str) -> Result<(), String> {
    if EVENT_TRANSPORTS.contains(&transport) {
        return Ok(());
    }
    Err(format!(
        "transporte de eventos desconocido '{transport}'. Usa uno de estos: {}",
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
            "nombre de evento inválido '{event}'. Va en snake_case y en participio, p. ej. \
             '{snake}_publicado'"
        ));
    }
    let variant = event_variant(snake, event);
    if variant == "Self" {
        return Err(format!("el evento '{event}' daría la variante `Self`"));
    }
    if let Some((_, crud)) = crud_events(snake)
        .into_iter()
        .find(|(crud_variant, crud)| crud == event || *crud_variant == variant)
    {
        return Err(format!(
            "el evento '{event}' choca con '{crud}', que ya registra cada escritura de la entidad"
        ));
    }
    if let Some(existing) = declared
        .iter()
        .find(|d| *d == event || event_variant(snake, d) == variant)
    {
        return Err(format!(
            "el evento '{event}' ya está declarado (como '{existing}')"
        ));
    }
    Ok(())
}

/// The modes `[proyecto] auth` can name (ADR-0021 § 1).
pub const AUTH_MODES: &[&str] = &["oidc", "local"];

pub fn validate_auth_mode(mode: &str) -> Result<(), String> {
    if AUTH_MODES.contains(&mode) {
        return Ok(());
    }
    Err(format!(
        "modo de auth desconocido '{mode}'. Usa uno de estos: {}",
        AUTH_MODES.join(", ")
    ))
}

/// Names an auth project already occupies: its `usuario` context with the `Usuario` entity it is
/// created with, and the APIs of `GET /me` (`ApiMe`) and `/auth/*` (`ApiAuth`).
const AUTH_RESERVED_MODULE_NAMES: &[&str] = &["usuario", "me", "auth"];

/// The entity names an auth project cannot scaffold, on top of [`validate_entity_name`].
pub fn validate_auth_entity_name(pascal: &str) -> Result<(), String> {
    let snake = crate::generators::naming::pascal_to_snake(pascal);
    if AUTH_RESERVED_MODULE_NAMES.contains(&snake.as_str()) {
        return Err(format!(
            "la entidad '{pascal}' da el contexto '{snake}', que un proyecto con auth ya usa (el \
             contexto usuario, la entidad Usuario, GET /me, /auth). Elige otro nombre"
        ));
    }
    Ok(())
}

/// `[a-z][a-z0-9_:-]*` — what `#[requiere_rol]` accepts (`admin`, `billing:read`, `team-lead`).
pub fn validate_role_name(role: &str) -> Result<(), String> {
    let mut chars = role.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | ':' | '-'));
    if !valid {
        return Err(format!(
            "rol inválido '{role}'. Usa minúsculas, dígitos, '_', ':' y '-', y empieza por una \
             letra (p. ej. 'admin', 'facturas:leer')"
        ));
    }
    Ok(())
}

/// An entity's `unicos`: each group names two or more of its own fields, once each, none of them
/// an `Option`, a `Vec` or a map (a null never collides in a unique index; a list or a map is not
/// what a row is told apart by), and no group is listed twice. Its `orden`: each key (`-` for
/// descending) names one of its own plain, required fields, once.
pub fn validate_entity_shape(entity: &crate::puerto_toml::Entity) -> Vec<String> {
    let fields = crate::generators::effective_fields(&entity.fields);
    let mut errors = vec![];
    let mut seen: Vec<Vec<&String>> = vec![];
    for group in &entity.uniques {
        let shown = group.join(", ");
        if group.len() < 2 {
            errors.push(format!(
                "unicos: [{shown}] necesita al menos dos campos; para uno solo, `unico = true` en el \
                 campo"
            ));
        }
        for (i, name) in group.iter().enumerate() {
            match fields.iter().find(|f| &f.name == name) {
                None => errors.push(format!(
                    "unicos: '{name}' no es un campo de {}",
                    entity.name
                )),
                Some(f)
                    if f.field_type.starts_with("Option<")
                        || f.field_type.starts_with("Vec<")
                        || f.field_type.starts_with("HashMap<") =>
                {
                    errors.push(format!(
                        "unicos: '{name}' es un {}: un único lleva campos simples y obligatorios",
                        f.field_type
                    ));
                }
                Some(_) => {}
            }
            if group[..i].contains(name) {
                errors.push(format!("unicos: [{shown}] nombra '{name}' dos veces"));
            }
        }
        let mut sorted: Vec<&String> = group.iter().collect();
        sorted.sort();
        if seen.contains(&sorted) {
            errors.push(format!("unicos: [{shown}] está dos veces"));
        }
        seen.push(sorted);
    }
    for f in fields.iter().filter(|f| f.private) {
        let text = matches!(f.field_type.as_str(), "String" | "Option<String>");
        // A reference is a `Uuid`: never text.
        if !text || crate::generators::types::is_enum_vo(f) {
            errors.push(format!(
                "privado: '{}' es un {}: un campo privado es texto (String, Option<String> o un \
                 objeto valor sobre uno de ellos, que no sea enum)",
                f.name, f.field_type
            ));
        }
    }
    for (i, key) in entity.order.iter().enumerate() {
        let name = key.strip_prefix('-').unwrap_or(key);
        match fields.iter().find(|f| f.name == name) {
            None if name.is_empty() => errors.push(format!(
                "orden: '{key}' no nombra ningún campo: un campo, o `-` y un campo para el orden \
                 descendente"
            )),
            None => errors.push(format!("orden: '{name}' no es un campo de {}", entity.name)),
            Some(f)
                if f.field_type.starts_with("Option<")
                    || f.field_type.starts_with("Vec<")
                    || f.field_type.starts_with("HashMap<") =>
            {
                errors.push(format!(
                    "orden: '{name}' es un {}: una lista se ordena por campos simples y obligatorios",
                    f.field_type
                ));
            }
            Some(_) => {}
        }
        let earlier = entity.order[..i]
            .iter()
            .any(|k| k.strip_prefix('-').unwrap_or(k) == name);
        if earlier {
            errors.push(format!("orden: nombra '{name}' dos veces"));
        }
    }
    errors
}

/// An entity's `public` and `roles`: each names one of its use cases, a public route requires no
/// role, and every role is a role name.
pub fn validate_entity_access(entity: &crate::puerto_toml::Entity) -> Vec<String> {
    let mut errors = vec![];
    let names_use_case = |uc: &String| entity.use_cases.contains(uc);
    for (i, uc) in entity.public.iter().enumerate() {
        if !names_use_case(uc) {
            errors.push(format!(
                "publicos: '{uc}' no es uno de los casos de uso de la entidad"
            ));
        }
        if entity.public[..i].contains(uc) {
            errors.push(format!("publicos: '{uc}' está dos veces"));
        }
    }
    for (uc, roles) in &entity.roles {
        if !names_use_case(uc) {
            errors.push(format!(
                "roles: '{uc}' no es uno de los casos de uso de la entidad"
            ));
        }
        if entity.public.contains(uc) {
            errors.push(format!(
                "'{uc}' es público y a la vez exige un rol: una ruta pública no tiene llamante que \
                 comprobar"
            ));
        }
        if roles.is_empty() {
            errors.push(format!(
                "roles: '{uc}' no exige ningún rol: quítalo o nombra uno"
            ));
        }
        for role in roles {
            if let Err(e) = validate_role_name(role) {
                errors.push(format!("roles: '{uc}': {e}"));
            }
        }
    }
    // 27.13: an internal use case has no route — nothing to open, nothing to guard.
    let crud = crate::generators::naming::crud_use_cases(
        &crate::generators::naming::pascal_to_snake(&entity.name),
    );
    for uc in &entity.internal {
        if !names_use_case(uc) {
            errors.push(format!(
                "internos: '{uc}' no es uno de los casos de uso de la entidad"
            ));
        } else if crud.contains(uc) {
            errors.push(format!(
                "internos: '{uc}' es un caso de uso CRUD: su ruta está en rutas.rs"
            ));
        }
        if entity.public.contains(uc) || entity.roles.contains_key(uc) {
            errors.push(format!(
                "'{uc}' es interno: no tiene ruta, así que no va en publicos ni en roles"
            ));
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
        assert!(validate_entity_name("Correo").is_err()); // contextos/correo holds the mails
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
        for name in ["Usuario", "Me", "Auth"] {
            assert!(validate_auth_entity_name(name).is_err(), "{name}");
        }
        for name in ["Usuarios", "UsuarioPerfil", "User", "Author", "Producto"] {
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
            use_cases: vec!["crear_product".into(), "listar_products".into()],
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
        let entity = product(&["listar_products"], &[("crear_product", &["admin"])]);
        assert_eq!(validate_entity_access(&entity), Vec::<String>::new());
    }

    #[test]
    fn should_reject_public_and_roles_when_they_name_no_use_case_of_the_entity() {
        let entity = product(&["obtener_product"], &[("borrar_product", &["admin"])]);
        let errors = validate_entity_access(&entity);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(
            errors[0].contains("'obtener_product' no es uno de"),
            "{errors:?}"
        );
        assert!(
            errors[1].contains("'borrar_product' no es uno de"),
            "{errors:?}"
        );
    }

    #[test]
    fn should_reject_a_use_case_when_it_is_both_public_and_role_protected() {
        let entity = product(&["crear_product"], &[("crear_product", &["admin"])]);
        let errors = validate_entity_access(&entity);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("es público y a la vez exige un rol"),
            "{errors:?}"
        );
    }

    #[test]
    fn should_reject_roles_when_they_are_empty_or_not_role_names() {
        let errors = validate_entity_access(&product(&[], &[("crear_product", &[])]));
        assert!(errors[0].contains("no exige ningún rol"), "{errors:?}");
        let errors = validate_entity_access(&product(&[], &[("crear_product", &["Admin"])]));
        assert!(errors[0].contains("rol inválido 'Admin'"), "{errors:?}");
    }

    #[test]
    fn should_reject_a_public_use_case_when_it_is_listed_twice() {
        let errors = validate_entity_access(&product(&["listar_products", "listar_products"], &[]));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("está dos veces"), "{errors:?}");
    }

    #[test]
    fn should_name_the_cycle_a_reference_would_close() {
        // Arrange: Customer → Region already; Region gains a reference back to Customer.
        let reference = |name: &str, target: &str| Field {
            name: name.to_string(),
            field_type: "Uuid".to_string(),
            references: Some(target.to_string()),
            ..Field::default()
        };
        let crud = |name: &str, snake: &str, _plural: &str, fields: Vec<Field>| {
            crate::puerto_toml::Entity {
                name: name.to_string(),
                use_cases: crate::generators::naming::crud_use_cases(snake),
                fields,
                ..Default::default()
            }
        };
        let config = crate::puerto_toml::PuertoConfig {
            entity: vec![
                crud(
                    "Customer",
                    "customer",
                    "customers",
                    vec![reference("region_id", "Region")],
                ),
                crud("Region", "region", "regions", vec![]),
            ],
            ..Default::default()
        };

        // Act
        let errors = validate_references("Region", &[reference("owner_id", "Customer")], &config);

        // Assert
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("Region → Customer → Region"),
            "{}",
            errors[0]
        );
    }
}
