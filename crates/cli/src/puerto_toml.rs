use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::validation::is_pascal_case;

// ── Config types ──────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PuertoConfig {
    #[serde(rename = "proyecto")]
    pub project: Project,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "entidad")]
    pub entity: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "objeto_valor")]
    pub value_object: Vec<ValueObjectDefinition>,
    /// The worker's consumers (ADR-0018 § 5) — `events` projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "consumidor")]
    pub consumer: Vec<Consumer>,
    /// The project's mails (ADR-0020) — wiring only: each one's data, text and recipient live in
    /// its files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "correo")]
    pub mail: Vec<Mail>,
}

/// One `[[correo]]`: its port, its template mailer and its templates, in the `correo` context.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Mail {
    /// snake_case, unique: the modules, the template folder, `{Name}Mail` / `{Name}Mailer`.
    #[serde(rename = "nombre")]
    pub name: String,
    /// The event types the worker sends it on — `events` projects. Empty: nothing sends it yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "tras")]
    pub on: Vec<String>,
}

/// One `[[consumidor]]`: `presentacion/worker/src/consumidores/<name>.rs`, subscribed to `events`.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Consumer {
    /// snake_case, unique: the module, the `db` transport's consumer name and the Kafka group id.
    #[serde(rename = "nombre")]
    pub name: String,
    /// The event types it is delivered (`product_created`), each declared by some entity.
    #[serde(rename = "eventos")]
    pub events: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ValueObjectDefinition {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "tipo")]
    pub inner_type: String,
    /// The valid value its fields' tests use instead of the registry's (27.15): `ana@ejemplo.es`.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "ejemplo")]
    pub example: Option<String>,
    /// A class with a rule (27.16): `email` (a `String`) or `dinero` (an `i64` of cents).
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "clase")]
    pub class: Option<String>,
}

/// The value object classes with a rule (27.16), and the primitive each wraps.
pub const VO_CLASSES: &[(&str, &str)] = &[("email", "String"), ("dinero", "i64")];

/// Where the app goes under a portada unless `puerto nuevo --ruta-app` says otherwise.
pub const DEFAULT_APP_PATH: &str = "/panel";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Project {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
    /// The project has a `web/` frontend (`puerto nuevo` without `--sin-web`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub web: bool,
    /// Domain events (ADR-0018): the transport, `"db"` or `"kafka"`. Absent: no events. Set by
    /// `puerto nuevo --db --eventos <transport>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "eventos")]
    pub events: Option<String>,
    /// Auth (ADR-0021): `"oidc"` or `"local"`. Absent: no auth. Set by `puerto nuevo --auth <mode>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    /// A public portada at `/` and the app under `app_path` (ADR-0028). Set by `puerto nuevo
    /// --portada`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub portada: bool,
    /// Where the app goes when there is a portada: `/panel` unless `puerto nuevo --ruta-app` says.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "ruta_app")]
    pub app_path: Option<String>,
}

impl Project {
    /// Every CRUD write goes through a unit of work and records an event in the outbox.
    pub fn has_events(&self) -> bool {
        self.events.is_some()
    }

    /// Routes take `auth: BearerAuth` unless the entity lists them as `public`.
    pub fn has_auth(&self) -> bool {
        self.auth.is_some()
    }

    /// Where the frontend mounts the app: under the portada, or at the root without one.
    pub fn app_path(&self) -> &str {
        match (&self.app_path, self.portada) {
            (Some(path), true) => path,
            (None, true) => DEFAULT_APP_PATH,
            _ => "/",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Entity {
    /// PascalCase entity name, e.g. "Product"
    #[serde(rename = "nombre")]
    pub name: String,
    /// snake_case use case action names, e.g. ["create_product"]
    #[serde(rename = "casos_de_uso")]
    pub use_cases: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub db: bool,
    /// Uniques over several fields (`--unico jugador_id,partido_id`): one partial unique index
    /// each, checked by the in-memory repository too. Values, so written before any table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "unicos")]
    pub uniques: Vec<Vec<String>>,
    /// The order of its list (`--orden=-puntos,nombre`): own fields, a `-` for descending; then
    /// `creado_en DESC, id`. Empty: the newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "orden")]
    pub order: Vec<String>,
    /// Auth projects: the use cases whose routes take no `BearerAuth` — what scaffold wrote
    /// (`--publico`); after it, `rutas.rs` is the user's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "publicos")]
    pub public: Vec<String>,
    /// Auth projects: the roles a use case's route requires (`#[requiere_rol]`), by use case —
    /// what scaffold wrote (`--rol`). Declared before `fields`: written after the
    /// `[[entidad.campos]]`, `[entity.roles]` would read as if it belonged to the last field.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    #[serde(rename = "campos")]
    pub fields: Vec<Field>,
    /// Custom events beyond the CRUD ones (`product_published`), snake_case — `events` projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "eventos")]
    pub events: Vec<String>,
    /// State changes written by `generar transicion` (27.12), by verb: `cancelar` is the method
    /// `dominio/src/cancelar.rs`, the use case `cancelar_<entity>` (also in `use_cases`) and the
    /// route `rest/src/cancelar_<entity>.rs`, mounted on its own `Api<UseCase>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "transiciones")]
    pub transitions: Vec<String>,
    /// Use cases with no route (27.13, `--interno`): their params take no caller, `Api{Entity}`
    /// has no field for them and a transition among them no `Api{UseCase}`. `cableado` builds them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "internos")]
    pub internal: Vec<String>,
}

impl Entity {
    /// The use cases `Api{Entity}` holds: every one but its transitions' and its internal ones.
    pub fn api_use_cases(&self) -> Vec<String> {
        let transitions = self.transition_use_cases();
        self.use_cases
            .iter()
            .filter(|uc| !transitions.contains(uc) && !self.internal.contains(uc))
            .cloned()
            .collect()
    }

    /// The use cases of its transitions with a route, each on its own `Api{UseCase}`.
    pub fn routed_transition_use_cases(&self) -> Vec<String> {
        self.transition_use_cases()
            .into_iter()
            .filter(|uc| !self.internal.contains(uc))
            .collect()
    }

    /// The use cases of its transitions: `cancelar_producto` for `cancelar`.
    pub fn transition_use_cases(&self) -> Vec<String> {
        let snake = crate::generators::naming::pascal_to_snake(&self.name);
        self.transitions
            .iter()
            .map(|verb| crate::generators::naming::transition_use_case(verb, &snake))
            .collect()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Field {
    /// Snake_case field name, e.g. "name"
    #[serde(rename = "nombre")]
    pub name: String,
    /// Rust type string, e.g. "String", "i64", "Option<String>"
    #[serde(rename = "tipo")]
    pub field_type: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[serde(rename = "unico")]
    pub unique: bool,
    /// Auth projects: public routes answer it only to a caller with a session (`--privado`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[serde(rename = "privado")]
    pub private: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "objeto_valor")]
    pub value_object: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "clase_objeto_valor")]
    pub value_object_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "variantes")]
    pub enum_variants: Option<Vec<String>>,
    /// A reference (ADR-0022): the CRUD entity whose id this `Uuid` / `Option<Uuid>` holds. The
    /// domain holds that entity's id type; a `--db` project's column is a foreign key to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "referencia")]
    pub references: Option<String>,
    /// What the screens call it (27.5); without it, [`crate::generators::naming::etiqueta`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "etiqueta")]
    pub label: Option<String>,
    /// Its shared value object's `ejemplo` (27.15), never written: [`read`] and
    /// [`apply_shared_vo_inference`] give it to every field of that value object.
    #[serde(skip)]
    pub example: Option<String>,
    /// What the screens show for each variant of an enum (27.9: `{ AlaVenta = "A la venta" }`);
    /// a variant it leaves out, [`crate::generators::naming::texto_variante`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "textos")]
    pub texts: Option<BTreeMap<String, String>>,
}

/// Parse a CLI field argument into a `Field`.
///
/// Shell-safe syntax (no quoting required):
///
/// ```text
/// name:Type                          primitive (String, i64, bool, f64, Uuid, DateTime, map)
/// name:opt:Type                      Option<Type> primitive
/// name:vec:Type                      Vec<Type> primitive
/// name:VoName:Type                   VO wrapping a primitive
/// name:VoName:opt:Type               Option<VO> field
/// name:VoName:vec:Type               Vec<VO> field
/// name:VoName:enum:V1/V2/V3          Enum VO (variants separated by /)
/// name:ref:Entity                    Reference to another entity: its id (ADR-0022)
/// name:ref:opt:Entity                Optional reference
/// ```
///
/// Append `!` to the whole argument to mark the field as `unique = true`.
/// Type shorthands: `DateTime` → `DateTime<Utc>`, `map` → `HashMap<String, String>`.
pub fn parse_field_arg(s: &str) -> Result<Field, String> {
    let (raw, unique) = if let Some(stripped) = s.strip_suffix('!') {
        (stripped, true)
    } else {
        (s, false)
    };

    let parts: Vec<&str> = raw.split(':').collect();

    if parts.len() == 1 || parts[0].is_empty() {
        return Err(format!(
            "argumento de campo inválido '{}'. El formato es nombre:Tipo (p. ej. titulo:String)",
            s
        ));
    }

    let name = parts[0].to_string();
    crate::validation::validate_field_name(&name)?;

    let field = parse_field_shape(s, name, &parts, unique)?;
    // Fail at the command that caused it, not at `cargo build` inside the generated project.
    crate::validation::validate_field(&field)?;
    Ok(field)
}

fn parse_field_shape(s: &str, name: String, parts: &[&str], unique: bool) -> Result<Field, String> {
    match parts[1..] {
        ["ref", target] => return Ok(reference(name, target, "Uuid", unique)),
        ["ref", "opt", target] => return Ok(reference(name, target, "Option<Uuid>", unique)),
        _ => {}
    }
    match parts.len() {
        1 => unreachable!("handled above"),
        2 => {
            // name:PrimitiveType
            let ft = expand_type(parts[1]);
            if ft.is_empty() {
                return Err(format!(
                    "argumento de campo inválido '{}'. Falta el tipo",
                    s
                ));
            }
            Ok(Field {
                name,
                field_type: ft,
                unique,
                ..Default::default()
            })
        }
        3 => {
            let second = parts[1];
            let third = parts[2];
            match second {
                "opt" => Ok(Field {
                    name,
                    field_type: format!("Option<{}>", expand_type(third)),
                    unique,
                    ..Default::default()
                }),
                "vec" => Ok(Field {
                    name,
                    field_type: format!("Vec<{}>", expand_type(third)),
                    unique,
                    ..Default::default()
                }),
                vo_name if is_pascal_case(vo_name) => {
                    // A class with a rule (27.16) says its primitive: `email:Email:email`.
                    let class = VO_CLASSES.iter().find(|(class, _)| *class == third);
                    Ok(Field {
                        name,
                        field_type: class
                            .map_or_else(|| expand_type(third), |(_, ty)| ty.to_string()),
                        unique,
                        value_object: Some(vo_name.to_string()),
                        value_object_kind: class.map(|(class, _)| class.to_string()),
                        ..Default::default()
                    })
                }
                _ => Err(format!(
                    "argumento de campo inválido '{}'. '{}' no es una palabra conocida (opt, vec) ni el nombre de un objeto valor en PascalCase",
                    s, second
                )),
            }
        }
        4 => {
            let vo_name = parts[1];
            let modifier = parts[2];
            let rest = parts[3];

            if !is_pascal_case(vo_name) {
                return Err(format!(
                    "nombre de objeto valor inválido '{}'. Va en PascalCase (una mayúscula seguida de letras y dígitos)",
                    vo_name
                ));
            }

            match modifier {
                "opt" => Ok(Field {
                    name,
                    field_type: format!("Option<{}>", expand_type(rest)),
                    unique,
                    value_object: Some(vo_name.to_string()),
                    ..Default::default()
                }),
                "vec" => Ok(Field {
                    name,
                    field_type: format!("Vec<{}>", expand_type(rest)),
                    unique,
                    value_object: Some(vo_name.to_string()),
                    ..Default::default()
                }),
                "enum" => {
                    if rest.is_empty() {
                        return Err(format!(
                            "argumento de campo inválido '{}'. Faltan las variantes del enum",
                            s
                        ));
                    }
                    let variants: Vec<String> =
                        rest.split('/').map(|v| v.trim().to_string()).collect();
                    for variant in &variants {
                        if !is_pascal_case(variant) {
                            return Err(format!(
                                "variante de enum inválida '{}'. Va en PascalCase (una mayúscula seguida de letras y dígitos)",
                                variant
                            ));
                        }
                    }
                    Ok(Field {
                        name,
                        field_type: "String".to_string(),
                        unique,
                        value_object: Some(vo_name.to_string()),
                        value_object_kind: Some("enum".to_string()),
                        enum_variants: Some(variants),
                        ..Default::default()
                    })
                }
                _ => Err(format!(
                    "argumento de campo inválido '{}'. El modificador '{}' no existe (valen: opt, vec, enum)",
                    s, modifier
                )),
            }
        }
        _ => Err(format!(
            "argumento de campo inválido '{}'. Demasiadas partes separadas por dos puntos (van de 2 a 4)",
            s
        )),
    }
}

fn reference(name: String, target: &str, field_type: &str, unique: bool) -> Field {
    Field {
        name,
        field_type: field_type.to_string(),
        unique,
        references: Some(target.to_string()),
        ..Default::default()
    }
}

/// Expands CLI type shorthands to full Rust type strings.
/// `DateTime` → `DateTime<Utc>`, `map` → `HashMap<String, String>`.
pub fn expand_type(t: &str) -> String {
    match t {
        "DateTime" => "DateTime<Utc>".to_string(),
        "map" => "HashMap<String, String>".to_string(),
        other => other.to_string(),
    }
}

// ── I/O helpers ───────────────────────────────────────────────────────────────

pub fn read(base: &Path) -> Result<PuertoConfig, Box<dyn std::error::Error>> {
    let path = base.join("puerto.toml");
    let src = std::fs::read_to_string(&path)?;
    let mut config: PuertoConfig = toml::from_str(&src)?;
    for entity in &mut config.entity {
        give_examples(&mut entity.fields, &config.value_object);
    }
    Ok(config)
}

/// Each field of a shared value object takes its `ejemplo` (27.15) and, when the field names
/// none, its `clase` (27.16).
fn give_examples(fields: &mut [Field], shared_vos: &[ValueObjectDefinition]) {
    for field in fields {
        let shared = field
            .value_object
            .as_ref()
            .and_then(|vo| shared_vos.iter().find(|s| &s.name == vo));
        field.example = shared.and_then(|s| s.example.clone());
        if field.value_object_kind.is_none() {
            field.value_object_kind = shared.and_then(|s| s.class.clone());
        }
    }
}

pub fn write(base: &Path, config: &PuertoConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("puerto.toml");
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

/// Insert the entity, or update it in place if already present.
///
/// Upsert rather than no-op: re-scaffolding an entity with a different field list used to leave
/// puerto.toml describing the *old* fields while the code on disk had the new ones — a divergence
/// neither `puerto validar` nor `puerto generar bootstrap` could see, in the one file documented
/// as the source of truth.
///
/// Use cases are **merged**, not replaced: actions added by `puerto generar caso-de-uso` survive a
/// re-scaffold. Fields are replaced — they are what the caller just regenerated the code from.
pub fn add_entity(
    base: &Path,
    name: &str,
    use_cases: Vec<String>,
    db: bool,
    fields: Vec<Field>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    match config.entity.iter_mut().find(|e| e.name == name) {
        Some(existing) => {
            for uc in use_cases {
                if !existing.use_cases.contains(&uc) {
                    existing.use_cases.push(uc);
                }
            }
            existing.db = db;
            existing.fields = fields;
        }
        None => config.entity.push(Entity {
            name: name.to_string(),
            use_cases,
            db,
            fields,
            ..Default::default()
        }),
    }
    write(base, &config)
}

/// For fields written as `name:VoName` (no explicit inner type), infer the inner type
/// and VO name from the shared VO registry. Only applies when `value_object` is unset
/// and `field_type` matches a declared shared VO name.
pub fn apply_shared_vo_inference(
    fields: Vec<Field>,
    shared_vos: &[ValueObjectDefinition],
) -> Vec<Field> {
    if shared_vos.is_empty() {
        return fields;
    }
    let mut fields: Vec<Field> = fields
        .into_iter()
        .map(|f| {
            if f.value_object.is_none() {
                if let Some(svo) = shared_vos.iter().find(|v| v.name == f.field_type) {
                    return Field {
                        field_type: svo.inner_type.clone(),
                        value_object: Some(svo.name.clone()),
                        ..f
                    };
                }
            }
            f
        })
        .collect();
    give_examples(&mut fields, shared_vos);
    fields
}

/// Append a shared VO declaration to puerto.toml. No-op if already present.
pub fn add_value_object(
    base: &Path,
    name: &str,
    inner_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    if config.value_object.iter().any(|vo| vo.name == name) {
        return Ok(());
    }
    config.value_object.push(ValueObjectDefinition {
        name: name.to_string(),
        inner_type: inner_type.to_string(),
        ..Default::default()
    });
    write(base, &config)
}

/// Whether the project at `base` has domain events. `false` when puerto.toml cannot be read.
pub fn events_enabled(base: &Path) -> bool {
    read(base).is_ok_and(|c| c.project.has_events())
}

/// Append a custom event to an existing entity. Errors if the entity is not declared.
pub fn add_event(
    base: &Path,
    entity_name: &str,
    event: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    let entity = config
        .entity
        .iter_mut()
        .find(|e| e.name == entity_name)
        .ok_or_else(|| format!("la entidad '{entity_name}' no está en puerto.toml"))?;
    if !entity.events.iter().any(|e| e == event) {
        entity.events.push(event.to_string());
        write(base, &config)?;
    }
    Ok(())
}

/// Append a consumer. Errors if one with the same name is already declared.
pub fn add_consumer(base: &Path, consumer: Consumer) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    if config.consumer.iter().any(|c| c.name == consumer.name) {
        return Err(format!("el consumidor '{}' ya está declarado", consumer.name).into());
    }
    config.consumer.push(consumer);
    write(base, &config)
}

pub fn add_mail(base: &Path, mail: Mail) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    if config.mail.iter().any(|m| m.name == mail.name) {
        return Err(format!("el correo '{}' ya está declarado", mail.name).into());
    }
    config.mail.push(mail);
    write(base, &config)
}

/// Marks a use case of an existing entity as internal (27.13). No-op if it already is.
pub fn add_internal(
    base: &Path,
    entity_name: &str,
    use_case: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    let entity = config
        .entity
        .iter_mut()
        .find(|e| e.name == entity_name)
        .ok_or_else(|| format!("la entidad '{entity_name}' no está en puerto.toml"))?;
    if !entity.internal.iter().any(|uc| uc == use_case) {
        entity.internal.push(use_case.to_string());
        write(base, &config)?;
    }
    Ok(())
}

/// Append a use case action to an existing entity. Errors if entity not found. No-op if action already present.
pub fn add_use_case(
    base: &Path,
    entity_name: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = read(base)?;
    let entity = config
        .entity
        .iter_mut()
        .find(|e| e.name == entity_name)
        .ok_or_else(|| format!("la entidad '{entity_name}' no está en puerto.toml"))?;
    if !entity.use_cases.contains(&action.to_string()) {
        entity.use_cases.push(action.to_string());
        write(base, &config)?;
    }
    Ok(())
}
