use crate::generators::layout::{self, Layer};
use serde_json::json;
use std::path::Path;

use crate::generators::auth::Access;
use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::crates::ContextDeps;
use crate::generators::effective_fields;
use crate::generators::naming::{
    crud_use_case, crud_use_cases, entity_ctx, pascal_to_snake, to_pascal_case, write_file,
};
use crate::generators::render::render;
use crate::generators::types::{FieldCtx, field_ctx, literal_import, resolve_type};
use crate::puerto_toml::Field;

pub(crate) fn build_dto_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    is_crud: bool,
) -> tera::Context {
    let eff = effective_fields(fields);

    // DTO fields keep the primitive type of the field, so the DTO file needs the same imports the
    // domain model does — except `uuid::Uuid`, which `dto.tera` imports itself for the
    // always-present `id`. Without them a `DateTime<Utc>` or `HashMap` field names a type that was
    // never in scope (E0425).
    let mut imports: Vec<&str> = vec![];
    for import in eff.iter().filter_map(|f| {
        resolve_type(&f.field_type)
            .ok()
            .and_then(|m| m.needs_import)
    }) {
        if import != "uuid::Uuid" && !imports.contains(&import) {
            imports.push(import);
        }
    }

    let mut ctx = simple_ctx(pascal, snake, &eff);
    ctx.insert("is_crud", &is_crud);
    ctx.insert("imports", &imports);
    ctx
}

pub(crate) fn build_error_mapper_ctx(pascal: &str, snake: &str, fields: &[Field]) -> tera::Context {
    let eff = effective_fields(fields);
    let mut vos: Vec<serde_json::Value> = vec![];
    for vo in eff.iter().filter_map(|f| f.value_object.as_deref()) {
        let entry = json!({ "vo": vo, "vo_snake": pascal_to_snake(vo) });
        if !vos.contains(&entry) {
            vos.push(entry);
        }
    }
    let mut ctx = build_simple_ctx(pascal, snake);
    ctx.insert("vos", &vos);
    ctx
}

/// `pascal`, `snake` and one [`FieldCtx`] per field.
fn simple_ctx(pascal: &str, snake: &str, eff: &[Field]) -> tera::Context {
    let fields: Vec<FieldCtx> = eff.iter().map(|f| field_ctx(f, pascal, &[])).collect();
    let mut ctx = build_simple_ctx(pascal, snake);
    crate::generators::types::insert_field_lists(&mut ctx, &fields);
    ctx
}

pub(crate) fn build_simple_ctx(pascal: &str, snake: &str) -> tera::Context {
    entity_ctx(pascal, snake)
}

/// Who may call each CRUD route (ADR-0021): `access.<action>.protected` / `.roles` / `.optional`
/// (a public route that answers the entity, with private fields: it takes `BearerAuthOpcional`),
/// `bearer` when one route takes `BearerAuth`, `optional_auth` when one takes
/// `BearerAuthOpcional`, `auth` when either does (the route tests sign in), `any_role` when one
/// requires a role, and `caller_roles` — every role, held by the route tests' caller.
fn insert_access(ctx: &mut tera::Context, snake: &str, access: &Access, private: bool) {
    let mut per_action = serde_json::Map::new();
    let mut optional_auth = false;
    for action in ["create", "get", "list", "update", "delete"] {
        let use_case = crud_use_case(action, snake);
        let protected = access.is_protected(&use_case);
        // A deletion answers no entity.
        let optional = private && !protected && action != "delete";
        optional_auth |= optional;
        per_action.insert(
            action.into(),
            json!({ "protected": protected, "roles": access.roles(&use_case), "optional": optional }),
        );
    }
    let use_cases = crud_use_cases(snake);
    let bearer = use_cases.iter().any(|uc| access.is_protected(uc));
    ctx.insert("access", &per_action);
    ctx.insert("bearer", &bearer);
    ctx.insert("optional_auth", &optional_auth);
    ctx.insert("auth", &(bearer || optional_auth));
    ctx.insert(
        "any_role",
        &use_cases.iter().any(|uc| !access.roles(uc).is_empty()),
    );
    ctx.insert("caller_roles", &access.all_roles(&use_cases));
}

fn build_routes_crud_ctx(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    access: &Access,
) -> tera::Context {
    let eff = effective_fields(fields);
    // The route tests build request bodies from the registry defaults (`Utc::now()`,
    // `HashMap::new()`, …). `uuid::Uuid` is already imported by the routes themselves.
    let mut test_imports: Vec<&str> = vec![];
    for import in eff.iter().filter_map(literal_import) {
        if import != "uuid::Uuid" && !test_imports.contains(&import) {
            test_imports.push(import);
        }
    }
    let mut reference_imports = vec![];
    crate::generators::domain::push_reference_imports(&mut reference_imports, &eff);
    let mut ctx = simple_ctx(pascal, snake, &eff);
    ctx.insert("test_imports", &test_imports);
    ctx.insert("reference_imports", &reference_imports);
    insert_access(&mut ctx, snake, access, eff.iter().any(|f| f.private));
    let private_fields: Vec<String> = eff
        .iter()
        .filter(|f| f.private)
        .map(|f| crate::generators::naming::snake_to_camel(&f.name))
        .collect();
    ctx.insert("private_fields", &private_fields);
    ctx
}

#[cfg(test)]
pub fn generate_crud_dto(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, crate::generators::render::RenderError> {
    render(
        "presentation/dto.tera",
        &build_dto_ctx(pascal, snake, fields, true),
    )
}

#[cfg(test)]
pub fn generate_crud_routes(
    pascal: &str,
    snake: &str,
    fields: &[Field],
) -> Result<String, crate::generators::render::RenderError> {
    render(
        "presentation/routes_crud.tera",
        &build_routes_crud_ctx(pascal, snake, fields, &Access::default()),
    )
}

/// The context's REST crate: DTOs, responses, the error mapping and the routes with their tests,
/// then its `Cargo.toml` and `lib.rs`.
pub fn write_presentation_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    let access = Access::of(base, pascal);
    let mut responses_ctx = build_simple_ctx(pascal, snake);
    insert_access(
        &mut responses_ctx,
        snake,
        &access,
        effective_fields(fields).iter().any(|f| f.private),
    );
    let mut error_mapper_ctx = build_error_mapper_ctx(pascal, snake, fields);
    // `{Entity}Error::Forbidden` exists in every CRUD entity of an auth project.
    error_mapper_ctx.insert("auth", &access.is_auth());
    write_file(
        &base.join(layout::rest_file(snake, "dto")),
        &render(
            "presentation/dto.tera",
            &build_dto_ctx(pascal, snake, fields, true),
        )?,
    )?;
    write_file(
        &base.join(layout::rest_file(snake, "respuestas")),
        &render("presentation/responses_crud.tera", &responses_ctx)?,
    )?;
    write_file(
        &base.join(layout::rest_file(snake, "errores")),
        &render("presentation/error_mapper.tera", &error_mapper_ctx)?,
    )?;
    write_file(
        &base.join(layout::rest_file(snake, "rutas")),
        &render(
            "presentation/routes_crud.tera",
            &build_routes_crud_ctx(pascal, snake, fields, &access),
        )?,
    )?;
    crate::generators::crates::finish(base, snake, Layer::Rest, &ContextDeps::read(base, fields))
}

pub fn run_generate_presentation(
    name: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} no está en puerto.toml. Ejecuta antes `puerto generar dominio {pascal}`."
        )
        .into());
    }

    // The routes hold the use cases' traits, which live in the application crate (ADR-0023 § 4).
    if !base
        .join(layout::crate_manifest(&snake, Layer::Application))
        .exists()
    {
        return Err(format!(
            "{pascal} aún no tiene crate de aplicación, y las rutas guardan sus casos de uso. \
             Ejecuta antes `puerto generar aplicacion {pascal}`."
        )
        .into());
    }

    let fields = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_presentation_files(&pascal, &snake, base, &fields)?;
    crate::patchers::routes_rs::patch_routes_from_manifest(base, &pascal, &snake)?;
    println!(
        "✓ {}/: rutas, dto, respuestas, errores",
        layout::crate_dir(&snake, Layer::Rest)
    );

    // The routes only need the domain and the application crates. The wiring also names the
    // repository adapter, so it waits for it — written now, it would not compile.
    let entity = config.entity.iter().find(|e| e.name == pascal);
    let missing: Vec<_> = entity
        .map(crate::generators::consistency::bootstrap_required_files)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| !base.join(path).exists())
        .collect();
    if missing.is_empty() {
        regenerate_bootstrap(base)?;
        println!("✓ bootstrap.rs regenerado");
        println!();
        println!("  Todas las capas cableadas. Arranca con `make run`.");
        return Ok(());
    }

    let next = "repositorio";
    println!();
    println!("  bootstrap.rs aún sin regenerar: {pascal} no tiene la capa {next}.");
    println!("  Siguiente: puerto generar {next} {pascal}, y después `puerto generar bootstrap`.");
    Ok(())
}
