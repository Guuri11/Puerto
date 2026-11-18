use std::{fs, path::Path};

/// Normalize any casing to PascalCase: `order_item` → `OrderItem`, `product` → `Product`.
pub fn to_pascal_case(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

/// Convert PascalCase to snake_case: `OrderItem` → `order_item`.
pub fn pascal_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// snake_case to kebab-case, the frontend's file names: `order_item` → `order-item`.
pub fn snake_to_kebab(snake: &str) -> String {
    snake.replace('_', "-")
}

/// snake_case to camelCase, as poem-openapi renames DTO fields on the wire and the frontend names
/// its variables: `in_stock` → `inStock`, `order_item` → `orderItem`.
pub fn snake_to_camel(snake: &str) -> String {
    let pascal = to_pascal_case(snake);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().to_string() + chars.as_str(),
    }
}

/// snake_case to the words a screen shows: `in_stock` → `in stock`, `order_items` → `order items`.
pub fn snake_to_words(snake: &str) -> String {
    snake
        .split('_')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first letter in upper case: a label (`in stock` → `In stock`).
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Words whose plural is the word itself: they end in an unstressed `-is` / `-es` (or `-x`), which
/// no suffix rule can tell from a stressed one (`pais` → `paises`).
const INVARIABLE: &[&str] = &[
    "analisis",
    "crisis",
    "tesis",
    "sintesis",
    "hipotesis",
    "dosis",
    "lunes",
    "martes",
    "miercoles",
    "jueves",
    "viernes",
    "virus",
    "torax",
    "fenix",
];

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

/// Spanish plural of a snake_case name (docs/glosario.md), inflecting its first word — the head of
/// a Spanish name: `linea_pedido` → `lineas_pedido`. A vowel takes `s` (`productos`), `z` becomes
/// `ces` (`lapices`), any other consonant, `y` included, takes `es` (`ciudades`, `leyes`); a few
/// words are invariable (`analisis`).
pub fn pluralize(snake: &str) -> String {
    let (word, tail) = match snake.find('_') {
        Some(i) => snake.split_at(i),
        None => (snake, ""),
    };
    let plural = if INVARIABLE.contains(&word) {
        word.to_string()
    } else if word.ends_with(VOWELS) {
        format!("{word}s")
    } else if let Some(stem) = word.strip_suffix('z') {
        format!("{stem}ces")
    } else {
        format!("{word}es")
    };
    format!("{plural}{tail}")
}

/// The names every entity template starts from: `pascal` (`LineaPedido`), `snake` (`linea_pedido`),
/// `plural` (`lineas_pedido` — tables and REST paths) and `plural_pascal` (`LineasPedido`).
pub fn entity_ctx(pascal: &str, snake: &str) -> tera::Context {
    let plural = pluralize(snake);
    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("plural_pascal", &to_pascal_case(&plural));
    ctx.insert("plural", &plural);
    ctx
}

/// The verb a CRUD action is named with in the generated code (docs/glosario.md).
pub fn crud_verb(action: &str) -> &'static str {
    match action {
        "create" => "crear",
        "get" => "obtener",
        "list" => "listar",
        "update" => "actualizar",
        "delete" => "borrar",
        _ => unreachable!("unknown CRUD action {action}"),
    }
}

/// The module name of one scaffolded use case: `list` is named after the plural
/// (`listar_productos`), the others after the entity (`crear_producto`). `action` is Puerto's own
/// key (`create`, `get`, `list`, `update`, `delete`).
pub fn crud_use_case(action: &str, snake: &str) -> String {
    if action == "list" {
        format!("{}_{}", crud_verb(action), pluralize(snake))
    } else {
        format!("{}_{snake}", crud_verb(action))
    }
}

/// The five use cases `scaffold` and `generate domain` create for an entity.
pub fn crud_use_cases(snake: &str) -> Vec<String> {
    ["create", "get", "list", "update", "delete"]
        .iter()
        .map(|action| crud_use_case(action, snake))
        .collect()
}

/// Whether an entity has the five scaffolded use cases — the ones that get a unit of work, an
/// `events.rs` and a frontend slice. A simple entity (`obtener_saludo`) has none of them.
pub fn is_crud_entity(snake: &str, use_cases: &[String]) -> bool {
    crud_use_cases(snake)
        .iter()
        .all(|uc| use_cases.contains(uc))
}

/// The events every CRUD write records (ADR-0018 § 3): `(variant, event_type)`, past tense.
pub fn crud_events(snake: &str) -> Vec<(String, String)> {
    ["creado", "actualizado", "borrado"]
        .iter()
        .map(|verb| (to_pascal_case(verb), format!("{snake}_{verb}")))
        .collect()
}

/// The `{Entity}Event` variant of a custom event: the event type without the entity prefix, in
/// PascalCase — `product_published` → `Published`, `price_changed` → `PriceChanged`.
pub fn event_variant(snake: &str, event_type: &str) -> String {
    let unprefixed = event_type
        .strip_prefix(snake)
        .and_then(|rest| rest.strip_prefix('_'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(event_type);
    to_pascal_case(unprefixed)
}

/// Writes `content` to `path`, creating parent directories. The error names the path — a bare
/// `Permission denied (os error 13)` does not tell the user which file.
pub fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let with_path = |e: std::io::Error| {
        std::io::Error::new(
            e.kind(),
            format!("no se puede escribir {}: {e}", path.display()),
        )
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(with_path)?;
    }
    fs::write(path, content).map_err(with_path)
}
