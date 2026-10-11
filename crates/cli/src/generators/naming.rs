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

/// Words whose plural is the word itself.
const UNCOUNTABLE: &[&str] = &[
    "data",
    "metadata",
    "information",
    "equipment",
    "news",
    "series",
    "species",
];

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

/// Plurals no suffix rule produces.
const IRREGULAR: &[(&str, &str)] = &[
    ("person", "people"),
    ("child", "children"),
    ("man", "men"),
    ("woman", "women"),
    ("mouse", "mice"),
    ("goose", "geese"),
    ("foot", "feet"),
    ("tooth", "teeth"),
];

/// English plural of a snake_case name, inflecting only its last word: `category` → `categories`,
/// `order_item` → `order_items`, `person` → `people`, `box` → `boxes`.
///
/// Deliberately small — table names and REST paths, not prose. Words ending in `f`/`fe`/`o`
/// take the regular `s` (`roofs`, `photos`): their irregular forms are not predictable.
pub fn pluralize(snake: &str) -> String {
    let (head, word) = match snake.rfind('_') {
        Some(i) => snake.split_at(i + 1),
        None => ("", snake),
    };
    let plural = if UNCOUNTABLE.contains(&word) {
        word.to_string()
    } else if let Some((_, p)) = IRREGULAR.iter().find(|(s, _)| *s == word) {
        p.to_string()
    } else if word.len() > 1 && word.ends_with('y') && !word[..word.len() - 1].ends_with(VOWELS) {
        format!("{}ies", &word[..word.len() - 1])
    } else if word.ends_with(['s', 'x', 'z']) || word.ends_with("ch") || word.ends_with("sh") {
        format!("{word}es")
    } else {
        format!("{word}s")
    };
    format!("{head}{plural}")
}

/// The names every entity template starts from: `pascal` (`OrderItem`), `snake` (`order_item`),
/// `plural` (`order_items` — tables and REST paths) and `plural_pascal` (`OrderItems`).
pub fn entity_ctx(pascal: &str, snake: &str) -> tera::Context {
    let plural = pluralize(snake);
    let mut ctx = tera::Context::new();
    ctx.insert("pascal", pascal);
    ctx.insert("snake", snake);
    ctx.insert("plural_pascal", &to_pascal_case(&plural));
    ctx.insert("plural", &plural);
    ctx
}

/// The module name of one scaffolded use case: `list` is named after the plural
/// (`list_products`), the others after the entity (`create_product`).
pub fn crud_use_case(action: &str, snake: &str) -> String {
    if action == "list" {
        format!("list_{}", pluralize(snake))
    } else {
        format!("{action}_{snake}")
    }
}

/// The five use cases `scaffold` and `generate domain` create for an entity.
pub fn crud_use_cases(snake: &str) -> Vec<String> {
    ["create", "get", "list", "update", "delete"]
        .iter()
        .map(|action| crud_use_case(action, snake))
        .collect()
}

/// Writes `content` to `path`, creating parent directories. The error names the path — a bare
/// `Permission denied (os error 13)` does not tell the user which file.
pub fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    let with_path = |e: std::io::Error| {
        std::io::Error::new(e.kind(), format!("cannot write {}: {e}", path.display()))
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(with_path)?;
    }
    fs::write(path, content).map_err(with_path)
}
