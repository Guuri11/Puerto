//! The frontend slice of an entity (`web/`, Phase 9 — ADR-0013, ADR-0025).
//!
//! `scaffold` writes it next to the Rust layers in a `[proyecto] web = true` project; `generate web`
//! writes it alone. The files under `generado/` are regenerated from puerto.toml with
//! `bootstrap.rs`, never patched. The screens work on the API's own types (ADR-0025).

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::generators::auth::Access;
use crate::generators::conflict::{OverwritePolicy, guard_files_overwrite};
use crate::generators::effective_fields;
use crate::generators::naming::{
    capitalize, crud_use_case, crud_use_cases, pascal_to_snake, pluralize, snake_to_camel,
    snake_to_kebab, snake_to_words, to_pascal_case, write_file,
};
use crate::generators::render::{RenderError, render};
use crate::generators::types::{WebFieldCtx, web_field_ctx};
use crate::puerto_toml::{Entity, Field, PuertoConfig};

/// Every frontend file `scaffold` writes for one entity, relative to the project root — what the
/// overwrite guard checks. `api/cliente.ts` and `pruebas/api-falsa.ts` are not: they are written
/// once, with the first entity, and never overwritten.
pub fn web_entity_files(snake: &str) -> Vec<PathBuf> {
    let k = snake_to_kebab(snake);
    [
        format!("web/src/contextos/{k}/mensajes.ts"),
        format!("web/src/contextos/{k}/vistas/lista-{k}.tsx"),
        format!("web/src/contextos/{k}/vistas/detalle-{k}.tsx"),
        format!("web/src/contextos/{k}/vistas/formulario-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/lista-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/lista-{k}.test.tsx"),
        format!("web/src/contextos/{k}/contenedores/detalle-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/detalle-{k}.test.tsx"),
        format!("web/src/contextos/{k}/contenedores/formulario-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/formulario-{k}.test.tsx"),
        format!("web/src/pruebas/{k}.ts"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// The frontend files `generado/rutas.tsx` imports for one entity: without them
/// `bun run typecheck` fails, the way a missing module fails `cargo build` (see `consistency.rs`).
pub fn web_required_files(snake: &str) -> Vec<PathBuf> {
    let k = snake_to_kebab(snake);
    [
        format!("web/src/contextos/{k}/contenedores/lista-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/detalle-{k}.tsx"),
        format!("web/src/contextos/{k}/contenedores/formulario-{k}.tsx"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// Written with the first entity, never overwritten: both import `api/esquema.d.ts`, which exists
/// only after `make web/sync`, and a fresh project's `bun run typecheck` must pass.
const ONCE_FILES: [(&str, &str); 2] = [
    ("web/src/api/cliente.ts", "web/client.tera"),
    ("web/src/pruebas/api-falsa.ts", "web/fake_api.tera"),
];

/// Whether `entity` gets a frontend slice: it has the five CRUD use cases, the endpoints the
/// screens call. The Saludo demo and custom-only entities have none.
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
    /// `Linea pedido` / `Lineas pedido` / `linea pedido` / `lineas pedido`: the screens' texts
    /// use them without an article, since the entity's gender is unknown (ADR-0024, Fase 22).
    label: String,
    label_plural: String,
    words: String,
    words_plural: String,
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
    "api",
    "borrar",
    "campo",
    "codigo",
    "confirmando",
    "consulta",
    "creado",
    "cuerpo",
    "desplazamiento",
    "dueno",
    "encontrado",
    "etiqueta",
    "existente",
    "fallo",
    "fallos",
    "guardado",
    "guardados",
    "guardando",
    "guardar",
    "historial",
    "navegar",
    "objetivo",
    "opciones",
    "pantalla",
    "params",
    "peticion",
    "problema",
    "ruta",
    "select",
    "semilla",
    "sembrado",
    "titulo",
    "ubicacion",
    "usuario",
    "valor",
    "valores",
];

/// `name`, unless it is taken; then `name` + `suffix` (`case` → `caseElemento`).
fn identifier(name: &str, suffix: &str) -> String {
    if TAKEN_IDENTIFIERS.contains(&name) {
        format!("{name}{suffix}")
    } else {
        name.to_string()
    }
}

fn web_entity_ctx(pascal: &str) -> WebEntityCtx {
    let snake = pascal_to_snake(pascal);
    let plural = pluralize(&snake);
    let ident = identifier(&snake_to_camel(&snake), "Elemento");
    let plural_camel = snake_to_camel(&plural);
    // An uncountable noun (`series`) has no plural to tell the list from one of its items.
    let plural_ident = if plural_camel == ident {
        format!("{plural_camel}Lista")
    } else {
        identifier(&plural_camel, "Lista")
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
        words: snake_to_words(&snake),
        words_plural: snake_to_words(&plural),
        plural,
        snake,
    }
}

/// What the screens of an auth project check (`PLAN.md` § 16.5): the roles that hide an action, and
/// whether deleting is protected (the use case may still answer `403`). Empty outside auth.
#[derive(Serialize)]
struct WebAccessCtx {
    auth: bool,
    create_roles: Vec<String>,
    update_roles: Vec<String>,
    delete_roles: Vec<String>,
    delete_protected: bool,
}

fn web_access_ctx(access: &Access, snake: &str) -> WebAccessCtx {
    let uc = |action: &str| crud_use_case(action, snake);
    WebAccessCtx {
        auth: access.is_auth(),
        create_roles: access.roles(&uc("create")),
        update_roles: access.roles(&uc("update")),
        delete_roles: access.roles(&uc("delete")),
        delete_protected: access.is_protected(&uc("delete")),
    }
}

/// An entity this one references, as its form loads it (ADR-0022): once, whatever the number of
/// references to it.
#[derive(Serialize)]
struct WebTargetCtx {
    #[serde(flatten)]
    entity: WebEntityCtx,
    /// What an option shows: the target's first required `String` field (camelCase), else `id`.
    label: String,
    /// The references to it, in field order (camelCase) — their selects share its options.
    fields: Vec<String>,
    /// The `useCarga(…)` loading it fits on one line.
    load_inline: bool,
    /// The form test's targets for `dueno`: one `Madre{Target}.aleatorio({ id })` per reference.
    seeds: Vec<String>,
    /// `coleccion{Target}([…seeds]),` fits on one line.
    seeds_inline: bool,
}

/// A reference as the detail screen shows it (Phase 20.6): a link named by its target's label,
/// which the detail container loads.
#[derive(Serialize)]
struct WebReferenceCtx {
    camel: String,
    /// What a screen calls the field, and the target's list path.
    label: String,
    ref_path: String,
    required: bool,
    /// The target's name in the tests (`MadreCliente`, `coleccionCliente`) and its label.
    target_pascal: String,
    target_label: String,
    /// What the detail view calls to name it: `etiquetaCliente`.
    loader: String,
    /// The view's `<EnlaceReferencia … />` fits on one line.
    link_inline: bool,
    /// The detail test's `const objetivo = Madre….aleatorio({ id: … });` fits on one line, and so
    /// does its `const api = apiFalsa([…]);`.
    target_inline: bool,
    api_inline: bool,
}

/// The entities `fields` reference, in the order they are first named. `config` gives their
/// fields; without it (or without the target in it) an option shows the id.
fn web_targets(fields: &[Field], config: Option<&PuertoConfig>) -> Vec<WebTargetCtx> {
    let mut targets: Vec<WebTargetCtx> = vec![];
    for f in fields {
        let Some(target) = f.references.as_deref() else {
            continue;
        };
        let camel = snake_to_camel(&f.name);
        // A Mother's reference is present; `??` only satisfies `Partial`.
        let seed =
            format!("Madre{target}.aleatorio({{ id: dueno.{camel} ?? crypto.randomUUID() }})");
        if let Some(known) = targets.iter_mut().find(|t| t.entity.pascal == target) {
            known.fields.push(camel);
            known.seeds.push(seed);
            continue;
        }
        let label = config
            .and_then(|c| c.entity.iter().find(|e| e.name == target))
            .and_then(|e| {
                effective_fields(&e.fields)
                    .into_iter()
                    .find(|t| t.field_type == "String" && !crate::generators::types::is_enum_vo(t))
            })
            .map_or_else(|| "id".to_string(), |t| snake_to_camel(&t.name));
        let entity = web_entity_ctx(target);
        let load_inline = fits(&format!(
            "  const {} = useCarga(() => desenvolver(api.GET(\"/{}\", PRIMERA_PAGINA)), [api]);",
            entity.plural_ident, entity.plural
        ));
        targets.push(WebTargetCtx {
            entity,
            label,
            fields: vec![camel],
            load_inline,
            seeds: vec![seed],
            seeds_inline: false,
        });
    }
    for t in &mut targets {
        t.seeds_inline = fits(&format!(
            "    coleccion{}([{}]),",
            t.entity.pascal,
            t.seeds.join(", ")
        ));
    }
    targets
}

/// Each reference of `entity`, as its detail screen and its test show it.
fn web_references(
    entity: &WebEntityCtx,
    web_fields: &[WebFieldCtx],
    fields: &[Field],
    targets: &[WebTargetCtx],
) -> Vec<WebReferenceCtx> {
    fields
        .iter()
        .zip(web_fields)
        .filter_map(|(field, web)| {
            let target = targets
                .iter()
                .find(|t| Some(t.entity.pascal.as_str()) == field.references.as_deref())?;
            let loader = format!("etiqueta{}", target.entity.pascal);
            let fallback = if web.required {
                ""
            } else {
                " ?? crypto.randomUUID()"
            };
            Some(WebReferenceCtx {
                link_inline: fits(&format!(
                    "        <EnlaceReferencia ruta=\"{}\" id={{{}.{}}} cargar={{{loader}}} />",
                    web.ref_path, entity.ident, web.camel
                )),
                target_inline: fits(&format!(
                    "    const objetivo = Madre{}.aleatorio({{ id: sembrado.{}{fallback} }});",
                    target.entity.pascal, web.camel
                )),
                api_inline: fits(&format!(
                    "    const api = apiFalsa([coleccion{}([sembrado]), coleccion{}([objetivo])]);",
                    entity.pascal, target.entity.pascal
                )),
                camel: web.camel.clone(),
                label: web.label.clone(),
                ref_path: web.ref_path.clone(),
                required: web.required,
                target_pascal: target.entity.pascal.clone(),
                target_label: target.label.clone(),
                loader,
            })
        })
        .collect()
}

fn entity_template_ctx(
    pascal: &str,
    fields: &[Field],
    access: &Access,
    config: Option<&PuertoConfig>,
) -> tera::Context {
    let entity = web_entity_ctx(pascal);
    let eff = effective_fields(fields);
    let web_fields: Vec<WebFieldCtx> = eff
        .iter()
        .map(|f| web_field_ctx(f, &entity.ident))
        .collect();
    // The `compartido/formularios.ts` helpers each group of files calls.
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
    ctx.insert("access", &web_access_ctx(access, &entity.snake));
    let targets = web_targets(&eff, config);
    let mut form_helpers = helpers(|f| &f.to_form_helpers);
    form_helpers.extend(helpers(|f| &f.from_form_helpers));
    form_helpers.sort_unstable();
    form_helpers.dedup();
    // Biome's `useImportType`: an import of types only is `import type { … }`.
    let form_helpers_type_only = form_helpers.is_empty() && !targets.is_empty();
    if !targets.is_empty() {
        form_helpers.push(if form_helpers_type_only {
            "OpcionReferencia"
        } else {
            "type OpcionReferencia"
        });
    }
    ctx.insert("form_helpers", &form_helpers);
    ctx.insert("form_helpers_type_only", &form_helpers_type_only);
    ctx.insert("fill_helpers", &helpers(|f| &f.to_form_helpers));
    ctx.insert("display_helpers", &helpers(|f| &f.display_helpers));
    let problems = web_problems(&entity, &eff, &web_fields);
    ctx.insert(
        "field_problem",
        &problems.iter().find(|p| p.field.is_some()),
    );
    ctx.insert("problems", &problems);
    ctx.insert("duplicate_text", DUPLICATE_TEXT);
    ctx.insert(
        "list_load_inline",
        &fits(&format!(
            "  const {} = useCarga(() => desenvolver(api.GET(\"/{}\", {{ params: {{ query: consulta }} }})), [api, desplazamiento]);",
            entity.plural_ident, entity.plural
        )),
    );
    ctx.insert(
        "detail_load_inline",
        &fits(&format!(
            "  const {} = useCarga(() => desenvolver(api.GET(\"/{}/{{id}}\", {{ params: {{ path: {{ id }} }} }})), [api, id]);",
            entity.ident, entity.plural
        )),
    );
    ctx.insert(
        "detail_view_inline",
        &fits(&format!(
            "            <VistaDetalle{} {}={{{}.datos}}{} />",
            entity.pascal,
            entity.ident,
            entity.ident,
            targets
                .iter()
                .map(|t| format!(" etiqueta{0}={{etiqueta{0}}}", t.entity.pascal))
                .collect::<String>()
        )),
    );
    // Signatures and calls whose length depends on the entity's name.
    ctx.insert(
        "save_inline",
        &fits(&format!(
            "          : api.PUT(\"/{}/{{id}}\", {{ params: {{ path: {{ id: existente.id }} }}, body: cuerpo }}),",
            entity.plural
        )),
    );
    ctx.insert(
        "to_form_inline",
        &fits(&format!(
            "export function aFormulario{0}({1}: {0} | null): ValoresFormulario{0} {{",
            entity.pascal, entity.ident
        )),
    );
    ctx.insert(
        "to_request_inline",
        &fits(&format!(
            "export function aPeticion{0}(valores: ValoresFormulario{0}): Peticion{0} {{",
            entity.pascal
        )),
    );
    // Biome keeps an array on one line while it fits: one target, few references.
    ctx.insert(
        "referenced_inline",
        &(targets.iter().all(|t| t.seeds_inline)
            && fits(&format!(
                "  return [{}];",
                targets
                    .iter()
                    .map(|t| format!("coleccion{}([{}])", t.entity.pascal, t.seeds.join(", ")))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
    );
    ctx.insert(
        "references",
        &web_references(&entity, &web_fields, &eff, &targets),
    );
    ctx.insert("fields", &web_fields);
    ctx.insert("targets", &targets);
    ctx
}

/// What the form says when the API answers a duplicate: no article, the entity's gender is unknown.
const DUPLICATE_TEXT: &str = "Estos valores ya están en uso";

/// An error code of the entity's API and what the screens say for it (`contextos/<k>/mensajes.ts`).
#[derive(Serialize)]
struct WebProblemCtx {
    code: String,
    /// The form field it is about (camelCase), shown under that input; `None` goes to the alert.
    field: Option<String>,
    /// That field's label — how a test finds its input.
    label: String,
    text: String,
}

/// The codes `errors.rs` declares, in its order: a required `String` field's `<field>_vacio`, a
/// value object's `<vo>_invalido`, then the entity-wide ones.
fn web_problems(
    entity: &WebEntityCtx,
    eff: &[Field],
    web_fields: &[WebFieldCtx],
) -> Vec<WebProblemCtx> {
    let snake = &entity.snake;
    let mut problems: Vec<WebProblemCtx> = vec![];
    for (f, web) in eff.iter().zip(web_fields) {
        // A checkbox has no message under it; its code (a bool VO never fails) goes to the alert.
        let field = (web.input != "checkbox").then(|| web.camel.clone());
        let mut add = |code: String, text: String| {
            if !problems.iter().any(|p| p.code == code) {
                problems.push(WebProblemCtx {
                    code,
                    field: field.clone(),
                    label: web.label.clone(),
                    text,
                });
            }
        };
        if f.field_type == "String" && f.value_object.is_none() {
            add(
                format!("{snake}.validacion.{}_vacio", f.name),
                format!("{}: obligatorio", web.label),
            );
        }
        if f.references.is_some() {
            add(
                format!("{snake}.validacion.{}_no_encontrado", f.name),
                format!("{}: no existe", web.label),
            );
        }
        if let Some(vo) = &f.value_object {
            add(
                format!("{snake}.{}_invalido", pascal_to_snake(vo)),
                format!("{}: no válido", web.label),
            );
        }
    }
    for (code, text) in [
        ("duplicado", DUPLICATE_TEXT),
        ("no_encontrado", "Ya no existe"),
        (
            "repositorio",
            "No se ha podido guardar; inténtalo más tarde",
        ),
        ("desconocido", "Algo ha ido mal"),
    ] {
        problems.push(WebProblemCtx {
            code: format!("{snake}.{code}"),
            field: None,
            label: String::new(),
            text: text.to_string(),
        });
    }
    problems
}

/// Renders a TypeScript template and orders its imports the way Biome's `organizeImports` does,
/// so `bun run lint` passes on generated code.
fn render_ts(template: &str, ctx: &tera::Context) -> Result<String, RenderError> {
    render(template, ctx).map(|ts| organize_imports(&ts))
}

/// One template per file of the slice, in `web_entity_files` order.
const ENTITY_TEMPLATES: [&str; 11] = [
    "web/messages.tera",
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

/// Writes the entity's frontend slice, and the [`ONCE_FILES`] that are missing.
pub fn write_web_files(
    pascal: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base).ok();
    let ctx = entity_template_ctx(pascal, fields, &Access::of(base, pascal), config.as_ref());
    let snake = pascal_to_snake(pascal);
    for (path, template) in web_entity_files(&snake).iter().zip(ENTITY_TEMPLATES) {
        write_file(&base.join(path), &render_ts(template, &ctx)?)?;
    }
    for (path, template) in ONCE_FILES {
        let path = base.join(path);
        if !path.exists() {
            write_file(&path, &render_ts(template, &tera::Context::new())?)?;
        }
    }
    Ok(())
}

/// An entity in `generado/rutas.tsx`, with the line-width decisions Biome would make.
#[derive(Serialize)]
struct RoutedEntityCtx {
    #[serde(flatten)]
    entity: WebEntityCtx,
    /// `<Route path="/…/:id/editar">{(params) => …}</Route>` fits on one line.
    edit_route_inline: bool,
    detail_route_inline: bool,
    /// The screen sits behind `<RequiereAuth>`: a use case it calls is protected (auth projects).
    list_guarded: bool,
    new_guarded: bool,
    edit_guarded: bool,
    detail_guarded: bool,
}

/// Indentation of a `<Route>` inside `RutasApp`.
const ROUTE_INDENT: usize = 6;

fn fits(line: &str) -> bool {
    line.chars().count() <= LINE_WIDTH
}

fn generated_ctx(config: &PuertoConfig) -> tera::Context {
    let mut sliced: Vec<RoutedEntityCtx> = config
        .entity
        .iter()
        .filter(|e| has_web_slice(e))
        .map(|e| {
            let entity = web_entity_ctx(&e.name);
            let access = Access::in_config(config, &e.name);
            let guarded = |action: &str| access.is_protected(&crud_use_case(action, &entity.snake));
            let route = |suffix: &str, container: &str| {
                fits(&format!(
                    "{:ROUTE_INDENT$}<Route path=\"/{}/:id{suffix}\">{{(params) => <Contenedor{container}{} id={{params.id}} />}}</Route>",
                    "", entity.plural, entity.pascal
                ))
            };
            RoutedEntityCtx {
                edit_route_inline: route("/editar", "Formulario"),
                detail_route_inline: route("", "Detalle"),
                list_guarded: guarded("list"),
                new_guarded: guarded("create"),
                // The form loads the entity, then saves it.
                edit_guarded: guarded("get") || guarded("update"),
                detail_guarded: guarded("get"),
                entity,
            }
        })
        .collect();
    sliced.sort_by(|a, b| compare_sources(&a.entity.kebab, &b.entity.kebab));
    // Biome breaks an array of two or more objects; a single one stays inline while it fits.
    let nav_inline = match sliced.as_slice() {
        [] => true,
        [only] => fits(&format!(
            "export const entradasMenu: EntradaMenu[] = [{{ etiqueta: \"{}\", href: \"/{}\" }}];",
            only.entity.label_plural, only.entity.plural
        )),
        _ => false,
    };
    // Every role a screen checks, once each: what the screen tests' caller holds.
    let mut roles: Vec<String> = vec![];
    for entity in config.entity.iter().filter(|e| has_web_slice(e)) {
        let access = Access::in_config(config, &entity.name);
        for role in access.all_roles(&entity.use_cases) {
            if !roles.contains(&role) {
                roles.push(role);
            }
        }
    }
    let mut ctx = tera::Context::new();
    ctx.insert("auth", &config.project.has_auth());
    ctx.insert("local", &(config.project.auth.as_deref() == Some("local")));
    ctx.insert(
        "guarded",
        &sliced
            .iter()
            .any(|e| e.list_guarded || e.new_guarded || e.edit_guarded || e.detail_guarded),
    );
    ctx.insert("entities", &sliced);
    ctx.insert("nav_inline", &nav_inline);
    ctx.insert("roles", &roles);
    ctx
}

/// `web/src/generado/dependencias.ts` — the composition root.
pub fn generate_web_dependencies(config: &PuertoConfig) -> Result<String, RenderError> {
    render_ts("web/dependencies.tera", &generated_ctx(config))
}

/// `web/src/generado/rutas.tsx` — the route table and the menu entries.
pub fn generate_web_routes(config: &PuertoConfig) -> Result<String, RenderError> {
    render_ts("web/routes.tera", &generated_ctx(config))
}

/// `web/src/generado/roles.ts` — auth projects only.
pub fn generate_web_roles(config: &PuertoConfig) -> Result<String, RenderError> {
    render_ts("web/roles.tera", &generated_ctx(config))
}

/// Rewrites the `generado/*` files from puerto.toml. Nothing in a project without `web`.
pub fn regenerate_web(
    base: &Path,
    config: &PuertoConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.project.web {
        return Ok(());
    }
    let generated = base.join("web/src/generado");
    write_file(
        &generated.join("dependencias.ts"),
        &generate_web_dependencies(config)?,
    )?;
    write_file(&generated.join("rutas.tsx"), &generate_web_routes(config)?)?;
    if config.project.has_auth() {
        write_file(&generated.join("roles.ts"), &generate_web_roles(config)?)?;
    }
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
            "no tiene endpoints CRUD a los que llamen las pantallas (faltan {})",
            missing.join(", ")
        ));
    }
    None
}

/// `puerto generar web <Name>` — the frontend slice alone, plus the `generado/*` files.
pub fn run_generate_web(
    name: &str,
    base: &Path,
    overwrite: OverwritePolicy,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    if !config.project.web {
        return Err(
            "este proyecto no tiene frontend ([proyecto] web no es true en puerto.toml)".into(),
        );
    }
    let Some(entity) = config.entity.iter().find(|e| e.name == pascal) else {
        return Err(format!(
            "{pascal} no está en puerto.toml. Ejecuta antes `puerto generar scaffold {pascal}`."
        )
        .into());
    };
    if let Some(reason) = web_slice_refusal(entity) {
        return Err(format!("{pascal} no tiene pantallas web: {reason}").into());
    }
    let snake = pascal_to_snake(&pascal);
    guard_files_overwrite(base, &snake, &web_entity_files(&snake), overwrite)?;

    write_web_files(&pascal, base, &entity.fields)?;
    regenerate_web(base, &config)?;

    let kebab = snake_to_kebab(&snake);
    println!("✓ web/src/contextos/{kebab}/: contenedores, vistas, mensajes");
    println!("✓ web/src/generado/: dependencias.ts, rutas.tsx");
    println!(
        "\nSiguiente: make web/sync   (exporta la especificación y regenera los tipos de la web)"
    );
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
            ..Default::default()
        }
    }

    #[test]
    fn should_not_name_a_variable_after_a_reserved_word_or_an_uncountable_plural() {
        let case = web_entity_ctx("Case");
        assert_eq!(
            (case.ident.as_str(), case.plural_ident.as_str()),
            ("caseElemento", "cases")
        );

        let analisis = web_entity_ctx("Analisis");
        assert_eq!(
            (analisis.ident.as_str(), analisis.plural_ident.as_str()),
            ("analisis", "analisisLista")
        );

        let linea = web_entity_ctx("LineaPedido");
        assert_eq!(
            (linea.ident.as_str(), linea.plural_ident.as_str()),
            ("lineaPedido", "lineasPedido")
        );
    }

    #[test]
    fn should_slice_only_crud_entities_whatever_their_fields() {
        let crud = crud_use_cases("order_item");
        assert!(has_web_slice(&entity("OrderItem", crud.clone(), vec![])));
        assert!(!has_web_slice(&entity(
            "Saludo",
            vec!["obtener_saludo".into()],
            vec![]
        )));
        let date = Field {
            name: "due".into(),
            field_type: "DateTime<Utc>".into(),
            ..Default::default()
        };
        assert!(has_web_slice(&entity("OrderItem", crud, vec![date])));
        assert!(
            web_slice_refusal(&entity("Saludo", vec!["obtener_saludo".into()], vec![]))
                .is_some_and(|r| r.contains("crear_saludo"))
        );
    }
}
