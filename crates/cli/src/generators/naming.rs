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

pub fn apply(template: &str, pascal: &str, snake: &str) -> String {
    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
}

pub fn apply_uc(template: &str, pascal: &str, snake: &str, uc_pascal: &str, uc: &str) -> String {
    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
        .replace("{uc_pascal}", uc_pascal)
        .replace("{uc}", uc)
}

pub fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}
