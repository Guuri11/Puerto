use std::path::Path;

use crate::generators::types::{
    is_enum_vo, is_option_vo, is_value_object, is_vec_vo, resolve_type,
};
use crate::puerto_toml;

use crate::validation::SHARED_VO_ALLOWED_INNER_TYPES;

const VO_ALLOWED_INNER_TYPES: &[&str] = &[
    "String",
    "i64",
    "bool",
    "f64",
    "Uuid",
    "DateTime<Utc>",
    "Option<String>",
    "Option<i64>",
    "Option<bool>",
    "Option<f64>",
    "Option<DateTime<Utc>>",
    "Vec<String>",
    "Vec<i64>",
];

fn vo_inner_type_is_valid(field_type: &str) -> bool {
    if field_type.starts_with("Option<") {
        let inner = &field_type[7..field_type.len() - 1];
        return !inner.starts_with("Vec<")
            && !inner.starts_with("Option<")
            && !inner.starts_with("HashMap<");
    }
    if field_type.starts_with("Vec<") {
        let inner = &field_type[4..field_type.len() - 1];
        return !inner.starts_with("Option<")
            && !inner.starts_with("Vec<")
            && !inner.starts_with("HashMap<");
    }
    !field_type.starts_with("HashMap<")
}

pub fn run_validate(cwd: &Path) -> Result<(), Box<dyn std::error::Error>> {
    crate::commands::list::require_puerto_project(cwd)?;
    let config = puerto_toml::read(cwd)?;

    let mut errors: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    if config.project.name.is_empty() {
        errors.push("proyecto.nombre está vacío".into());
    }
    if let Some(transport) = &config.project.events {
        if let Err(e) = crate::validation::validate_events_transport(transport) {
            errors.push(format!("proyecto.eventos: {e}"));
        }
        // The outbox is written in the same transaction as the entity (ADR-0018 § 1).
        if !config.project.db {
            errors.push("proyecto.eventos necesita proyecto.db = true: los eventos se escriben en una tabla outbox, en la transacción de la entidad".into());
        }
    }

    if let Some(mode) = &config.project.auth {
        if let Err(e) = crate::validation::validate_auth_mode(mode) {
            errors.push(format!("proyecto.auth: {e}"));
        }
        // Local sessions and credentials are rows (ADR-0021 § 2).
        if mode == "local" && !config.project.db {
            errors.push("proyecto.auth = \"local\" necesita proyecto.db = true: las sesiones y las credenciales son filas de la base de datos".into());
        }
    }

    if config.project.portada {
        if !config.project.web {
            errors.push(
                "proyecto.portada necesita proyecto.web = true: la portada es una página de web/"
                    .into(),
            );
        } else if !cwd.join("web/src/paginas/portada.tsx").exists() {
            errors.push("proyecto.portada: falta web/src/paginas/portada.tsx (puerto generar web-bloque portada/bienvenida)".into());
        }
    }
    if let Some(path) = &config.project.app_path {
        if !config.project.portada {
            errors.push("proyecto.ruta_app solo vale con proyecto.portada = true: sin portada, la app va en /".into());
        }
        if let Err(e) = crate::validation::validate_app_path(path) {
            errors.push(format!("proyecto.ruta_app: {e}"));
        }
    }

    let mut seen_entities: Vec<String> = vec![];
    for entity in &config.entity {
        if let Err(e) = crate::validation::validate_entity_name(&entity.name) {
            errors.push(format!("entidad '{}': {}", entity.name, e));
        }

        if config.project.has_auth() {
            if let Err(e) = crate::validation::validate_auth_entity_name(&entity.name) {
                errors.push(format!("entidad '{}': {}", entity.name, e));
            }
        } else if !entity.public.is_empty() || !entity.roles.is_empty() {
            errors.push(format!(
                "entidad '{}': declara publicos o roles, pero proyecto.auth no está puesto",
                entity.name
            ));
        }
        for e in crate::validation::validate_entity_access(entity) {
            errors.push(format!("entidad '{}': {e}", entity.name));
        }
        for e in crate::validation::validate_entity_shape(entity) {
            errors.push(format!("entidad '{}': {e}", entity.name));
        }
        if !config.project.has_auth() {
            for f in entity.fields.iter().filter(|f| f.private) {
                errors.push(format!(
                    "entidad '{}': el campo '{}' es privado, pero proyecto.auth no está puesto: sin \
                     sesión, nadie lo vería nunca",
                    entity.name, f.name
                ));
            }
        }

        if seen_entities.contains(&entity.name) {
            errors.push(format!("la entidad '{}' está repetida", entity.name));
        }
        seen_entities.push(entity.name.clone());

        if entity.use_cases.is_empty() {
            warnings.push(format!("entidad '{}': no tiene casos de uso", entity.name));
        }

        for uc in &entity.use_cases {
            if crate::validation::validate_use_case_name(uc).is_err() {
                errors.push(format!(
                    "entidad '{}': el caso de uso '{}' no es snake_case válido",
                    entity.name, uc
                ));
            }
        }

        let snake = crate::generators::naming::pascal_to_snake(&entity.name);
        if !entity.events.is_empty() && !config.project.has_events() {
            errors.push(format!(
                "entidad '{}': declara eventos, pero proyecto.eventos no está puesto",
                entity.name
            ));
        }
        for (i, event) in entity.events.iter().enumerate() {
            if let Err(e) =
                crate::validation::validate_event_name(&snake, event, &entity.events[..i])
            {
                errors.push(format!("entidad '{}': {e}", entity.name));
            }
        }
        // Transitions (27.12): a verb each, whose use case `bootstrap.rs` mounts on its own API.
        for (i, (verb, uc)) in entity
            .transitions
            .iter()
            .zip(entity.transition_use_cases())
            .enumerate()
        {
            if let Err(e) = crate::validation::validate_transition_verb(verb) {
                errors.push(format!("entidad '{}': transiciones: {e}", entity.name));
            }
            if entity.transitions[..i].contains(verb) {
                errors.push(format!(
                    "entidad '{}': transiciones: '{verb}' está dos veces",
                    entity.name
                ));
            }
            if !entity.use_cases.contains(&uc) {
                errors.push(format!(
                    "entidad '{}': transiciones: '{verb}' necesita su caso de uso '{uc}' en \
                     casos_de_uso",
                    entity.name
                ));
            }
        }

        let mut seen_fields: Vec<String> = vec![];
        for field in &entity.fields {
            if field.name.is_empty() {
                errors.push(format!(
                    "entidad '{}': un campo tiene el nombre vacío",
                    entity.name
                ));
                continue;
            }

            if let Err(e) = crate::validation::validate_field_name(&field.name) {
                errors.push(format!("entidad '{}': {}", entity.name, e));
            }
            if config.project.db {
                if let Err(e) = crate::validation::validate_sql_field_name(&field.name) {
                    errors.push(format!("entidad '{}': {}", entity.name, e));
                }
            }

            if seen_fields.contains(&field.name) {
                errors.push(format!(
                    "entidad '{}': el campo '{}' está repetido",
                    entity.name, field.name
                ));
            }
            seen_fields.push(field.name.clone());

            if let Err(e) = resolve_type(&field.field_type) {
                errors.push(format!(
                    "entidad '{}': el campo '{}' tiene {}",
                    entity.name, field.name, e
                ));
            }

            if field.field_type.starts_with("Option<") && field.unique {
                warnings.push(format!(
                    "entidad '{}': el campo '{}' es Option<> y único: un único que admite nulos quizá no sea lo que buscas",
                    entity.name, field.name
                ));
            }

            if is_value_object(field) {
                if let Err(e) =
                    crate::validation::validate_vo_name(field.value_object.as_deref().unwrap())
                {
                    errors.push(format!(
                        "entidad '{}': campo '{}': {}",
                        entity.name, field.name, e
                    ));
                }
                if is_enum_vo(field) {
                    if field.field_type != "String" {
                        errors.push(format!(
                            "entidad '{}': el campo '{}' tiene un objeto_valor enum, pero su tipo '{}' no es String",
                            entity.name, field.name, field.field_type
                        ));
                    }
                    if field.enum_variants.is_none() {
                        errors.push(format!(
                            "entidad '{}': el campo '{}' tiene clase_objeto_valor enum, pero no tiene variantes",
                            entity.name, field.name
                        ));
                    }
                    if let Some(variants) = &field.enum_variants {
                        if let Err(e) = crate::validation::validate_enum_variants(variants) {
                            errors.push(format!(
                                "entidad '{}': campo '{}': {}",
                                entity.name, field.name, e
                            ));
                        }
                    }
                } else {
                    if !VO_ALLOWED_INNER_TYPES.contains(&field.field_type.as_str()) {
                        errors.push(format!(
                            "entidad '{}': el campo '{}' tiene objeto_valor, pero el tipo interno '{}' no vale (valen: {})",
                            entity.name, field.name, field.field_type, VO_ALLOWED_INNER_TYPES.join(", ")
                        ));
                    }
                    if !vo_inner_type_is_valid(&field.field_type) {
                        errors.push(format!(
                            "entidad '{}': el campo '{}' tiene objeto_valor con un tipo interno compuesto inválido '{}'",
                            entity.name, field.name, field.field_type
                        ));
                    }
                    if field.enum_variants.is_some() {
                        errors.push(format!(
                            "entidad '{}': el campo '{}' tiene variantes, pero clase_objeto_valor no es 'enum'",
                            entity.name, field.name
                        ));
                    }
                }
                if field.unique && (is_option_vo(field) || is_vec_vo(field)) {
                    errors.push(format!(
                        "entidad '{}': el campo '{}' es un objeto valor Option/Vec y no puede ser único",
                        entity.name, field.name
                    ));
                }
            }
        }

        for field in entity.fields.iter().filter(|f| f.references.is_some()) {
            if let Err(e) = crate::validation::validate_field(field) {
                errors.push(format!("entidad '{}': {e}", entity.name));
            }
        }
        for e in crate::validation::validate_references(&entity.name, &entity.fields, &config) {
            errors.push(format!("entidad '{}': {e}", entity.name));
        }

        // One `value_objects.rs` is written per entity — two fields cannot define the same VO
        // with different inner types.
        for e in crate::validation::validate_vo_coherence(&entity.fields, &config.value_object) {
            errors.push(format!("entidad '{}': {}", entity.name, e));
        }
    }

    let mut seen_shared_vo_names: Vec<String> = vec![];
    for vo_def in &config.value_object {
        if let Err(e) = crate::validation::validate_vo_name(&vo_def.name) {
            errors.push(format!("objeto_valor compartido '{}': {}", vo_def.name, e));
        }
        if let Some(example) = &vo_def.example {
            if let Err(e) = crate::validation::validate_vo_example(&vo_def.inner_type, example) {
                errors.push(format!("objeto_valor compartido '{}': {e}", vo_def.name));
            }
        }
        if seen_shared_vo_names.contains(&vo_def.name) {
            errors.push(format!(
                "el objeto_valor compartido '{}' está repetido",
                vo_def.name
            ));
        }
        seen_shared_vo_names.push(vo_def.name.clone());
        if !SHARED_VO_ALLOWED_INNER_TYPES.contains(&vo_def.inner_type.as_str()) {
            errors.push(format!(
                "objeto_valor compartido '{}': el tipo interno '{}' no vale (valen: {})",
                vo_def.name,
                vo_def.inner_type,
                SHARED_VO_ALLOWED_INNER_TYPES.join(", ")
            ));
        }
    }

    // Consumers (ADR-0018 § 5): the worker's registry names each one and the events it handles.
    let declared = crate::generators::worker::declared_events(&config);
    let mut seen_consumers: Vec<&str> = vec![];
    for consumer in &config.consumer {
        if !config.project.has_events() {
            errors.push(format!(
                "consumidor '{}': declarado, pero proyecto.eventos no está puesto",
                consumer.name
            ));
        }
        if let Err(e) = crate::validation::validate_consumer_name(&consumer.name) {
            errors.push(format!("consumidor '{}': {e}", consumer.name));
        }
        if seen_consumers.contains(&consumer.name.as_str()) {
            errors.push(format!("el consumidor '{}' está repetido", consumer.name));
        }
        seen_consumers.push(&consumer.name);
        if consumer.events.is_empty() {
            errors.push(format!(
                "consumidor '{}': no se suscribe a ningún evento",
                consumer.name
            ));
        }
        for event in &consumer.events {
            if !declared.iter().any(|d| d.event_type == *event) {
                errors.push(format!(
                    "consumidor '{}': ninguna entidad declara el evento '{event}'",
                    consumer.name
                ));
            }
        }
    }
    errors.extend(crate::generators::consistency::missing_consumer_files(
        cwd, &config,
    ));

    // Mails (ADR-0020): two modules and a template folder each; `on` is worker wiring.
    let mut seen_mails: Vec<&str> = vec![];
    for mail in &config.mail {
        if let Err(e) = crate::validation::validate_mail_name(&mail.name) {
            errors.push(format!("correo '{}': {e}", mail.name));
        }
        if seen_mails.contains(&mail.name.as_str()) {
            errors.push(format!("el correo '{}' está repetido", mail.name));
        }
        seen_mails.push(&mail.name);
        if mail.on.is_empty() {
            continue;
        }
        if !config.project.has_events() {
            errors.push(format!(
                "correo '{}': se envía tras eventos, pero proyecto.eventos no está puesto",
                mail.name
            ));
        }
        for event in &mail.on {
            if !declared.iter().any(|d| d.event_type == *event) {
                errors.push(format!(
                    "correo '{}': ninguna entidad declara el evento '{event}'",
                    mail.name
                ));
            }
        }
        if config
            .consumer
            .iter()
            .any(|c| c.name == format!("correo_{}", mail.name))
        {
            errors.push(format!(
                "correo '{0}': el worker lo ejecuta como el consumidor 'correo_{0}', que ya usa un \
                 [[consumidor]]",
                mail.name
            ));
        }
    }
    errors.extend(crate::generators::consistency::missing_mail_files(
        cwd, &config,
    ));

    // The manifest can be internally consistent and still describe code that is not there —
    // `generate bootstrap` would then emit imports for modules that do not exist.
    for line in crate::generators::consistency::drift_lines(
        &crate::generators::consistency::missing_entity_files(cwd, &config),
    ) {
        errors.push(line);
    }
    errors.extend(crate::generators::consistency::auth_drift(cwd, &config));
    for orphan in crate::generators::consistency::orphan_entity_dirs(cwd, &config) {
        warnings.push(format!(
            "'contextos/{orphan}' existe, pero no tiene bloque [[entidad]]: los comandos de Puerto \
             no lo ven"
        ));
    }

    // Generated code expects the runtime of the CLI that generates it; another minor may not have
    // what it imports. A warning: the project still builds against the runtime it pins.
    if let Ok(manifest) = std::fs::read_to_string(cwd.join("Cargo.toml")) {
        warnings.extend(crate::runtime::version_warnings(
            &manifest,
            env!("CARGO_PKG_VERSION"),
        ));
    }

    if !errors.is_empty() {
        eprintln!("La validación falló:");
        for e in &errors {
            eprintln!("  ✗ {e}");
        }
        if !warnings.is_empty() {
            for w in &warnings {
                eprintln!("  ⚠ {w}");
            }
        }
        let msg = errors.join("\n  ");
        return Err(format!("{}\n  {} error(es) encontrado(s)", msg, errors.len()).into());
    }

    if !warnings.is_empty() {
        eprintln!("Avisos:");
        for w in &warnings {
            eprintln!("  ⚠ {w}");
        }
    }

    println!(
        "✓ puerto.toml es válido: {} entidad{} validada{}",
        config.entity.len(),
        if config.entity.len() == 1 { "" } else { "es" },
        if config.entity.len() == 1 { "" } else { "s" }
    );
    Ok(())
}
