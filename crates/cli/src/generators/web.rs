//! The frontend slice of an entity (`web/`, Phase 9 — ADR-0013).
//!
//! `scaffold` writes it next to the Rust layers in a `[project] web = true` project; `generate web`
//! writes it alone. The two files under `presentation/core/generated/` are regenerated from
//! puerto.toml with `bootstrap.rs`, never patched.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::generators::conflict::{OverwritePolicy, guard_files_overwrite};
use crate::generators::effective_fields;
use crate::generators::naming::{
    capitalize, crud_use_cases, pascal_to_snake, pluralize, snake_to_camel, snake_to_kebab,
    snake_to_words, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{WebEnumCtx, WebFieldCtx, web_enum_ctx, web_field_ctx};
use crate::puerto_toml::{Entity, Field, PuertoConfig};

/// Every frontend file `scaffold` writes for one entity, relative to the project root — what the
/// overwrite guard checks. `infrastructure/http/client.ts` is not one: it is written once, with
/// the first entity, and never overwritten.
pub fn web_entity_files(snake: &str) -> Vec<PathBuf> {
    let k = snake_to_kebab(snake);
    [
        format!("web/src/domain/{k}/model.ts"),
        format!("web/src/domain/{k}/errors.ts"),
        format!("web/src/domain/{k}/repository.ts"),
        format!("web/src/infrastructure/{k}/{k}-repository-http.ts"),
        format!("web/src/infrastructure/{k}/{k}-repository-http.test.ts"),
        format!("web/src/presentation/features/{k}/views/{k}-list-view.tsx"),
        format!("web/src/presentation/features/{k}/views/{k}-detail-view.tsx"),
        format!("web/src/presentation/features/{k}/views/{k}-form-view.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-list-container.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-list-container.test.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-detail-container.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-detail-container.test.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-form-container.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-form-container.test.tsx"),
        format!("web/src/test-support/{k}.ts"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// The frontend files `core/generated/*` import for one entity: without them `bun run typecheck`
/// fails, the way a missing module fails `cargo build` (see `consistency.rs`).
pub fn web_required_files(snake: &str) -> Vec<PathBuf> {
    let k = snake_to_kebab(snake);
    [
        format!("web/src/domain/{k}/repository.ts"),
        format!("web/src/infrastructure/{k}/{k}-repository-http.ts"),
        format!("web/src/presentation/features/{k}/containers/{k}-list-container.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-detail-container.tsx"),
        format!("web/src/presentation/features/{k}/containers/{k}-form-container.tsx"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// Whether `entity` gets a frontend slice: it has the five CRUD use cases, the endpoints the
/// screens call. The Greeting demo and custom-only entities have none.
pub fn has_web_slice(entity: &Entity) -> bool {
    web_slice_refusal(entity).is_none()
}

// ── Templates ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct WebEntityCtx {
    pascal: String,
    snake: String,
    kebab: String,
    camel: String,
    /// snake_case plural: REST paths (`/order_items`) and routes.
    plural: String,
    plural_camel: String,
    /// The local variable holding one entity (`orderItem`) and a list of them (`orderItems`).
    ident: String,
    plural_ident: String,
    /// `Order item` / `Order items` / `order item` / `order items`.
    label: String,
    label_plural: String,
    words: String,
    words_plural: String,
    /// `an order item`, `a product`.
    a_words: String,
}

/// TypeScript's reserved words, and the names the templates already give their own locals: an
/// entity named after one cannot use its camelCase name as a variable (`const case = …`).
const TAKEN_IDENTIFIERS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    // locals and parameters of the generated files
    "existing",
    "failure",
    "navigate",
    "remove",
    "saving",
    "values",
];

/// `name`, unless it is taken; then `name` + `suffix` (`case` → `caseItem`).
fn identifier(name: &str, suffix: &str) -> String {
    if TAKEN_IDENTIFIERS.contains(&name) {
        format!("{name}{suffix}")
    } else {
        name.to_string()
    }
}

/// `a` or `an` by the first letter — good enough for entity names (`an order item`, `a user`).
fn with_article(words: &str) -> String {
    let article = if words.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {words}")
}

fn web_entity_ctx(pascal: &str) -> WebEntityCtx {
    let snake = pascal_to_snake(pascal);
    let plural = pluralize(&snake);
    let ident = identifier(&snake_to_camel(&snake), "Item");
    let plural_camel = snake_to_camel(&plural);
    // An uncountable noun (`series`) has no plural to tell the list from one of its items.
    let plural_ident = if plural_camel == ident {
        format!("{plural_camel}List")
    } else {
        identifier(&plural_camel, "List")
    };
    WebEntityCtx {
        pascal: pascal.to_string(),
        kebab: snake_to_kebab(&snake),
        camel: snake_to_camel(&snake),
        ident,
        plural_ident,
        plural_camel,
        label: capitalize(&snake_to_words(&snake)),
        label_plural: capitalize(&snake_to_words(&plural)),
        a_words: with_article(&snake_to_words(&snake)),
        words: snake_to_words(&snake),
        words_plural: snake_to_words(&plural),
        plural,
        snake,
    }
}

fn entity_template_ctx(pascal: &str, fields: &[Field]) -> tera::Context {
    let entity = web_entity_ctx(pascal);
    let web_fields: Vec<WebFieldCtx> = effective_fields(fields)
        .iter()
        .map(|f| web_field_ctx(f, &entity.ident))
        .collect();
    let mut enums: Vec<WebEnumCtx> = vec![];
    for declared in effective_fields(fields)
        .iter()
        .filter_map(|f| web_enum_ctx(f, &entity.snake))
    {
        if !enums.iter().any(|e| e.name == declared.name) {
            enums.push(declared);
        }
    }
    // The `presentation/shared/forms.ts` helpers each group of files calls.
    let helpers = |pick: fn(&WebFieldCtx) -> &Vec<&'static str>| {
        let mut names: Vec<&str> = web_fields
            .iter()
            .flat_map(|f| pick(f).iter().copied())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    };
    let mut ctx = tera::Context::from_serialize(&entity).unwrap_or_default();
    ctx.insert("form_helpers", &{
        let mut names = helpers(|f| &f.to_form_helpers);
        names.extend(helpers(|f| &f.from_form_helpers));
        names.sort_unstable();
        names.dedup();
        names
    });
    ctx.insert("fill_helpers", &helpers(|f| &f.to_form_helpers));
    ctx.insert("display_helpers", &helpers(|f| &f.display_helpers));
    ctx.insert("enums", &enums);
    ctx.insert("fields", &web_fields);
    ctx
}

/// Renders a TypeScript template and orders its imports the way Biome's `organizeImports` does,
/// so `bun run lint` passes on generated code.
fn render_ts(template: &str, ctx: &tera::Context) -> Result<String, RenderError> {
    render(template, ctx).map(|ts| organize_imports(&ts))
}

/// One template per file of the slice, in `web_entity_files` order.
const ENTITY_TEMPLATES: [&str; 15] = [
    "web/model.tera",
    "web/errors.tera",
    "web/repository.tera",
    "web/repository_http.tera",
    "web/repository_http_test.tera",
    "web/list_view.tera",
    "web/detail_view.tera",
    "web/form_view.tera",
    "web/list_container.tera",
    "web/list_container_test.tera",
    "web/detail_container.tera",
    "web/detail_container_test.tera",
    "web/form_container.tera",
    "web/form_container_test.tera",
    "web/test_support.tera",
];

/// Writes the entity's frontend slice, and `infrastructure/http/client.ts` when it is missing.
///
/// The client is not in the `puerto new` template: it imports `schema.d.ts`, which exists only
/// after `make web/sync`, and a fresh project's `bun run typecheck` must pass. The first entity
/// needs a sync anyway.
pub fn write_web_files(
    pascal: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = entity_template_ctx(pascal, fields);
    let snake = pascal_to_snake(pascal);
    for (path, template) in web_entity_files(&snake).iter().zip(ENTITY_TEMPLATES) {
        write_file(&base.join(path), &render_ts(template, &ctx)?)?;
    }
    let client = base.join("web/src/infrastructure/http/client.ts");
    if !client.exists() {
        write_file(
            &client,
            &render_ts("web/client.tera", &tera::Context::new())?,
        )?;
    }
    Ok(())
}

/// An entity in `core/generated/routes.tsx`, with the line-width decisions Biome would make.
#[derive(Serialize)]
struct RoutedEntityCtx {
    #[serde(flatten)]
    entity: WebEntityCtx,
    /// `<Route path="/…/:id/edit">{(params) => …}</Route>` fits on one line.
    edit_route_inline: bool,
    detail_route_inline: bool,
}

/// Indentation of a `<Route>` inside `AppRoutes`.
const ROUTE_INDENT: usize = 6;

fn fits(line: &str) -> bool {
    line.chars().count() <= LINE_WIDTH
}

fn generated_ctx(entities: &[Entity]) -> tera::Context {
    let mut sliced: Vec<RoutedEntityCtx> = entities
        .iter()
        .filter(|e| has_web_slice(e))
        .map(|e| {
            let entity = web_entity_ctx(&e.name);
            let route = |suffix: &str, container: &str| {
                fits(&format!(
                    "{:ROUTE_INDENT$}<Route path=\"/{}/:id{suffix}\">{{(params) => <{}{container} id={{params.id}} />}}</Route>",
                    "", entity.plural, entity.pascal
                ))
            };
            RoutedEntityCtx {
                edit_route_inline: route("/edit", "FormContainer"),
                detail_route_inline: route("", "DetailContainer"),
                entity,
            }
        })
        .collect();
    sliced.sort_by(|a, b| compare_sources(&a.entity.kebab, &b.entity.kebab));
    // Biome breaks an array of two or more objects; a single one stays inline while it fits.
    let nav_inline = match sliced.as_slice() {
        [] => true,
        [only] => fits(&format!(
            "export const navEntries: NavEntry[] = [{{ label: \"{}\", href: \"/{}\" }}];",
            only.entity.label_plural, only.entity.plural
        )),
        _ => false,
    };
    let mut ctx = tera::Context::new();
    ctx.insert("entities", &sliced);
    ctx.insert("nav_inline", &nav_inline);
    ctx
}

/// `web/src/presentation/core/generated/dependencies.ts` — the composition root.
pub fn generate_web_dependencies(entities: &[Entity]) -> Result<String, RenderError> {
    render_ts("web/dependencies.tera", &generated_ctx(entities))
}

/// `web/src/presentation/core/generated/routes.tsx` — the route table and the nav entries.
pub fn generate_web_routes(entities: &[Entity]) -> Result<String, RenderError> {
    render_ts("web/routes.tera", &generated_ctx(entities))
}

/// Rewrites both `core/generated/*` files from puerto.toml. Nothing in a project without `web`.
pub fn regenerate_web(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.project.web {
        return Ok(());
    }
    let generated = base.join("web/src/presentation/core/generated");
    write_file(
        &generated.join("dependencies.ts"),
        &generate_web_dependencies(&config.entity)?,
    )?;
    write_file(
        &generated.join("routes.tsx"),
        &generate_web_routes(&config.entity)?,
    )?;
    Ok(())
}

/// Why `entity` gets no frontend slice in a web project, or `None` when it gets one.
pub fn web_slice_refusal(entity: &Entity) -> Option<String> {
    let snake = pascal_to_snake(&entity.name);
    let missing: Vec<String> = crud_use_cases(&snake)
        .into_iter()
        .filter(|uc| !entity.use_cases.contains(uc))
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "it has no CRUD endpoints for the screens to call (missing {})",
            missing.join(", ")
        ));
    }
    None
}

/// `puerto generate web <Name>` — the frontend slice alone, plus the `core/generated/*` files.
pub fn run_generate_web(
    name: &str,
    base: &Path,
    overwrite: OverwritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    if !config.project.web {
        return Err(
            "this project has no frontend ([project] web is not true in puerto.toml)".into(),
        );
    }
    let Some(entity) = config.entity.iter().find(|e| e.name == pascal) else {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate scaffold {pascal}` first."
        )
        .into());
    };
    if let Some(reason) = web_slice_refusal(entity) {
        return Err(format!("no web slice for {pascal}: {reason}").into());
    }
    let snake = pascal_to_snake(&pascal);
    guard_files_overwrite(base, &snake, &web_entity_files(&snake), overwrite)?;

    write_web_files(&pascal, base, &entity.fields)?;
    regenerate_web(base, &config)?;

    let kebab = snake_to_kebab(&snake);
    println!("✓ web/src/          — domain/{kebab}, infrastructure/{kebab}, features/{kebab}");
    println!("✓ core/generated/   — dependencies.ts, routes.tsx");
    println!("\nNext: make web/sync   (exports the spec, regenerates the web types)");
    Ok(())
}

// ── Import order ──────────────────────────────────────────────────────────────

/// One `import … from "…";` line as the templates write it.
#[derive(Debug, PartialEq)]
struct Import {
    /// `import type { … }`.
    type_only: bool,
    /// `import createClient, { … }`.
    default: Option<String>,
    /// The named specifiers, each with its own `type ` when it has one.
    names: Vec<String>,
    source: String,
}

fn parse_import(line: &str) -> Option<Import> {
    let rest = line.strip_prefix("import ")?.strip_suffix(';')?;
    let (clause, source) = rest.rsplit_once(" from ")?;
    let source = source.strip_prefix('"')?.strip_suffix('"')?.to_string();
    let (type_only, clause) = match clause.strip_prefix("type ") {
        Some(c) => (true, c),
        None => (false, clause),
    };
    let (default, named) = match clause.find('{') {
        Some(0) => (None, clause),
        Some(i) => (
            Some(clause[..i].trim_end().trim_end_matches(',').to_string()),
            &clause[i..],
        ),
        None => (Some(clause.to_string()), ""),
    };
    let names = named
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(String::from)
        .collect();
    Some(Import {
        type_only,
        default,
        names,
        source,
    })
}

/// Biome's group of an import source: `bun:` protocols, then packages, then relative paths —
/// those farthest up the tree (`../../`) first, `./` last.
fn source_rank(source: &str) -> (u8, isize) {
    if source.starts_with("./") {
        (2, 0)
    } else if source.starts_with("../") {
        (2, -(source.matches("../").count() as isize))
    } else if source.contains(':') {
        (0, 0)
    } else {
        (1, 0)
    }
}

/// Compares sources within a group: `/` before any other character, so a directory sorts before
/// its longer siblings (`order/` before `order-item/`).
fn compare_sources(a: &str, b: &str) -> Ordering {
    source_rank(a).cmp(&source_rank(b)).then_with(|| {
        let key = |s: &str| {
            s.chars()
                .map(|c| if c == '/' { '\0' } else { c })
                .collect::<String>()
        };
        key(a).cmp(&key(b))
    })
}

/// Specifiers sort as Biome's natural order does, ignoring their `type ` keyword: letter by letter
/// case-insensitively, except that the same letter in upper case comes first — `Product` before
/// `parseStatus`, but `fakeOrder` before `OrderMother`.
fn specifier_key(name: &str) -> Vec<(char, bool)> {
    name.trim_start_matches("type ")
        .chars()
        .map(|c| (c.to_ascii_lowercase(), c.is_ascii_lowercase()))
        .collect()
}

/// Biome's `lineWidth` in the template's `biome.json`.
const LINE_WIDTH: usize = 100;

fn print_import(import: &Import) -> String {
    let keyword = if import.type_only {
        "import type"
    } else {
        "import"
    };
    let default = import.default.as_deref().map(|d| {
        if import.names.is_empty() {
            format!("{d} ")
        } else {
            format!("{d}, ")
        }
    });
    let default = default.unwrap_or_default();
    if import.names.is_empty() {
        return format!("{keyword} {default}from \"{}\";", import.source);
    }
    let one_line = format!(
        "{keyword} {default}{{ {} }} from \"{}\";",
        import.names.join(", "),
        import.source
    );
    // Like Prettier, Biome never breaks an import of a single specifier, however long.
    if one_line.chars().count() <= LINE_WIDTH || import.names.len() == 1 {
        return one_line;
    }
    let names: String = import.names.iter().map(|n| format!("  {n},\n")).collect();
    format!(
        "{keyword} {default}{{\n{names}}} from \"{}\";",
        import.source
    )
}

/// Sorts the contiguous block of single-line imports in `ts` (sources, then the specifiers of each)
/// and wraps the ones over the line width — what `biome check` expects. Everything else is left
/// as rendered.
pub fn organize_imports(ts: &str) -> String {
    let lines: Vec<&str> = ts.split_inclusive('\n').collect();
    let Some(start) = lines.iter().position(|l| l.starts_with("import ")) else {
        return ts.to_string();
    };
    let end = lines[start..]
        .iter()
        .position(|l| !l.starts_with("import "))
        .map_or(lines.len(), |n| start + n);
    let parsed: Option<Vec<Import>> = lines[start..end]
        .iter()
        .map(|l| parse_import(l.trim_end()))
        .collect();
    let Some(mut imports) = parsed else {
        return ts.to_string();
    };
    for import in &mut imports {
        import.names.sort_by_key(|name| specifier_key(name));
    }
    imports.sort_by(|a, b| compare_sources(&a.source, &b.source));

    let mut out: String = lines[..start].concat();
    for import in &imports {
        out.push_str(&print_import(import));
        out.push('\n');
    }
    out.push_str(&lines[end..].concat());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_sort_relative_imports_by_distance_then_path() {
        let ts = "import { b } from \"./b\";\nimport { a } from \"../x/a\";\nimport { c } from \"../../c\";\nimport { Route } from \"wouter\";\nimport { test } from \"bun:test\";\n\nconst x = 1;\n";

        assert_eq!(
            organize_imports(ts),
            "import { test } from \"bun:test\";\nimport { Route } from \"wouter\";\nimport { c } from \"../../c\";\nimport { a } from \"../x/a\";\nimport { b } from \"./b\";\n\nconst x = 1;\n"
        );
    }

    #[test]
    fn should_sort_a_directory_before_its_longer_siblings() {
        assert_eq!(
            compare_sources("../features/order/routes", "../features/order-item/routes"),
            Ordering::Less
        );
        assert_eq!(
            compare_sources("../test-support/product", "../test-support/render"),
            Ordering::Less
        );
        assert_eq!(
            compare_sources("../test-support/shipment", "../test-support/render"),
            Ordering::Greater
        );
    }

    #[test]
    fn should_sort_specifiers_case_insensitively_ignoring_type() {
        let ts = "import { toUserForm, UserFormView, type Apple } from \"./x\";\n";

        assert_eq!(
            organize_imports(ts),
            "import { type Apple, toUserForm, UserFormView } from \"./x\";\n"
        );
    }

    #[test]
    fn should_put_the_upper_case_letter_first_when_specifiers_differ_only_in_case_there() {
        // Biome's order: `parseStatus` before `type Product` failed `bun run lint`.
        let ts = "import { parseStatus, type Product, type ProductDraft } from \"./model\";\n";

        assert_eq!(
            organize_imports(ts),
            "import { type Product, type ProductDraft, parseStatus } from \"./model\";\n"
        );
    }

    #[test]
    fn should_wrap_an_import_over_the_line_width_unless_it_has_one_specifier() {
        let long = "import { InventoryAdjustmentFormView, toInventoryAdjustmentDraft } from \"../views/inventory-adjustment-form-view\";\n";
        let single = "import { InventoryAdjustmentDetailContainer } from \"../../features/inventory-adjustment/containers/inventory-adjustment-detail-container\";\n";

        assert_eq!(
            organize_imports(long),
            "import {\n  InventoryAdjustmentFormView,\n  toInventoryAdjustmentDraft,\n} from \"../views/inventory-adjustment-form-view\";\n"
        );
        assert_eq!(organize_imports(single), single);
    }

    #[test]
    fn should_keep_default_and_type_only_imports() {
        let ts = "import type { paths } from \"./schema\";\nimport createClient, { type Client } from \"openapi-fetch\";\n";

        assert_eq!(
            organize_imports(ts),
            "import createClient, { type Client } from \"openapi-fetch\";\nimport type { paths } from \"./schema\";\n"
        );
    }

    #[test]
    fn should_leave_text_after_the_header_comment_alone() {
        let ts = "// header\n\nimport { b } from \"./b\";\nimport { a } from \"./a\";\n";

        assert_eq!(
            organize_imports(ts),
            "// header\n\nimport { a } from \"./a\";\nimport { b } from \"./b\";\n"
        );
    }

    fn entity(name: &str, use_cases: Vec<String>, fields: Vec<Field>) -> Entity {
        Entity {
            name: name.into(),
            use_cases,
            db: false,
            fields,
        }
    }

    #[test]
    fn should_not_name_a_variable_after_a_reserved_word_or_an_uncountable_plural() {
        let case = web_entity_ctx("Case");
        assert_eq!(
            (case.ident.as_str(), case.plural_ident.as_str()),
            ("caseItem", "cases")
        );

        let series = web_entity_ctx("Series");
        assert_eq!(
            (series.ident.as_str(), series.plural_ident.as_str()),
            ("series", "seriesList")
        );

        assert_eq!(web_entity_ctx("OrderItem").a_words, "an order item");
        assert_eq!(web_entity_ctx("Product").a_words, "a product");

        let item = web_entity_ctx("OrderItem");
        assert_eq!(
            (item.ident.as_str(), item.plural_ident.as_str()),
            ("orderItem", "orderItems")
        );
    }

    #[test]
    fn should_slice_only_crud_entities_whatever_their_fields() {
        let crud = crud_use_cases("order_item");
        assert!(has_web_slice(&entity("OrderItem", crud.clone(), vec![])));
        assert!(!has_web_slice(&entity(
            "Greeting",
            vec!["get_greeting".into()],
            vec![]
        )));
        let date = Field {
            name: "due".into(),
            field_type: "DateTime<Utc>".into(),
            ..Default::default()
        };
        assert!(has_web_slice(&entity("OrderItem", crud, vec![date])));
        assert!(
            web_slice_refusal(&entity("Greeting", vec!["get_greeting".into()], vec![]))
                .is_some_and(|r| r.contains("create_greeting"))
        );
    }
}
