use crate::puerto_toml::{Field, ValueObjectDefinition};

#[derive(Debug)]
pub struct TypeMapping {
    pub rust_type: &'static str,
    pub sql_type: &'static str,
    pub sql_nullable: bool,
    // The OpenAPI schema is derived by poem from the Rust type; these two record what it derives,
    // so the registry stays the one table describing a type across Rust, SQL and the API docs.
    #[allow(dead_code)]
    pub openapi_type: &'static str,
    #[allow(dead_code)]
    pub openapi_format: Option<&'static str>,
    pub default_expr: &'static str,
    pub needs_import: Option<&'static str>,
}

const TYPE_REGISTRY: &[TypeMapping] = &[
    TypeMapping {
        rust_type: "String",
        sql_type: "TEXT",
        sql_nullable: false,
        openapi_type: "string",
        openapi_format: None,
        default_expr: r#""example".to_string()"#,
        needs_import: None,
    },
    TypeMapping {
        rust_type: "i64",
        sql_type: "BIGINT",
        sql_nullable: false,
        openapi_type: "integer",
        openapi_format: Some("int64"),
        default_expr: "42",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "bool",
        sql_type: "BOOLEAN",
        sql_nullable: false,
        openapi_type: "boolean",
        openapi_format: None,
        default_expr: "true",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "f64",
        sql_type: "DOUBLE",
        sql_nullable: false,
        openapi_type: "number",
        openapi_format: Some("double"),
        default_expr: "1.5",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Option<String>",
        sql_type: "TEXT",
        sql_nullable: true,
        openapi_type: "string",
        openapi_format: None,
        default_expr: "None",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Option<i64>",
        sql_type: "BIGINT",
        sql_nullable: true,
        openapi_type: "integer",
        openapi_format: Some("int64"),
        default_expr: "None",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Option<bool>",
        sql_type: "BOOLEAN",
        sql_nullable: true,
        openapi_type: "boolean",
        openapi_format: None,
        default_expr: "None",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Option<f64>",
        sql_type: "DOUBLE",
        sql_nullable: true,
        openapi_type: "number",
        openapi_format: Some("double"),
        default_expr: "None",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Uuid",
        sql_type: "UUID",
        sql_nullable: false,
        openapi_type: "string",
        openapi_format: Some("uuid"),
        default_expr: "Uuid::new_v4()",
        needs_import: Some("uuid::Uuid"),
    },
    TypeMapping {
        rust_type: "Option<Uuid>",
        sql_type: "UUID",
        sql_nullable: true,
        openapi_type: "string",
        openapi_format: Some("uuid"),
        default_expr: "None",
        needs_import: Some("uuid::Uuid"),
    },
    TypeMapping {
        rust_type: "DateTime<Utc>",
        sql_type: "TIMESTAMPTZ",
        sql_nullable: false,
        openapi_type: "string",
        openapi_format: Some("date-time"),
        default_expr: "Utc::now()",
        needs_import: Some("chrono::{DateTime, Utc}"),
    },
    TypeMapping {
        rust_type: "Option<DateTime<Utc>>",
        sql_type: "TIMESTAMPTZ",
        sql_nullable: true,
        openapi_type: "string",
        openapi_format: Some("date-time"),
        default_expr: "None",
        needs_import: Some("chrono::{DateTime, Utc}"),
    },
    TypeMapping {
        rust_type: "Vec<String>",
        sql_type: "TEXT[]",
        sql_nullable: false,
        openapi_type: "array",
        openapi_format: None,
        default_expr: "vec![]",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "Vec<i64>",
        sql_type: "BIGINT[]",
        sql_nullable: false,
        openapi_type: "array",
        openapi_format: None,
        default_expr: "vec![]",
        needs_import: None,
    },
    TypeMapping {
        rust_type: "HashMap<String, String>",
        sql_type: "JSONB",
        sql_nullable: false,
        openapi_type: "object",
        openapi_format: None,
        default_expr: "HashMap::new()",
        needs_import: Some("std::collections::HashMap"),
    },
];

pub fn resolve_type(field_type: &str) -> Result<&'static TypeMapping, String> {
    TYPE_REGISTRY
        .iter()
        .find(|m| m.rust_type == field_type)
        .ok_or_else(|| {
            let valid: Vec<&str> = TYPE_REGISTRY.iter().map(|m| m.rust_type).collect();
            format!(
                "tipo de campo no admitido '{}'. Los admitidos: {}",
                field_type,
                valid.join(", ")
            )
        })
}

pub fn validate_fields(fields: &[Field]) -> Result<(), String> {
    for field in fields {
        resolve_type(&field.field_type)?;
    }
    Ok(())
}

pub fn is_value_object(field: &Field) -> bool {
    field.value_object.is_some()
}

pub fn is_option_vo(field: &Field) -> bool {
    field.value_object.is_some() && field.field_type.starts_with("Option<")
}

pub fn is_vec_vo(field: &Field) -> bool {
    field.value_object.is_some() && field.field_type.starts_with("Vec<")
}

pub fn is_enum_vo(field: &Field) -> bool {
    field.value_object_kind.as_deref() == Some("enum")
}

/// A value object's class with a rule (27.16): `email` or `dinero`; `None` for any other.
pub fn vo_class(field: &Field) -> Option<&'static str> {
    let kind = field.value_object_kind.as_deref()?;
    crate::puerto_toml::VO_CLASSES
        .iter()
        .find(|(class, _)| *class == kind)
        .map(|(class, _)| *class)
}

pub fn is_shared_vo(field: &Field, shared_vos: &[ValueObjectDefinition]) -> bool {
    field
        .value_object
        .as_deref()
        .is_some_and(|vo| shared_vos.iter().any(|d| d.name == vo))
}

/// A reference to another entity (ADR-0022): its id is what the field holds.
pub fn is_reference(field: &Field) -> bool {
    field.references.is_some()
}

/// The id type a reference holds — `CustomerId` for `references = "Customer"`.
pub fn reference_id(field: &Field) -> Option<String> {
    field
        .references
        .as_deref()
        .map(|target| format!("Id{target}"))
}

/// The path of a reference's id type, which lives in its owner's domain crate (ADR-0023 § 3):
/// `dominio_customer::modelo::CustomerId`.
pub fn reference_import(field: &Field) -> Option<String> {
    let target = field.references.as_deref()?;
    let snake = crate::generators::naming::pascal_to_snake(target);
    Some(format!("dominio_{snake}::modelo::Id{target}"))
}

/// What naming the field's **domain** type needs from the registry: nothing for a reference,
/// whose type is the target's id (see [`reference_import`]).
pub fn type_import(field: &Field) -> Option<&'static str> {
    if is_reference(field) {
        return None;
    }
    resolve_type(&field.field_type).ok()?.needs_import
}

/// The import a **value literal** of this field needs — as opposed to `needs_import`, which is what
/// naming the field's *type* needs.
///
/// Test Props/Params literals and Object Mother defaults write `Utc::now()`, `Uuid::new_v4()` or
/// `HashMap::new()`, never the type itself, so a `DateTime<Utc>` field needs `chrono::Utc` only —
/// importing `chrono::{DateTime, Utc}` there was an `unused_imports` warning (an error under the
/// generated project's `make lint`). `Option<…>` / `Vec<…>` literals are `None` / `vec![]`, and
/// enum VOs a variant path: none needs a primitive import.
pub fn literal_import(field: &Field) -> Option<&'static str> {
    // A reference's literal is its id type's (`CustomerId::generate()`): see [`reference_import`].
    if is_enum_vo(field) || is_reference(field) {
        return None;
    }
    match field.field_type.as_str() {
        "DateTime<Utc>" => Some("chrono::Utc"),
        "Uuid" => Some("uuid::Uuid"),
        "HashMap<String, String>" => Some("std::collections::HashMap"),
        _ => None,
    }
}

/// Whether a literal of this field names its value object (`Name::new(…)`, `Status::Active`).
/// `Option<VO>` / `Vec<VO>` literals are `None` / `vec![]` and do not.
pub fn literal_names_vo(field: &Field) -> bool {
    is_value_object(field) && !is_option_vo(field) && !is_vec_vo(field)
}

/// A value object's path: a shared one is `nucleo`'s; a local one, its domain crate's — `crate`
/// from inside it (`in_domain`).
pub fn vo_import_path(
    field: &Field,
    snake: &str,
    shared_vos: &[ValueObjectDefinition],
    in_domain: bool,
) -> String {
    let vo = field.value_object.as_deref().unwrap_or_default();
    if is_shared_vo(field, shared_vos) {
        format!("nucleo::valores::{vo}")
    } else if in_domain {
        format!("crate::valores::{vo}")
    } else {
        format!("dominio_{snake}::valores::{vo}")
    }
}

pub fn vo_inner_type(field: &Field) -> String {
    if field.field_type.starts_with("Option<") {
        field.field_type[7..field.field_type.len() - 1].to_string()
    } else if field.field_type.starts_with("Vec<") {
        field.field_type[4..field.field_type.len() - 1].to_string()
    } else {
        field.field_type.clone()
    }
}

pub fn field_rust_type(field: &Field) -> String {
    if let Some(id) = reference_id(field) {
        return if field.field_type.starts_with("Option<") {
            format!("Option<{id}>")
        } else {
            id
        };
    }
    match &field.value_object {
        Some(vo) => {
            if field.field_type.starts_with("Option<") {
                format!("Option<{}>", vo)
            } else if field.field_type.starts_with("Vec<") {
                format!("Vec<{}>", vo)
            } else {
                vo.clone()
            }
        }
        None => field.field_type.clone(),
    }
}

/// What a template needs to know about one field. The `.tera` files iterate over these and own the
/// shape of the generated code; the decisions (which VO shape, which literal) are made here once.
#[derive(Debug, serde::Serialize)]
pub struct FieldCtx {
    pub name: String,
    /// The name on the wire (`customerId`): DTOs say it with `rename_all`, a query parameter by name.
    pub camel: String,
    /// `unique = true`: a partial unique index in `--db` projects.
    pub unique: bool,
    /// `privado = true` (auth projects): `None` in the DTO a public route gives a caller without a
    /// session.
    pub private: bool,
    /// The type in domain structs: the VO name, `Option<Vo>`, `Vec<Vo>`, or the primitive.
    pub rust_type: String,
    pub is_vo: bool,
    pub is_option_vo: bool,
    pub is_vec_vo: bool,
    pub is_enum_vo: bool,
    pub is_shared_vo: bool,
    /// The VO name (`""` for a primitive field) and its snake_case form.
    pub vo: String,
    pub vo_snake: String,
    /// A primitive `String`: `new()` rejects it empty.
    pub required_string: bool,
    /// A plain `String` VO (not enum, `Option` or `Vec`): `Vo::new("")` must fail.
    pub string_vo: bool,
    /// A valid default value, see [`props_literal`].
    pub props_literal: String,
    /// The value the Pg integration tests seed, see [`seed_literal`].
    pub seed_literal: String,
    /// The primitive (registry) type, as persistence rows and params hold it.
    pub field_type: String,
    /// A `HashMap<String, String>` field: a JSONB column wrapped in `sqlx::types::Json`.
    pub is_map: bool,
    /// A `Vec<…>` column (primitive or VO): sqlx binds it as a slice.
    pub is_array: bool,
    /// The primitive read out of a borrowed entity, see [`field_value_accessor`].
    pub accessor: String,
    /// Reading the primitive out of a borrowed request body needs `.clone()`.
    pub needs_clone: bool,
    /// The domain value built from a persistence row (`row.{name}`), see [`field_vo_conversion`].
    pub from_row: String,
    /// The domain value built from a use case param (`params.{name}`).
    pub from_params: String,
    /// The Params value read out of the mother's props (`props.{name}`) in the use case tests: the
    /// primitive, but a reference's id type.
    pub from_props: String,
    /// The registry default for the primitive type — what request bodies in route tests hold.
    pub default_literal: String,
    /// What Params literals in tests hold: [`Self::default_literal`], but a reference's id type.
    pub params_literal: String,
    /// The type in use case Params: the primitive, but a reference's id type.
    pub params_type: String,
    /// The Params value built from a borrowed request body (`body.{name}`) in the routes.
    pub from_body: String,
    /// A reference (ADR-0022): the target entity (`""` otherwise) and its snake_case form.
    pub is_ref: bool,
    pub ref_target: String,
    pub ref_target_snake: String,
    /// An enum VO's first variant (`""` otherwise): the valid value tests pass for it.
    pub first_variant: String,
    /// Object Mother: what `{Entity}Mother` stores in its `Option<…>` slot, the parameter type of
    /// `with_{field}` (`&str` for strings, converted with `.to_string()`), and the value `build()`
    /// puts in the Props — the slot, or a valid default.
    pub mother_storage: String,
    pub mother_param: String,
    pub mother_build: String,
    /// What `crear` asks for (27.17): neither a `por_defecto` nor a `generado` field.
    pub in_create: bool,
    /// What `actualizar` asks for: not a `generado` field, which keeps its value.
    pub in_update: bool,
    pub generated: bool,
    /// The domain value `crear` gives a field it does not ask for: its default
    /// (`EstadoEntrada::Pendiente`) or `<Entity>::generar_<field>()`, in its value object.
    pub server_value: String,
}

/// [`FieldCtx::server_value`] (27.17); `""` for a field `crear` asks for.
fn server_value(field: &Field, pascal: &str, shared_vos: &[ValueObjectDefinition]) -> String {
    let primitive = if field.generated {
        format!("{pascal}::generar_{}()", field.name)
    } else if let Some(value) = &field.default {
        if is_enum_vo(field) {
            return format!(
                "{}::{value}",
                field.value_object.as_deref().unwrap_or_default()
            );
        }
        match field.field_type.as_str() {
            "String" => format!("{value:?}.to_string()"),
            "f64" => value
                .parse::<f64>()
                .map_or_else(|_| value.clone(), |v| format!("{v:?}")),
            _ => value.clone(),
        }
    } else {
        return String::new();
    };
    let Some(vo) = field.value_object.as_deref() else {
        return primitive;
    };
    if is_shared_vo(field, shared_vos) {
        format!("{vo}::new({primitive}).map_err(|_| Error{pascal}::{vo}Invalido)?")
    } else {
        format!("{vo}::new({primitive})?")
    }
}

pub fn field_ctx(field: &Field, pascal: &str, shared_vos: &[ValueObjectDefinition]) -> FieldCtx {
    let vo = field.value_object.clone().unwrap_or_default();
    let is_map = field.field_type == "HashMap<String, String>";
    let default_literal = example_literal(field, &field.field_type).unwrap_or_else(|| {
        resolve_type(&field.field_type)
            .map_or("", |m| m.default_expr)
            .to_string()
    });
    let ref_target = field.references.clone().unwrap_or_default();
    FieldCtx {
        camel: crate::generators::naming::snake_to_camel(&field.name),
        params_literal: if is_reference(field) {
            props_literal(field)
        } else {
            default_literal.clone()
        },
        params_type: if is_reference(field) {
            field_rust_type(field)
        } else {
            field.field_type.clone()
        },
        from_body: match reference_id(field) {
            Some(id) if field.field_type.starts_with("Option<") => {
                format!("cuerpo.{}.map({id}::from_uuid)", field.name)
            }
            Some(id) => format!("{id}::from_uuid(cuerpo.{})", field.name),
            None if field_needs_clone(&field.field_type) => {
                format!("cuerpo.{}.clone()", field.name)
            }
            None => format!("cuerpo.{}", field.name),
        },
        is_ref: is_reference(field),
        ref_target_snake: crate::generators::naming::pascal_to_snake(&ref_target),
        ref_target,
        unique: field.unique,
        private: field.private,
        seed_literal: seed_literal(field),
        field_type: field.field_type.clone(),
        is_map,
        is_array: field.field_type.starts_with("Vec<"),
        accessor: field_value_accessor(field, "entidad"),
        needs_clone: field_needs_clone(&field.field_type),
        from_params: field_vo_conversion(field, "params.", pascal, shared_vos),
        from_props: if is_reference(field) || !is_value_object(field) {
            format!("props.{}", field.name)
        } else {
            field_value_accessor(field, "props")
        },
        default_literal,
        first_variant: field
            .enum_variants
            .as_deref()
            .and_then(<[String]>::first)
            .cloned()
            .unwrap_or_default(),
        from_row: if is_map {
            format!("fila.{}.0", field.name)
        } else if let Some(id) = reference_id(field) {
            if field.field_type.starts_with("Option<") {
                format!("fila.{}.map({id}::from_uuid)", field.name)
            } else {
                format!("{id}::from_uuid(fila.{})", field.name)
            }
        } else {
            field_vo_conversion(field, "fila.", pascal, shared_vos)
        },
        name: field.name.clone(),
        rust_type: field_rust_type(field),
        is_vo: is_value_object(field),
        is_option_vo: is_option_vo(field),
        is_vec_vo: is_vec_vo(field),
        is_enum_vo: is_enum_vo(field),
        is_shared_vo: is_shared_vo(field, shared_vos),
        vo_snake: crate::generators::naming::pascal_to_snake(&vo),
        vo,
        required_string: field.field_type == "String" && !is_value_object(field),
        string_vo: is_value_object(field) && field.field_type == "String" && !is_enum_vo(field),
        props_literal: props_literal(field),
        mother_storage: mother_storage(field),
        mother_param: match mother_storage(field).as_str() {
            "String" => "&str".to_string(),
            storage => storage.to_string(),
        },
        mother_build: mother_build(field),
        in_create: !field.generated && field.default.is_none(),
        in_update: !field.generated,
        generated: field.generated,
        server_value: server_value(field, pascal, shared_vos),
    }
}

/// Inserts `fields` and what crear and actualizar ask for of them (27.17).
pub fn insert_field_lists(ctx: &mut tera::Context, fields: &[FieldCtx]) {
    let create: Vec<&FieldCtx> = fields.iter().filter(|f| f.in_create).collect();
    let update: Vec<&FieldCtx> = fields.iter().filter(|f| f.in_update).collect();
    ctx.insert("fields", fields);
    ctx.insert("create_fields", &create);
    ctx.insert("update_fields", &update);
}

/// The Object Mother stores every field as `Option<…>`, so an `Option` primitive is stored as its
/// inner type and an `Option<Vo>` as the VO.
fn mother_storage(field: &Field) -> String {
    if let Some(id) = reference_id(field) {
        id
    } else if is_value_object(field) && !is_vec_vo(field) {
        field.value_object.clone().unwrap_or_default()
    } else if !is_value_object(field) && field.field_type.starts_with("Option<") {
        vo_inner_type(field)
    } else {
        field_rust_type(field)
    }
}

fn mother_build(field: &Field) -> String {
    let slot = format!("self.{}", field.name);
    if field.field_type.starts_with("Option<") {
        return slot;
    }
    if is_vec_vo(field) {
        return format!("{slot}.unwrap_or_default()");
    }
    if let Some(id) = reference_id(field) {
        return format!("{slot}.unwrap_or_else({id}::generar)");
    }
    if is_value_object(field) {
        return if is_enum_vo(field) {
            format!("{slot}.unwrap_or({})", props_literal(field))
        } else {
            format!("{slot}.unwrap_or_else(|| {})", props_literal(field))
        };
    }
    match field.field_type.as_str() {
        "String" => format!("{slot}.unwrap_or_else(|| {})", props_literal(field)),
        "i64" | "bool" | "f64" => format!("{slot}.unwrap_or({})", props_literal(field)),
        "Uuid" => format!("{slot}.unwrap_or_else(Uuid::new_v4)"),
        "DateTime<Utc>" => format!("{slot}.unwrap_or_else(Utc::now)"),
        _ => format!("{slot}.unwrap_or_default()"),
    }
}

/// A valid value for the field in a Props literal: `None` / `vec![]` for `Option` / `Vec` VOs, the
/// first variant of an enum VO, `Vo::new(default)` for a plain VO, the registry default otherwise.
///
/// **The one place that decides a Props field literal.** The four VO shapes must be handled in the
/// same order everywhere: when the validation-test builders carried their own copy of this
/// branching they omitted `Option`/`Vec`, emitting `Tag::new(vec![])` and `Mid::new(None)` —
/// generated tests that did not compile.
pub fn props_literal(field: &Field) -> String {
    literal_with(field, |ty| {
        example_literal(field, ty)
            .unwrap_or_else(|| resolve_type(ty).map_or("", |m| m.default_expr).to_string())
    })
}

/// The `ejemplo` of the field's shared value object (27.15) as a Rust literal of `ty`, its
/// primitive; `None` without one. An `f64` gets its decimal point (`12` → `12.0`).
fn example_literal(field: &Field, ty: &str) -> Option<String> {
    let example = example_of(field)?;
    match ty {
        "String" => Some(format!("{example:?}.to_string()")),
        "i64" | "bool" => Some(example.to_string()),
        "f64" => example
            .parse::<f64>()
            .ok()
            .map(|value| format!("{value:?}")),
        _ => None,
    }
}

/// The example an `email` value object uses without one of its own (27.16).
const EMAIL_EXAMPLE: &str = "ana@ejemplo.es";

/// The field's example: its shared value object's (27.15), or its class's.
fn example_of(field: &Field) -> Option<&str> {
    field
        .example
        .as_deref()
        .or_else(|| (vo_class(field) == Some("email")).then_some(EMAIL_EXAMPLE))
}

/// [`example_literal`] in TypeScript, for the web's mothers.
fn example_ts(field: &Field) -> Option<String> {
    let example = example_of(field)?;
    match vo_inner_type(field).as_str() {
        "String" => Some(ts_string(example)),
        "i64" | "f64" | "bool" => Some(example.to_string()),
        _ => None,
    }
}

/// Like [`props_literal`], with the values the Pg integration tests seed.
///
/// Differs from the registry's defaults in two ways. Timestamps are a fixed whole second:
/// `Utc::now()` has nanosecond precision and Postgres stores microseconds, so a seeded value never
/// compared equal after a round-trip. And `unique` fields get a fresh value per call: the list test
/// seeds twice, and a constant hit the unique index (`Duplicate`).
pub fn seed_literal(field: &Field) -> String {
    const FIXED_TIME: &str = "chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()";
    literal_with(field, |ty| {
        // A unique field seeds a fresh value per call: its example is one value.
        if let Some(example) = example_literal(field, ty).filter(|_| !field.unique) {
            return example;
        }
        match (ty, field.unique) {
        ("String", true) => "Uuid::new_v4().to_string()".to_string(),
        ("i64", true) => "(Uuid::new_v4().as_u128() >> 65) as i64".to_string(),
        ("f64", true) => "(Uuid::new_v4().as_u128() >> 76) as f64".to_string(),
        ("DateTime<Utc>", true) => {
            "chrono::DateTime::from_timestamp(1_700_000_000 + (Uuid::new_v4().as_u128() >> 100) as i64, 0).unwrap()".to_string()
        }
        ("DateTime<Utc>", false) => FIXED_TIME.to_string(),
        ("HashMap<String, String>", _) => "std::collections::HashMap::new()".to_string(),
        (other, _) => resolve_type(other).map_or("", |m| m.default_expr).to_string(),
    }
    })
}

/// What the mother's `<campo>_unico()` returns for a `unico` field (Fase 27.2): the primitive
/// type and a fresh value per call. `None` where a field has no such function: not unique, a
/// reference or an enum (their seeds come from elsewhere), or a type whose default is fresh.
pub fn unique_value(field: &Field) -> Option<(&'static str, &'static str)> {
    if !field.unique || is_reference(field) || is_enum_vo(field) {
        return None;
    }
    match field.field_type.as_str() {
        // An email's (27.16) still has the form its rule asks for.
        "String" if vo_class(field) == Some("email") => Some((
            "String",
            "format!(\"{}@ejemplo.es\", &uuid::Uuid::new_v4().simple().to_string()[..12])",
        )),
        // 12 hex characters: short enough for a length rule, enough to never repeat in a test.
        "String" => Some((
            "String",
            "uuid::Uuid::new_v4().simple().to_string()[..12].to_string()",
        )),
        "i64" => Some(("i64", "(uuid::Uuid::new_v4().as_u128() >> 65) as i64")),
        "f64" => Some(("f64", "(uuid::Uuid::new_v4().as_u128() >> 76) as f64")),
        "DateTime<Utc>" => Some((
            "chrono::DateTime<chrono::Utc>",
            "chrono::DateTime::from_timestamp(1_700_000_000 + (uuid::Uuid::new_v4().as_u128() >> 100) as i64, 0)\n            .expect(\"una fecha dentro del rango de chrono\")",
        )),
        _ => None,
    }
}

/// The seed of a field with a [`unique_value`]: the mother's, in the field's value-object shape,
/// so a rule of the value object is met in one place the user owns.
pub fn unique_seed(field: &Field, pascal: &str) -> Option<String> {
    unique_value(field)?;
    Some(literal_with(field, |_| {
        format!("Madre{pascal}::{}_unico()", field.name)
    }))
}

/// The value-object shape of a literal around `value(primitive type)`.
fn literal_with(field: &Field, value: impl Fn(&str) -> String) -> String {
    let vo = field.value_object.as_deref().unwrap_or_default();
    if let Some(id) = reference_id(field) {
        if field.field_type.starts_with("Option<") {
            "None".to_string()
        } else {
            format!("{id}::generar()")
        }
    } else if is_option_vo(field) {
        "None".to_string()
    } else if is_vec_vo(field) {
        "vec![]".to_string()
    } else if is_enum_vo(field) {
        let first = field.enum_variants.as_deref().and_then(<[String]>::first);
        format!("{vo}::{}", first.map_or("", String::as_str))
    } else if is_value_object(field) {
        format!(
            "{vo}::new({}).expect(\"{vo}::new rechazó el valor de ejemplo\")",
            value(&field.field_type)
        )
    } else {
        value(&field.field_type)
    }
}

/// Whether reading a primitive field out of a borrowed struct needs `.clone()` — every registry
/// type that is not `Copy`.
pub fn field_needs_clone(field_type: &str) -> bool {
    matches!(
        field_type,
        "String" | "Option<String>" | "Vec<String>" | "Vec<i64>" | "HashMap<String, String>"
    )
}

/// The expression that reads `field` out of `owner` (a borrowed domain entity) as the primitive
/// the persistence row and the DTO hold: `.value()` / `.as_str()` for value objects, mapped over
/// `Option` / `Vec`, `.clone()` for non-`Copy` primitives. The single home of this decision tree —
/// `entity.rs` and `dto.rs` both call it.
pub fn field_value_accessor(field: &Field, owner: &str) -> String {
    let access = format!("{owner}.{}", field.name);
    let string_inner = vo_inner_type(field) == "String";
    if is_reference(field) {
        if field.field_type.starts_with("Option<") {
            format!("{access}.map(|v| v.valor())")
        } else {
            format!("{access}.valor()")
        }
    } else if is_enum_vo(field) {
        format!("{access}.as_str().to_string()")
    } else if is_option_vo(field) {
        if string_inner {
            format!("{access}.as_ref().map(|v| v.valor().to_string())")
        } else {
            format!("{access}.map(|v| v.valor())")
        }
    } else if is_vec_vo(field) {
        if string_inner {
            format!("{access}.iter().map(|v| v.valor().to_string()).collect::<Vec<_>>()")
        } else {
            format!("{access}.iter().map(|v| v.valor()).collect::<Vec<_>>()")
        }
    } else if is_value_object(field) {
        if string_inner {
            format!("{access}.valor().to_string()")
        } else {
            format!("{access}.valor()")
        }
    } else if field_needs_clone(&field.field_type) {
        format!("{access}.clone()")
    } else {
        access
    }
}

/// The expression that turns `{source}{field}` (a primitive: a use case param, a DB row column)
/// into the field's domain type — parsing an enum VO, mapping over `Option` / `Vec`, constructing
/// a plain VO — with `?`. A shared VO has its own error type, mapped to the entity's
/// `Invalid{Vo}`. A primitive field is the source itself.
pub fn field_vo_conversion(
    field: &Field,
    source: &str,
    pascal: &str,
    shared_vos: &[ValueObjectDefinition],
) -> String {
    let value = format!("{source}{}", field.name);
    let Some(vo) = field.value_object.as_deref() else {
        return value;
    };
    let conversion = if is_enum_vo(field) {
        format!("{value}.parse::<{vo}>()")
    } else if is_option_vo(field) {
        format!("{value}.map({vo}::new).transpose()")
    } else if is_vec_vo(field) {
        format!("{value}.into_iter().map({vo}::new).collect::<Result<Vec<_>,_>>()")
    } else {
        format!("{vo}::new({value})")
    };
    if is_shared_vo(field, shared_vos) {
        format!("{conversion}.map_err(|_| Error{pascal}::{vo}Invalido)?")
    } else {
        format!("{conversion}?")
    }
}

// ── Frontend (`web/`) ─────────────────────────────────────────────────────────

/// One `<option>` of a `<select>`.
#[derive(Debug, serde::Serialize, PartialEq)]
pub struct WebOption {
    pub value: String,
    pub label: String,
}

/// What the frontend templates know about one field (ADR-0011, applied to `web/`): its names, its
/// input, and the expressions that move it between the API's types, the form and a screen
/// (ADR-0025: the screens use the API's types). Expressions name helpers of
/// `compartido/formularios.ts`; the `*_helpers` lists say which, so a file imports exactly those.
#[derive(Debug, serde::Serialize)]
pub struct WebFieldCtx {
    /// The JSON key: poem-openapi's camelCase (`inStock`).
    pub camel: String,
    /// What a screen calls it: `In stock`.
    pub label: String,
    /// `text`, `number`, `checkbox`, `datetime-local`, `textarea`, `select`, or `reference`: a
    /// select of the target's items, loaded by the form container (ADR-0022).
    pub input: &'static str,
    /// A number input that takes decimals (`step="any"`).
    pub decimal: bool,
    /// The input must be filled: not an `Option`, a `Vec`, a map or a checkbox.
    pub required: bool,
    /// A `select`'s options, after an empty one labelled `placeholder` (none for a required enum:
    /// it starts on its first variant).
    pub options: Vec<WebOption>,
    pub placeholder: String,
    /// An enum: what each variant reads as on a screen (27.9), in the views' `TEXTOS_<CAMPO>`.
    pub is_enum: bool,
    pub texts_const: String,
    /// The form value of a new, empty form.
    pub empty_form: String,
    /// The form value read out of `{entity}` (the entity's variable) and out of `peticion`. An
    /// `Option` is an optional property that the API answers as `null` when it is absent.
    pub to_form: String,
    pub request_to_form: String,
    /// The request's value read out of `valores` (the form): `undefined` for an empty optional one.
    pub from_form: String,
    /// What a list or detail screen shows for `{entity}`, as a JSX expression.
    pub display: String,
    /// A valid value, for the Object Mother: a present one for an `Option`, a non-empty one for a
    /// `Vec` or a map, so the form round-trip tests exercise the conversion.
    pub mother: String,
    pub to_form_helpers: Vec<&'static str>,
    pub from_form_helpers: Vec<&'static str>,
    pub display_helpers: Vec<&'static str>,
    /// A reference: the target's list path, which its detail page sits under (`/customers`).
    pub ref_path: String,
    /// 27.17: a `por_defecto` field is hidden when creating; a `generado` one is in no form.
    pub defaulted: bool,
    pub generated: bool,
}

/// A JSON string literal (double quotes, as Biome writes them).
fn ts_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A fixed whole-minute instant: a `datetime-local` input round-trips it unchanged.
const MOTHER_DATE_TIME: &str = "\"2026-01-01T10:00:00.000Z\"";

/// The shape of a field on the frontend, which every per-field expression branches on.
#[derive(Clone, Copy, PartialEq)]
enum WebShape {
    Text,
    Number,
    /// A `dinero` value object (27.16): cents, typed in euros with a comma.
    Dinero,
    Checkbox,
    DateTime,
    List {
        numbers: bool,
    },
    Map,
    Enum,
}

fn web_shape(field: &Field) -> WebShape {
    let ty = field.field_type.as_str();
    if is_enum_vo(field) {
        WebShape::Enum
    } else if ty.starts_with("Vec<") {
        WebShape::List {
            numbers: vo_inner_type(field) == "i64",
        }
    } else if ty == "HashMap<String, String>" {
        WebShape::Map
    } else if vo_class(field) == Some("dinero") {
        WebShape::Dinero
    } else {
        match vo_inner_type(field).as_str() {
            "i64" | "f64" => WebShape::Number,
            "bool" => WebShape::Checkbox,
            "DateTime<Utc>" => WebShape::DateTime,
            _ => WebShape::Text,
        }
    }
}

/// The form text of `{owner}.{camel}`, and the helpers that expression calls. A private field is
/// absent from what a public route answers a caller without a session (Phase 20.9).
fn web_to_form(
    shape: WebShape,
    optional: bool,
    private: bool,
    access: &str,
) -> (String, Vec<&'static str>) {
    match shape {
        WebShape::DateTime => (
            format!("aFechaHoraLocal({access})"),
            vec!["aFechaHoraLocal"],
        ),
        WebShape::List { .. } => (format!("{access}.join(\", \")"), vec![]),
        WebShape::Map => (
            format!("formatearEntradas({access})"),
            vec!["formatearEntradas"],
        ),
        WebShape::Dinero => (format!("aImporte({access})"), vec!["aImporte"]),
        _ if optional => (format!("textoDe({access})"), vec!["textoDe"]),
        WebShape::Number => (format!("String({access})"), vec![]),
        _ if private => (format!("{access} ?? \"\""), vec![]),
        _ => (access.to_string(), vec![]),
    }
}

/// What a screen shows for `{owner}.{camel}`, and the helpers that expression calls.
fn web_display(shape: WebShape, optional: bool, access: &str) -> (String, Vec<&'static str>) {
    let (shown, helpers) = match shape {
        WebShape::Checkbox => (format!("{access} ? \"Sí\" : \"No\""), vec![]),
        WebShape::DateTime => (
            format!("formatearFechaHora({access})"),
            vec!["formatearFechaHora"],
        ),
        WebShape::List { .. } => (format!("{access}.join(\", \")"), vec![]),
        WebShape::Map => (
            format!("formatearEntradas({access}, \", \")"),
            vec!["formatearEntradas"],
        ),
        WebShape::Dinero => (format!("euros({access})"), vec!["euros"]),
        _ => (access.to_string(), vec![]),
    };
    match (optional, shape) {
        (true, WebShape::Checkbox | WebShape::DateTime) => {
            (format!("{access} == null ? \"—\" : {shown}"), helpers)
        }
        (true, _) => (format!("{access} ?? \"—\""), helpers),
        _ => (shown, helpers),
    }
}

pub fn web_field_ctx(field: &Field, entity_ident: &str) -> WebFieldCtx {
    let camel = crate::generators::naming::snake_to_camel(&field.name);
    // A reference is named after what it points to: `customer_id` is the `Customer` input.
    let named = match field.references {
        Some(_) => field.name.strip_suffix("_id").unwrap_or(&field.name),
        None => &field.name,
    };
    // A reference's target list path, which its detail page sits under (`/clientes`).
    let target = field.references.as_deref().map(|target| {
        crate::generators::naming::pluralize(&crate::generators::naming::pascal_to_snake(target))
    });
    let optional = field.field_type.starts_with("Option<");
    let shape = web_shape(field);
    let variants = field.enum_variants.clone().unwrap_or_default();
    let value = format!("valores.{camel}");

    let (input, mother) = match shape {
        WebShape::Enum => (
            "select",
            variants
                .first()
                .map_or("\"\"".to_string(), |v| ts_string(v)),
        ),
        WebShape::List { numbers: true } => ("text", "[42]".into()),
        WebShape::List { numbers: false } => ("text", "[\"example\"]".into()),
        WebShape::Map => ("textarea", "{ key: \"value\" }".into()),
        WebShape::Dinero => ("text", "1250".into()),
        WebShape::Number => (
            "number",
            if vo_inner_type(field) == "f64" {
                "1.5"
            } else {
                "42"
            }
            .into(),
        ),
        WebShape::Checkbox => (if optional { "select" } else { "checkbox" }, "true".into()),
        WebShape::DateTime => ("datetime-local", MOTHER_DATE_TIME.into()),
        WebShape::Text if target.is_some() => ("reference", "crypto.randomUUID()".into()),
        WebShape::Text if vo_inner_type(field) == "Uuid" => ("text", "crypto.randomUUID()".into()),
        WebShape::Text if vo_class(field) == Some("email") => ("email", "\"example\"".into()),
        WebShape::Text => ("text", "\"example\"".into()),
    };
    // A shared value object's example (27.15) is the valid value its rule accepts.
    let mother = match shape {
        WebShape::Text | WebShape::Number | WebShape::Checkbox | WebShape::Dinero => {
            example_ts(field).unwrap_or(mother)
        }
        _ => mother,
    };

    // An enum is the API's `string`: its select only offers the variants. A required text or number
    // needs no helper.
    let from_form_helper = match shape {
        WebShape::List { numbers } => Some(if numbers {
            "separarNumeros"
        } else {
            "separarLista"
        }),
        WebShape::Map => Some("leerEntradas"),
        WebShape::Dinero => Some("deImporte"),
        WebShape::DateTime if optional => Some("fechaHoraOpcional"),
        WebShape::DateTime => Some("deFechaHoraLocal"),
        WebShape::Number if optional => Some("numeroOpcional"),
        WebShape::Checkbox if optional => Some("booleanoOpcional"),
        WebShape::Text if optional => Some("textoOpcional"),
        _ => None,
    };
    let from_form = match (shape, from_form_helper) {
        (_, Some(helper)) => format!("{helper}({value})"),
        (WebShape::Number, None) => format!("Number({value})"),
        _ => value.clone(),
    };

    let options = match (shape, input) {
        (WebShape::Enum, _) => variants
            .iter()
            .map(|v| WebOption {
                value: v.clone(),
                label: field
                    .texts
                    .as_ref()
                    .and_then(|texts| texts.get(v).cloned())
                    .unwrap_or_else(|| crate::generators::naming::texto_variante(v)),
            })
            .collect(),
        (_, "select") => vec![
            WebOption {
                value: "true".into(),
                label: "Sí".into(),
            },
            WebOption {
                value: "false".into(),
                label: "No".into(),
            },
        ],
        _ => vec![],
    };

    let (to_form, to_form_helpers) = web_to_form(
        shape,
        optional,
        field.private,
        &format!("{entity_ident}.{camel}"),
    );
    // A request always holds a private field.
    let (request_to_form, _) = web_to_form(shape, optional, false, &format!("peticion.{camel}"));
    let texts_const = format!("TEXTOS_{}", field.name.to_uppercase());
    let access = format!("{entity_ident}.{camel}");
    let (display, display_helpers) = if shape == WebShape::Enum {
        // A variant the texts do not know (an older API) shows as it is.
        let shown = format!("{texts_const}[{access}] ?? {access}");
        let display = if optional {
            format!("{access} == null ? \"—\" : {shown}")
        } else {
            shown
        };
        (display, vec![])
    } else {
        web_display(shape, optional, &access)
    };

    WebFieldCtx {
        label: field
            .label
            .clone()
            .unwrap_or_else(|| crate::generators::naming::etiqueta(named)),
        input,
        decimal: vo_inner_type(field) == "f64",
        required: !optional
            && !matches!(
                shape,
                WebShape::Checkbox | WebShape::List { .. } | WebShape::Map
            ),
        // No article and no adjective: the entity's gender is unknown (Fase 22).
        placeholder: if target.is_some() && !optional {
            "Elige una opción".to_string()
        } else {
            "—".to_string()
        },
        options,
        // A required enum starts on its first variant: an empty one could not be sent. A field the
        // server defaults (27.17) starts on its default: its input is hidden when creating.
        empty_form: match (input, shape, variants.first()) {
            ("checkbox", _, _) if field.default.is_some() => {
                field.default.clone().unwrap_or_default()
            }
            _ if field.default.is_some() => ts_string(field.default.as_deref().unwrap_or_default()),
            ("checkbox", _, _) => "false".to_string(),
            (_, WebShape::Enum, Some(first)) if !optional => ts_string(first),
            _ => "\"\"".to_string(),
        },
        is_enum: shape == WebShape::Enum,
        defaulted: field.default.is_some(),
        generated: field.generated,
        texts_const,
        to_form,
        request_to_form,
        from_form,
        display,
        mother,
        to_form_helpers,
        from_form_helpers: from_form_helper.into_iter().collect(),
        display_helpers,
        ref_path: target
            .as_ref()
            .map_or_else(String::new, |plural| format!("/{plural}")),
        camel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_string() {
        let m = resolve_type("String").unwrap();
        assert_eq!(m.rust_type, "String");
        assert_eq!(m.sql_type, "TEXT");
        assert!(!m.sql_nullable);
    }

    #[test]
    fn resolve_option_string() {
        let m = resolve_type("Option<String>").unwrap();
        assert_eq!(m.rust_type, "Option<String>");
        assert_eq!(m.sql_type, "TEXT");
        assert!(m.sql_nullable);
    }

    #[test]
    fn resolve_uuid() {
        let m = resolve_type("Uuid").unwrap();
        assert_eq!(m.sql_type, "UUID");
        assert_eq!(m.needs_import, Some("uuid::Uuid"));
    }

    #[test]
    fn resolve_datetime() {
        let m = resolve_type("DateTime<Utc>").unwrap();
        assert_eq!(m.sql_type, "TIMESTAMPTZ");
        assert_eq!(m.needs_import, Some("chrono::{DateTime, Utc}"));
    }

    #[test]
    fn resolve_vec_string() {
        let m = resolve_type("Vec<String>").unwrap();
        assert_eq!(m.sql_type, "TEXT[]");
        assert!(!m.sql_nullable);
    }

    #[test]
    fn resolve_hashmap() {
        let m = resolve_type("HashMap<String, String>").unwrap();
        assert_eq!(m.sql_type, "JSONB");
        assert_eq!(m.needs_import, Some("std::collections::HashMap"));
    }

    #[test]
    fn resolve_unknown_type_errors() {
        let err = resolve_type("CustomType").unwrap_err();
        assert!(err.contains("tipo de campo no admitido 'CustomType'"));
        assert!(err.contains("String"));
    }

    #[test]
    fn validate_fields_ok() {
        use crate::puerto_toml::Field;
        let fields = vec![
            Field {
                name: "name".into(),
                field_type: "String".into(),
                unique: false,
                private: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
                label: None,
                texts: None,
                example: None,
                default: None,
                generated: false,
            },
            Field {
                name: "price".into(),
                field_type: "i64".into(),
                unique: false,
                private: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
                label: None,
                texts: None,
                example: None,
                default: None,
                generated: false,
            },
        ];
        assert!(validate_fields(&fields).is_ok());
    }

    #[test]
    fn validate_fields_unknown_errors() {
        use crate::puerto_toml::Field;
        let fields = vec![
            Field {
                name: "name".into(),
                field_type: "String".into(),
                unique: false,
                private: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
                label: None,
                texts: None,
                example: None,
                default: None,
                generated: false,
            },
            Field {
                name: "bad".into(),
                field_type: "UnknownType".into(),
                unique: false,
                private: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
                label: None,
                texts: None,
                example: None,
                default: None,
                generated: false,
            },
        ];
        let err = validate_fields(&fields).unwrap_err();
        assert!(err.contains("tipo de campo no admitido 'UnknownType'"));
    }

    fn field(field_type: &str, vo: Option<&str>) -> crate::puerto_toml::Field {
        crate::puerto_toml::Field {
            name: "x".into(),
            field_type: field_type.into(),
            value_object: vo.map(Into::into),
            ..Default::default()
        }
    }

    #[test]
    fn should_tell_option_and_vec_value_objects_apart() {
        assert!(is_option_vo(&field("Option<String>", Some("Mid"))));
        assert!(!is_option_vo(&field("String", Some("Name"))));
        assert!(is_vec_vo(&field("Vec<String>", Some("Tag"))));
        assert!(!is_vec_vo(&field("String", Some("Name"))));
    }

    #[test]
    fn should_unwrap_the_inner_type_of_option_and_vec() {
        assert_eq!(
            vo_inner_type(&field("Option<String>", Some("Mid"))),
            "String"
        );
        assert_eq!(vo_inner_type(&field("Vec<i64>", Some("Score"))), "i64");
        assert_eq!(vo_inner_type(&field("Uuid", Some("Ref"))), "Uuid");
    }

    #[test]
    fn should_import_only_what_a_value_literal_names() {
        assert_eq!(
            literal_import(&field("DateTime<Utc>", None)),
            Some("chrono::Utc")
        );
        assert_eq!(literal_import(&field("Uuid", None)), Some("uuid::Uuid"));
        assert_eq!(
            literal_import(&field("HashMap<String, String>", None)),
            Some("std::collections::HashMap")
        );
        // `None` / `vec![]` literals name no type.
        assert_eq!(literal_import(&field("Option<DateTime<Utc>>", None)), None);
        assert_eq!(literal_import(&field("String", None)), None);
    }

    #[test]
    fn should_name_the_vo_only_in_plain_and_enum_literals() {
        assert!(literal_names_vo(&field("String", Some("Name"))));
        assert!(!literal_names_vo(&field("Option<String>", Some("Mid"))));
        assert!(!literal_names_vo(&field("Vec<String>", Some("Tag"))));
        assert!(!literal_names_vo(&field("String", None)));
    }

    fn enum_field() -> crate::puerto_toml::Field {
        crate::puerto_toml::Field {
            value_object_kind: Some("enum".into()),
            enum_variants: Some(vec!["Active".into()]),
            ..field("String", Some("Status"))
        }
    }

    #[test]
    fn should_clone_only_the_heap_backed_primitives() {
        for ty in [
            "String",
            "Option<String>",
            "Vec<String>",
            "Vec<i64>",
            "HashMap<String, String>",
        ] {
            assert!(field_needs_clone(ty), "{ty} is not Copy");
        }
        for ty in [
            "i64",
            "bool",
            "f64",
            "Uuid",
            "DateTime<Utc>",
            "Option<i64>",
            "Option<bool>",
            "Option<f64>",
            "Option<DateTime<Utc>>",
        ] {
            assert!(!field_needs_clone(ty), "{ty} is Copy");
        }
    }

    #[test]
    fn should_read_the_primitive_out_of_every_value_object_shape() {
        // {plain VO, Option<VO>, Vec<VO>, primitive} × {String, i64, Uuid, DateTime}: the one place
        // that decides how a domain field becomes a primitive (entity.rs `From`, dto.rs `from_domain`).
        let cases: &[(&str, Option<&str>, &str)] = &[
            ("String", Some("V"), "entidad.x.valor().to_string()"),
            ("i64", Some("V"), "entidad.x.valor()"),
            ("Uuid", Some("V"), "entidad.x.valor()"),
            ("DateTime<Utc>", Some("V"), "entidad.x.valor()"),
            (
                "Option<String>",
                Some("V"),
                "entidad.x.as_ref().map(|v| v.valor().to_string())",
            ),
            ("Option<i64>", Some("V"), "entidad.x.map(|v| v.valor())"),
            ("Option<Uuid>", Some("V"), "entidad.x.map(|v| v.valor())"),
            (
                "Option<DateTime<Utc>>",
                Some("V"),
                "entidad.x.map(|v| v.valor())",
            ),
            (
                "Vec<String>",
                Some("V"),
                "entidad.x.iter().map(|v| v.valor().to_string()).collect::<Vec<_>>()",
            ),
            (
                "Vec<i64>",
                Some("V"),
                "entidad.x.iter().map(|v| v.valor()).collect::<Vec<_>>()",
            ),
            (
                "Vec<Uuid>",
                Some("V"),
                "entidad.x.iter().map(|v| v.valor()).collect::<Vec<_>>()",
            ),
            (
                "Vec<DateTime<Utc>>",
                Some("V"),
                "entidad.x.iter().map(|v| v.valor()).collect::<Vec<_>>()",
            ),
            ("String", None, "entidad.x.clone()"),
            ("i64", None, "entidad.x"),
            ("Uuid", None, "entidad.x"),
            ("DateTime<Utc>", None, "entidad.x"),
            ("Option<String>", None, "entidad.x.clone()"),
            ("Option<i64>", None, "entidad.x"),
            ("Vec<String>", None, "entidad.x.clone()"),
            ("HashMap<String, String>", None, "entidad.x.clone()"),
        ];
        for (ty, vo, expected) in cases {
            assert_eq!(
                field_value_accessor(&field(ty, *vo), "entidad"),
                *expected,
                "{ty} / {vo:?}"
            );
        }
        assert_eq!(
            field_value_accessor(&enum_field(), "entidad"),
            "entidad.x.as_str().to_string()"
        );
        assert_eq!(
            field_value_accessor(&field("i64", None), "cuerpo"),
            "cuerpo.x"
        );
    }

    #[test]
    fn should_describe_a_field_to_the_templates() {
        let shared = [ValueObjectDefinition {
            name: "Email".into(),
            inner_type: "String".into(),
            ..Default::default()
        }];

        let plain = field_ctx(&field("String", None), "P", &shared);
        assert!(plain.required_string && !plain.is_vo && !plain.string_vo);
        assert_eq!(
            (plain.rust_type.as_str(), plain.vo.as_str()),
            ("String", "")
        );

        let name = field_ctx(&field("String", Some("FullName")), "P", &shared);
        assert!(name.is_vo && name.string_vo && !name.required_string && !name.is_shared_vo);
        assert_eq!(
            (name.vo.as_str(), name.vo_snake.as_str()),
            ("FullName", "full_name")
        );

        let email = field_ctx(&field("String", Some("Email")), "P", &shared);
        assert!(email.is_shared_vo && email.string_vo);

        let status = field_ctx(&enum_field(), "P", &shared);
        assert!(status.is_enum_vo && status.is_vo && !status.string_vo);
        assert_eq!(status.first_variant, "Active");
        assert_eq!(status.from_params, "params.x.parse::<Status>()?");
        assert_eq!(status.default_literal, "\"example\".to_string()");

        let mid = field_ctx(&field("Option<String>", Some("Mid")), "P", &shared);
        assert!(mid.is_option_vo && !mid.is_vec_vo && !mid.string_vo);
        assert_eq!(mid.rust_type, "Option<Mid>");

        let tags = field_ctx(&field("Vec<String>", Some("Tag")), "P", &shared);
        assert!(tags.is_vec_vo && !tags.is_option_vo && !tags.string_vo);

        let count = field_ctx(&field("i64", None), "P", &shared);
        assert!(!count.required_string && !count.is_vo);
    }

    #[test]
    fn should_take_the_shared_value_object_example_wherever_a_test_needs_a_value() {
        let shared = |name: &str, ty: &str, example: &str| Field {
            name: name.into(),
            field_type: ty.into(),
            value_object: Some(crate::generators::naming::to_pascal_case(name)),
            example: Some(example.into()),
            ..Default::default()
        };
        let email = shared("email", "String", "ana@ejemplo.es");
        let ratio = shared("ratio", "f64", "12");
        let activo = shared("activo", "bool", "false");

        assert_eq!(
            props_literal(&email),
            "Email::new(\"ana@ejemplo.es\".to_string()).expect(\"Email::new rechazó el valor de ejemplo\")"
        );
        assert_eq!(
            field_ctx(&email, "Cliente", &[]).default_literal,
            "\"ana@ejemplo.es\".to_string()"
        );
        assert_eq!(field_ctx(&ratio, "Cliente", &[]).default_literal, "12.0");
        assert_eq!(field_ctx(&activo, "Cliente", &[]).default_literal, "false");
        assert!(seed_literal(&email).contains("\"ana@ejemplo.es\""));
        let unique = Field {
            unique: true,
            ..email.clone()
        };
        assert!(
            !seed_literal(&unique).contains("ana@ejemplo.es"),
            "a fresh value per seed"
        );
        assert_eq!(
            web_field_ctx(&email, "cliente").mother,
            "\"ana@ejemplo.es\""
        );
        assert_eq!(web_field_ctx(&ratio, "cliente").mother, "12");
    }

    #[test]
    fn should_write_a_valid_default_literal_for_every_field_shape() {
        assert_eq!(
            props_literal(&field("String", None)),
            "\"example\".to_string()"
        );
        assert_eq!(props_literal(&field("i64", None)), "42");
        assert_eq!(props_literal(&field("Option<String>", Some("Mid"))), "None");
        assert_eq!(props_literal(&field("Vec<String>", Some("Tag"))), "vec![]");
        assert_eq!(props_literal(&enum_field()), "Status::Active");
        assert_eq!(
            props_literal(&field("i64", Some("Age"))),
            "Age::new(42).expect(\"Age::new rechazó el valor de ejemplo\")"
        );
    }

    #[test]
    fn should_store_build_and_set_every_field_shape_in_the_object_mother() {
        // (field, storage type, `with_` parameter, `build()` value)
        let cases = [
            (
                field("String", None),
                "String",
                "&str",
                "self.x.unwrap_or_else(|| \"example\".to_string())",
            ),
            (field("i64", None), "i64", "i64", "self.x.unwrap_or(42)"),
            (
                field("Uuid", None),
                "Uuid",
                "Uuid",
                "self.x.unwrap_or_else(Uuid::new_v4)",
            ),
            (
                field("DateTime<Utc>", None),
                "DateTime<Utc>",
                "DateTime<Utc>",
                "self.x.unwrap_or_else(Utc::now)",
            ),
            (field("Option<String>", None), "String", "&str", "self.x"),
            (field("Option<i64>", None), "i64", "i64", "self.x"),
            (
                field("Vec<String>", None),
                "Vec<String>",
                "Vec<String>",
                "self.x.unwrap_or_default()",
            ),
            (
                field("HashMap<String, String>", None),
                "HashMap<String, String>",
                "HashMap<String, String>",
                "self.x.unwrap_or_default()",
            ),
            (
                field("String", Some("Name")),
                "Name",
                "Name",
                "self.x.unwrap_or_else(|| Name::new(\"example\".to_string()).expect(\"Name::new rechazó el valor de ejemplo\"))",
            ),
            (field("Option<String>", Some("Mid")), "Mid", "Mid", "self.x"),
            (
                field("Vec<String>", Some("Tag")),
                "Vec<Tag>",
                "Vec<Tag>",
                "self.x.unwrap_or_default()",
            ),
            (
                enum_field(),
                "Status",
                "Status",
                "self.x.unwrap_or(Status::Active)",
            ),
        ];
        for (f, storage, param, build) in cases {
            let ctx = field_ctx(&f, "P", &[]);
            assert_eq!(ctx.mother_storage, storage, "{}", f.field_type);
            assert_eq!(ctx.mother_param, param, "{}", f.field_type);
            assert_eq!(ctx.mother_build, build, "{}", f.field_type);
        }
    }

    #[test]
    fn should_convert_a_primitive_source_into_every_value_object_shape() {
        let shared = [ValueObjectDefinition {
            name: "V".into(),
            inner_type: "String".into(),
            ..Default::default()
        }];
        let local = |ty: &str| field_vo_conversion(&field(ty, Some("V")), "fila.", "P", &[]);
        let shared_ = |ty: &str| field_vo_conversion(&field(ty, Some("V")), "fila.", "P", &shared);

        assert_eq!(local("String"), "V::new(fila.x)?");
        assert_eq!(local("Option<String>"), "fila.x.map(V::new).transpose()?");
        assert_eq!(
            local("Vec<String>"),
            "fila.x.into_iter().map(V::new).collect::<Result<Vec<_>,_>>()?"
        );
        assert_eq!(
            field_vo_conversion(&enum_field(), "fila.", "P", &[]),
            "fila.x.parse::<Status>()?"
        );
        assert_eq!(
            shared_("String"),
            "V::new(fila.x).map_err(|_| ErrorP::VInvalido)?"
        );
        assert_eq!(
            shared_("Option<String>"),
            "fila.x.map(V::new).transpose().map_err(|_| ErrorP::VInvalido)?"
        );
        assert_eq!(
            field_vo_conversion(&field("i64", None), "fila.", "P", &[]),
            "fila.x"
        );
    }

    #[test]
    fn should_seed_fixed_timestamps_and_fresh_unique_values() {
        let unique = |ty: &str| crate::puerto_toml::Field {
            unique: true,
            ..field(ty, None)
        };
        let fixed = "chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()";
        assert_eq!(seed_literal(&field("DateTime<Utc>", None)), fixed);
        assert_eq!(
            seed_literal(&unique("String")),
            "Uuid::new_v4().to_string()"
        );
        assert_eq!(
            seed_literal(&unique("i64")),
            "(Uuid::new_v4().as_u128() >> 65) as i64"
        );
        assert_eq!(
            seed_literal(&field("String", None)),
            "\"example\".to_string()"
        );
        assert_eq!(
            seed_literal(&field("HashMap<String, String>", None)),
            "std::collections::HashMap::new()"
        );
        assert_eq!(
            seed_literal(&field("DateTime<Utc>", Some("Joined"))),
            format!("Joined::new({fixed}).expect(\"Joined::new rechazó el valor de ejemplo\")")
        );
        assert_eq!(seed_literal(&field("Vec<String>", Some("Tag"))), "vec![]");
        assert_eq!(seed_literal(&enum_field()), "Status::Active");
    }

    // ── Frontend ──────────────────────────────────────────────────────────────

    fn named(name: &str, field_type: &str) -> crate::puerto_toml::Field {
        crate::puerto_toml::Field {
            name: name.into(),
            field_type: field_type.into(),
            ..Default::default()
        }
    }

    #[test]
    fn should_move_a_number_through_the_form_as_text() {
        let ctx = web_field_ctx(&named("unit_price", "i64"), "orderItem");

        assert_eq!(ctx.camel, "unitPrice");
        assert_eq!(ctx.label, "Unit price");
        assert_eq!(ctx.input, "number");
        assert!(!ctx.decimal && ctx.required);
        assert_eq!(ctx.to_form, "String(orderItem.unitPrice)");
        assert_eq!(ctx.request_to_form, "String(peticion.unitPrice)");
        assert_eq!(ctx.from_form, "Number(valores.unitPrice)");
        assert_eq!(ctx.display, "orderItem.unitPrice");
        assert_eq!(ctx.mother, "42");
    }

    #[test]
    fn should_give_decimals_a_step_and_booleans_a_checkbox() {
        assert!(web_field_ctx(&field("f64", None), "p").decimal);

        let flag = web_field_ctx(&field("bool", None), "p");
        assert_eq!(flag.input, "checkbox");
        assert!(!flag.required);
        assert_eq!(flag.empty_form, "false");
        assert_eq!(
            (flag.to_form.as_str(), flag.from_form.as_str()),
            ("p.x", "valores.x")
        );
        assert_eq!(flag.display, "p.x ? \"Sí\" : \"No\"");
    }

    #[test]
    fn should_keep_strings_uuids_and_value_objects_as_text() {
        let id = web_field_ctx(&field("Uuid", None), "p");
        assert_eq!(id.input, "text");
        assert_eq!(id.mother, "crypto.randomUUID()");

        let name = web_field_ctx(&field("String", Some("Name")), "p");
        assert_eq!((name.input, name.empty_form.as_str()), ("text", "\"\""));
        assert_eq!(
            (name.mother.as_str(), name.to_form.as_str()),
            ("\"example\"", "p.x")
        );
    }

    #[test]
    fn should_read_a_private_field_as_absent_and_always_send_it() {
        let email = crate::puerto_toml::Field {
            private: true,
            ..field("String", None)
        };
        let ctx = web_field_ctx(&email, "p");

        assert_eq!(ctx.to_form, "p.x ?? \"\"");
        assert_eq!(ctx.request_to_form, "peticion.x");
        assert_eq!(ctx.from_form, "valores.x");
    }

    #[test]
    fn should_read_an_absent_option_as_empty_text_and_leave_an_empty_one_out() {
        let views = web_field_ctx(&field("Option<i64>", None), "p");

        assert!(!views.required);
        assert_eq!(
            (views.to_form.as_str(), views.from_form.as_str()),
            ("textoDe(p.x)", "numeroOpcional(valores.x)")
        );
        assert_eq!(views.display, "p.x ?? \"—\"");
        assert_eq!(views.mother, "42", "present, so the round trip converts it");
        assert_eq!(views.to_form_helpers, ["textoDe"]);
        assert_eq!(views.from_form_helpers, ["numeroOpcional"]);

        let middle = web_field_ctx(&field("Option<String>", Some("MiddleName")), "p");
        assert_eq!(middle.from_form, "textoOpcional(valores.x)");
    }

    #[test]
    fn should_choose_an_optional_boolean_from_a_select() {
        let flag = web_field_ctx(&field("Option<bool>", None), "p");

        assert_eq!(flag.input, "select");
        assert_eq!(flag.empty_form, "\"\"");
        assert_eq!(
            flag.options
                .iter()
                .map(|o| o.value.as_str())
                .collect::<Vec<_>>(),
            ["true", "false"]
        );
        assert_eq!(flag.from_form, "booleanoOpcional(valores.x)");
        assert_eq!(flag.placeholder, "—");
        // The API answers `null` for an absent one, though its type says `undefined`.
        assert_eq!(flag.display, "p.x == null ? \"—\" : p.x ? \"Sí\" : \"No\"");
    }

    #[test]
    fn should_edit_a_date_time_in_local_time_and_send_iso_8601() {
        let at = web_field_ctx(&field("DateTime<Utc>", None), "p");
        assert_eq!(at.input, "datetime-local");
        assert!(at.required);
        assert_eq!(
            (at.to_form.as_str(), at.from_form.as_str()),
            ("aFechaHoraLocal(p.x)", "deFechaHoraLocal(valores.x)")
        );
        assert_eq!(at.display, "formatearFechaHora(p.x)");
        assert_eq!(at.display_helpers, ["formatearFechaHora"]);
        assert_eq!(at.mother, MOTHER_DATE_TIME);

        let expires = web_field_ctx(&field("Option<DateTime<Utc>>", None), "p");
        assert_eq!(expires.from_form, "fechaHoraOpcional(valores.x)");
        assert_eq!(
            expires.display,
            "p.x == null ? \"—\" : formatearFechaHora(p.x)"
        );
    }

    #[test]
    fn should_write_a_list_comma_separated_and_a_map_one_entry_per_line() {
        let tags = web_field_ctx(&field("Vec<String>", Some("Tag")), "p");
        assert_eq!(tags.input, "text");
        assert!(!tags.required);
        assert_eq!(
            (tags.to_form.as_str(), tags.from_form.as_str()),
            ("p.x.join(\", \")", "separarLista(valores.x)")
        );
        assert_eq!(tags.mother, "[\"example\"]");

        let scores = web_field_ctx(&field("Vec<i64>", None), "p");
        assert_eq!(scores.from_form, "separarNumeros(valores.x)");

        let meta = web_field_ctx(&field("HashMap<String, String>", None), "p");
        assert_eq!(meta.input, "textarea");
        assert_eq!(
            (meta.to_form.as_str(), meta.from_form.as_str()),
            ("formatearEntradas(p.x)", "leerEntradas(valores.x)")
        );
        assert_eq!(meta.display, "formatearEntradas(p.x, \", \")");
    }

    #[test]
    fn should_send_an_enum_as_the_string_its_select_chose() {
        let ctx = web_field_ctx(&enum_field(), "p");

        assert_eq!(ctx.input, "select");
        assert!(ctx.required);
        assert_eq!(ctx.from_form, format!("valores.{}", ctx.camel));
        // 27.9: a required enum starts on its first variant, with no empty option to choose.
        assert_eq!(ctx.empty_form, "\"Active\"");
        assert_eq!(
            ctx.options.first().map(|o| o.value.as_str()),
            Some("Active")
        );
        assert_eq!(ctx.mother, "\"Active\"");
    }
}
