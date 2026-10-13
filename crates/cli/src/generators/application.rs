use std::path::Path;

use crate::generators::effective_fields;
use crate::generators::naming::{
    crud_use_case, entity_ctx, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{
    FieldCtx, field_ctx, is_value_object, literal_import, literal_names_vo, reference_import,
    vo_import_path,
};
use crate::patchers::lib_rs::patch_business_lib_application_crud;
use crate::puerto_toml::{Field, ValueObjectDefinition};

// ── Context ───────────────────────────────────────────────────────────────────

/// Which imports a use case's test module needs.
#[derive(Clone, Copy)]
enum TestImports {
    /// Create: its tests build `CreateEntityParams` literals only — always primitives — so no VO
    /// import, which would be unused.
    ParamsOnly,
    /// Get / list / update / delete: their tests build `EntityProps` literals, naming the VOs they
    /// construct. A missing id is `EntityId::generate()`, imported by the template.
    Props,
}

/// The context every application template renders from.
///
/// Imports are split by where they are used. The impls construct VOs (`Name::new(params.name)?`)
/// but never name a primitive type — the params carry those — so the file level gets VO imports
/// only; a primitive import there is dead in a non-test build (`unused_imports`, `-D warnings` in
/// the generated project's `make lint`). The test module is where a `Uuid`, `DateTime<Utc>` or
/// `HashMap` literal needs its type in scope (E0433 otherwise).
fn use_case_context(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    test_imports: TestImports,
) -> tera::Context {
    let eff = effective_fields(fields);

    let mut vo_imports: Vec<String> = vec![];
    for import in eff
        .iter()
        .filter(|f| is_value_object(f))
        .map(|f| vo_import_path(f, snake, shared_vos))
    {
        if !vo_imports.contains(&import) {
            vo_imports.push(import);
        }
    }

    let mut tests: Vec<String> = vec![];
    let literal_imports = eff.iter().filter_map(literal_import).map(str::to_string);
    let named_vos = eff
        .iter()
        .filter(|f| !matches!(test_imports, TestImports::ParamsOnly) && literal_names_vo(f))
        .map(|f| vo_import_path(f, snake, shared_vos));
    // `CustomerId::generate()` in Params and Props literals; an optional reference's is `None`.
    let required_references = eff
        .iter()
        .filter(|f| !f.field_type.starts_with("Option<"))
        .filter_map(|f| reference_import(f, "crate"));
    for import in literal_imports.chain(named_vos).chain(required_references) {
        if !tests.contains(&import) {
            tests.push(import);
        }
    }

    let field_ctxs: Vec<FieldCtx> = eff
        .iter()
        .map(|f| field_ctx(f, pascal, shared_vos))
        .collect();
    let mut ctx = entity_ctx(pascal, snake);
    ctx.insert("fields", &field_ctxs);
    ctx.insert("vo_imports", &vo_imports);
    ctx.insert("test_imports", &tests);
    ctx
}

fn render_use_case(
    action: &str,
    test_imports: TestImports,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case_as(
        action,
        test_imports,
        pascal,
        snake,
        fields,
        shared_vos,
        false,
    )
}

/// `actor`: a protected use case of an auth project — its tests pass a `PrincipalMother` caller.
fn render_use_case_as(
    action: &str,
    test_imports: TestImports,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    actor: bool,
) -> Result<String, RenderError> {
    let mut ctx = use_case_context(pascal, snake, fields, shared_vos, test_imports);
    ctx.insert("actor", &actor);
    render(&format!("application/{action}.tera"), &ctx)
}

/// The writes of an `events` project: through the unit of work, recording their event.
fn render_write_use_case_with_events(
    action: &str,
    test_imports: TestImports,
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
    actor: bool,
) -> Result<String, RenderError> {
    let mut ctx = use_case_context(pascal, snake, fields, shared_vos, test_imports);
    ctx.insert("actor", &actor);
    render(&format!("application/{action}_events.tera"), &ctx)
}

// ── Public generators ─────────────────────────────────────────────────────────

pub fn generate_create_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case(
        "create",
        TestImports::ParamsOnly,
        pascal,
        snake,
        fields,
        shared_vos,
    )
}

#[cfg(test)]
pub fn generate_get_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case("get", TestImports::Props, pascal, snake, fields, shared_vos)
}

#[cfg(test)]
pub fn generate_list_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case(
        "list",
        TestImports::Props,
        pascal,
        snake,
        fields,
        shared_vos,
    )
}

#[cfg(test)]
pub fn generate_update_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case(
        "update",
        TestImports::Props,
        pascal,
        snake,
        fields,
        shared_vos,
    )
}

#[cfg(test)]
pub fn generate_delete_use_case_impl(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<String, RenderError> {
    render_use_case(
        "delete",
        TestImports::Props,
        pascal,
        snake,
        fields,
        shared_vos,
    )
}

// ── File writers ──────────────────────────────────────────────────────────────

pub fn write_application_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base.join(format!("business/src/application/{snake}"));
    let access = crate::generators::auth::Access::of(base, pascal);
    let events = crate::puerto_toml::events_enabled(base);
    let use_cases = [
        ("create", TestImports::ParamsOnly),
        ("get", TestImports::Props),
        ("list", TestImports::Props),
        ("update", TestImports::Props),
        ("delete", TestImports::Props),
    ];
    for (action, imports) in use_cases {
        let use_case = crud_use_case(action, snake);
        let actor = access.is_protected(&use_case);
        // An `events` project's writes go through the unit of work, recording their event.
        let content = if events && matches!(action, "create" | "update" | "delete") {
            render_write_use_case_with_events(
                action, imports, pascal, snake, fields, shared_vos, actor,
            )?
        } else {
            render_use_case_as(action, imports, pascal, snake, fields, shared_vos, actor)?
        };
        write_file(&dir.join(format!("{use_case}.rs")), &content)?;
    }
    Ok(())
}

pub fn run_generate_application(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate domain {pascal}` first."
        )
        .into());
    }

    let fields: Vec<Field> = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();
    let shared_vos = config.value_object.clone();

    write_application_files(&pascal, &snake, base, &fields, &shared_vos)?;
    patch_business_lib_application_crud(base, &snake)?;

    println!("✓ business/application/ — 5 use case impls (create, get, list, update, delete)");
    println!();
    println!("  Next: puerto generate repository {pascal}");
    Ok(())
}
