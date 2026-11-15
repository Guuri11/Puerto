//! One file per use case (ADR-0023 § 4): its contract — params, trait, double — and its
//! implementation with its tests, rendered by two templates and joined under one set of imports.

use std::collections::BTreeMap;

/// `contract` then `implementation`, their leading `use` statements merged: every imported item
/// once, grouped by the module it comes from. Two `use` lines naming the same item is E0252.
pub fn merge_use_case(contract: &str, implementation: &str) -> String {
    let (contract_docs, contract) = split_docs(contract);
    let (impl_docs, implementation) = split_docs(implementation);
    let (contract_uses, contract_body) = split_imports(contract);
    let (impl_uses, impl_body) = split_imports(implementation);
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for statement in contract_uses.iter().chain(&impl_uses) {
        for path in flatten(statement) {
            let (module, item) = match path.rsplit_once("::") {
                Some((module, item)) => (module.to_string(), item.to_string()),
                None => (String::new(), path.clone()),
            };
            let items = groups.entry(module).or_default();
            if !items.contains(&item) {
                items.push(item);
            }
        }
    }
    // The files' inner doc comments (`//!`) open the merged one: nothing may precede them.
    let mut out: String = [contract_docs, impl_docs]
        .iter()
        .filter(|d| !d.is_empty())
        .map(|d| format!("{d}\n"))
        .collect::<Vec<_>>()
        .join("//!\n");
    if !out.is_empty() {
        out.push('\n');
    }
    for (module, mut items) in groups {
        items.sort();
        let line = match (module.is_empty(), items.as_slice()) {
            (true, _) => items
                .iter()
                .map(|i| format!("use {i};\n"))
                .collect::<String>(),
            (false, [single]) => format!("use {module}::{single};\n"),
            (false, _) => format!("use {module}::{{{}}};\n", items.join(", ")),
        };
        out.push_str(&line);
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(contract_body.trim_end());
    out.push_str("\n\n");
    out.push_str(impl_body.trim_start());
    out
}

/// The leading `//!` lines (without their trailing newline) and what follows them.
fn split_docs(source: &str) -> (String, &str) {
    let mut docs = vec![];
    let mut rest = source;
    while let Some(line) = rest.trim_start_matches('\n').lines().next() {
        if !line.starts_with("//!") {
            break;
        }
        docs.push(line);
        let at = rest.find(line).unwrap_or(0) + line.len();
        rest = &rest[at..];
    }
    (docs.join("\n"), rest)
}

/// The leading `use …;` statements (without `use` and `;`) and what follows them.
fn split_imports(source: &str) -> (Vec<String>, &str) {
    let mut statements = vec![];
    let mut rest = source;
    loop {
        let trimmed = rest.trim_start();
        let Some(after) = trimmed.strip_prefix("use ") else {
            return (statements, trimmed);
        };
        let Some(end) = after.find(';') else {
            return (statements, trimmed);
        };
        statements.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
}

/// `a::{b, c::{d, e}}` → `a::b`, `a::c::d`, `a::c::e`.
fn flatten(tree: &str) -> Vec<String> {
    let tree: String = tree.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = vec![];
    expand("", tree.trim(), &mut out);
    out
}

fn expand(prefix: &str, tree: &str, out: &mut Vec<String>) {
    let join = |rest: &str| {
        if prefix.is_empty() {
            rest.to_string()
        } else {
            format!("{prefix}::{rest}")
        }
    };
    match tree.find('{') {
        None => {
            let tree = tree.trim();
            if tree == "self" {
                out.push(prefix.to_string());
            } else if !tree.is_empty() {
                out.push(join(tree));
            }
        }
        Some(open) => {
            let head = tree[..open].trim().trim_end_matches("::");
            let inner = &tree[open + 1..tree.rfind('}').unwrap_or(tree.len())];
            let prefix = if head.is_empty() {
                prefix.to_string()
            } else {
                join(head)
            };
            for part in split_top_level(inner) {
                expand(&prefix, part, out);
            }
        }
    }
}

/// `inner` split on the commas outside braces.
fn split_top_level(inner: &str) -> Vec<&str> {
    let mut parts = vec![];
    let (mut depth, mut start) = (0, 0);
    for (i, ch) in inner.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    parts.into_iter().filter(|p| !p.trim().is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_import_each_item_once_grouped_by_module() {
        let contract = "use async_trait::async_trait;\nuse dominio_p::{errores::PError, modelo::P};\n\npub struct Params;\n";
        let implementation = "use std::sync::Arc;\n\nuse async_trait::async_trait;\nuse dominio_p::{\n    errores::PError,\n    modelo::{P, PProps},\n};\n\npub struct Impl;\n";

        let merged = merge_use_case(contract, implementation);

        assert_eq!(
            merged,
            "use async_trait::async_trait;\nuse dominio_p::errores::PError;\n\
             use dominio_p::modelo::{P, PProps};\nuse std::sync::Arc;\n\n\
             pub struct Params;\n\npub struct Impl;\n"
        );
    }

    #[test]
    fn should_open_the_file_with_both_inner_doc_comments() {
        let merged = merge_use_case(
            "//! The contract.\n\nuse a::A;\n\npub struct P;\n",
            "//! Yours.\n//! Really.\n\nuse a::A;\n\npub struct I;\n",
        );

        assert_eq!(
            merged,
            "//! The contract.\n//!\n//! Yours.\n//! Really.\n\nuse a::A;\n\npub struct P;\n\npub struct I;\n"
        );
    }

    #[test]
    fn should_keep_self_and_aliases() {
        assert_eq!(
            flatten("a::{self, b as c, d::{e}}"),
            ["a", "a::b as c", "a::d::e"]
        );
    }

    #[test]
    fn should_leave_the_body_alone_when_there_are_no_imports() {
        assert_eq!(
            merge_use_case("struct A;\n", "struct B;\n"),
            "struct A;\n\nstruct B;\n"
        );
    }
}
