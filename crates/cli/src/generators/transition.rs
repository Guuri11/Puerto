//! `puerto generar transicion <Entity> <verb>` (27.12): a state change from end to end — the
//! entity's method, its use case, its own route and, with screens, its button. Every layer gets a
//! file of its own; a file that already existed only gains a line.

use std::path::Path;

use crate::generators::bootstrap::{regenerate_bootstrap, uses_unit_of_work};
use crate::generators::crates::add_lib_module;
use crate::generators::layout::{self, Layer};
use crate::generators::naming::{
    capitalize, entity_ctx, event_variant, is_crud_entity, pascal_to_snake, snake_to_words,
    to_pascal_case, transition_use_case, write_file,
};
use crate::generators::render::render;

pub fn run_generate_transition(
    entity: &str,
    verb: &str,
    event: Option<&str>,
    roles: &[String],
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(entity);
    let snake = pascal_to_snake(&pascal);
    let declared = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .ok_or_else(|| format!("la entidad '{pascal}' no está en puerto.toml"))?;
    if !is_crud_entity(&snake, &declared.use_cases) {
        return Err(format!(
            "{pascal} no es una entidad CRUD: una transición busca, cambia y guarda una entidad \
             del scaffold"
        )
        .into());
    }
    if !base.join(layout::rest_file(&snake, "rutas")).exists() {
        return Err(format!(
            "{pascal} aún no tiene presentación: ejecuta antes `puerto generar presentacion \
             {pascal}`"
        )
        .into());
    }
    crate::validation::validate_transition_verb(verb)?;
    let uc = transition_use_case(verb, &snake);
    if declared.transitions.iter().any(|t| t == verb)
        || declared.use_cases.contains(&uc)
        || base.join(layout::domain_file(&snake, verb)).exists()
        || base.join(layout::use_case(&snake, &uc)).exists()
    {
        return Err(format!(
            "la transición '{verb}' (el caso de uso '{uc}') ya existe en {pascal}. Elige otro \
             verbo, o borra antes sus ficheros y su entrada en puerto.toml"
        )
        .into());
    }
    let uow = uses_unit_of_work(declared, config.project.has_events());
    if let Some(event) = event {
        if !uow {
            return Err(
                "--evento necesita un proyecto con eventos de dominio (`puerto nuevo --db \
                 --eventos db`)"
                    .into(),
            );
        }
        if !declared.events.iter().any(|e| e == event) {
            crate::validation::validate_event_name(&snake, event, &declared.events)?;
        }
    }
    if !roles.is_empty() && !config.project.has_auth() {
        return Err("--rol necesita un proyecto con auth (`puerto nuevo --auth`)".into());
    }
    for role in roles {
        crate::validation::validate_role_name(role)?;
    }

    // Everything checked: the manifest first, so the wiring regenerated below sees the use case.
    let entry = config
        .entity
        .iter_mut()
        .find(|e| e.name == pascal)
        .ok_or("la entidad desapareció de puerto.toml")?;
    entry.use_cases.push(uc.clone());
    entry.transitions.push(verb.to_string());
    if let Some(event) = event.filter(|e| !entry.events.iter().any(|d| d == e)) {
        entry.events.push(event.to_string());
    }
    if !roles.is_empty() {
        entry.roles.insert(uc.clone(), roles.to_vec());
    }
    crate::puerto_toml::write(base, &config)?;

    let event_type = event.map_or_else(|| format!("{snake}_actualizado"), str::to_string);
    let mut ctx = entity_ctx(&pascal, &snake);
    ctx.insert("verb", verb);
    ctx.insert("verb_words", &snake_to_words(verb));
    ctx.insert("verb_label", &capitalize(&snake_to_words(verb)));
    ctx.insert("words", &snake_to_words(&snake));
    ctx.insert("uc", &uc);
    ctx.insert("uc_pascal", &to_pascal_case(&uc));
    ctx.insert("events", &uow);
    ctx.insert("uow", &uow);
    ctx.insert(
        "variant",
        &event.map_or_else(|| "Actualizado".to_string(), |e| event_variant(&snake, e)),
    );
    ctx.insert("event_type", &event_type);
    let auth = config.project.has_auth();
    ctx.insert("actor", &auth);
    ctx.insert("protected", &auth);
    ctx.insert("roles", roles);

    let files = [
        (
            Layer::Domain,
            verb,
            layout::domain_file(&snake, verb),
            "domain/transition.tera",
        ),
        (
            Layer::Application,
            &uc,
            layout::use_case(&snake, &uc),
            "use_case/transition.tera",
        ),
        (
            Layer::Rest,
            &uc,
            layout::rest_file(&snake, &uc),
            "presentation/transition.tera",
        ),
    ];
    for (layer, module, path, template) in &files {
        write_file(&base.join(path), &render(template, &ctx)?)?;
        add_lib_module(base, &layout::crate_lib(&snake, *layer), module)?;
    }
    regenerate_bootstrap(base)?;

    let mut missing = vec![];
    let has_screens = base
        .join(crate::generators::web::web_required_files(&snake)[1].clone())
        .exists();
    if config.project.web && has_screens {
        let config = crate::puerto_toml::read(base)?;
        missing = crate::generators::web::add_transition_to_web(base, &config, &pascal, verb)?;
    }

    println!("✓ Transición '{verb}' añadida a {pascal}:");
    for (_, _, path, _) in &files {
        println!("  {path}");
    }
    if config.project.web && has_screens {
        for path in crate::generators::web::transition_files(&snake, verb) {
            println!("  {}", path.display());
        }
    }
    println!("  puerto.toml, el cableado y bootstrap.rs actualizados.");
    for line in &missing {
        eprintln!("⚠ {line}");
    }
    println!(
        "  Escribe la regla en {} (quita el `#[ignore]` de sus tests).",
        layout::domain_file(&snake, verb)
    );
    Ok(())
}
