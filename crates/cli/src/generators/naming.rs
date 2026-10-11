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
