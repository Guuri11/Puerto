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
                "unsupported field type '{}'. Supported types: {}",
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
        .map(|target| format!("{target}Id"))
}

/// The path of a reference's id type from crate `krate` (`crate` in `business`, `business`
/// elsewhere): `{krate}::domain::customer::model::CustomerId`.
pub fn reference_import(field: &Field, krate: &str) -> Option<String> {
    let target = field.references.as_deref()?;
    let snake = crate::generators::naming::pascal_to_snake(target);
    Some(format!("{krate}::domain::{snake}::model::{target}Id"))
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

pub fn vo_import_path(field: &Field, snake: &str, shared_vos: &[ValueObjectDefinition]) -> String {
    if is_shared_vo(field, shared_vos) {
        format!(
            "crate::domain::shared::value_objects::{}",
            field.value_object.as_deref().unwrap()
        )
    } else {
        format!(
            "crate::domain::{}::value_objects::{}",
            snake,
            field.value_object.as_deref().unwrap()
        )
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
    /// `unique = true`: a partial unique index in `--db` projects.
    pub unique: bool,
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
}

pub fn field_ctx(field: &Field, pascal: &str, shared_vos: &[ValueObjectDefinition]) -> FieldCtx {
    let vo = field.value_object.clone().unwrap_or_default();
    let is_map = field.field_type == "HashMap<String, String>";
    let default_literal = resolve_type(&field.field_type)
        .map_or("", |m| m.default_expr)
        .to_string();
    let ref_target = field.references.clone().unwrap_or_default();
    FieldCtx {
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
                format!("body.{}.map({id}::from_uuid)", field.name)
            }
            Some(id) => format!("{id}::from_uuid(body.{})", field.name),
            None if field_needs_clone(&field.field_type) => format!("body.{}.clone()", field.name),
            None => format!("body.{}", field.name),
        },
        is_ref: is_reference(field),
        ref_target_snake: crate::generators::naming::pascal_to_snake(&ref_target),
        ref_target,
        unique: field.unique,
        seed_literal: seed_literal(field),
        field_type: field.field_type.clone(),
        is_map,
        is_array: field.field_type.starts_with("Vec<"),
        accessor: field_value_accessor(field, "entity"),
        needs_clone: field_needs_clone(&field.field_type),
        from_params: field_vo_conversion(field, "params.", pascal, shared_vos),
        default_literal,
        first_variant: field
            .enum_variants
            .as_deref()
            .and_then(<[String]>::first)
            .cloned()
            .unwrap_or_default(),
        from_row: if is_map {
            format!("row.{}.0", field.name)
        } else if let Some(id) = reference_id(field) {
            if field.field_type.starts_with("Option<") {
                format!("row.{}.map({id}::from_uuid)", field.name)
            } else {
                format!("{id}::from_uuid(row.{})", field.name)
            }
        } else {
            field_vo_conversion(field, "row.", pascal, shared_vos)
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
    }
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
        return format!("{slot}.unwrap_or_else({id}::generate)");
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
        resolve_type(ty).map_or("", |m| m.default_expr).to_string()
    })
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

/// The value-object shape of a literal around `value(primitive type)`.
fn literal_with(field: &Field, value: impl Fn(&str) -> String) -> String {
    let vo = field.value_object.as_deref().unwrap_or_default();
    if let Some(id) = reference_id(field) {
        if field.field_type.starts_with("Option<") {
            "None".to_string()
        } else {
            format!("{id}::generate()")
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
            "{vo}::new({}).expect(\"valid {vo}\")",
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
            format!("{access}.map(|v| v.value())")
        } else {
            format!("{access}.value()")
        }
    } else if is_enum_vo(field) {
        format!("{access}.as_str().to_string()")
    } else if is_option_vo(field) {
        if string_inner {
            format!("{access}.as_ref().map(|v| v.value().to_string())")
        } else {
            format!("{access}.map(|v| v.value())")
        }
    } else if is_vec_vo(field) {
        if string_inner {
            format!("{access}.iter().map(|v| v.value().to_string()).collect::<Vec<_>>()")
        } else {
            format!("{access}.iter().map(|v| v.value()).collect::<Vec<_>>()")
        }
    } else if is_value_object(field) {
        if string_inner {
            format!("{access}.value().to_string()")
        } else {
            format!("{access}.value()")
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
        format!("{conversion}.map_err(|_| {pascal}Error::Invalid{vo})?")
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
/// TypeScript type, its input, and the expressions that move it between the wire, the model, the
/// form and a screen. Expressions name helpers of `presentation/shared/forms.ts`; the `*_helpers`
/// lists say which, so a file imports exactly those.
#[derive(Debug, serde::Serialize)]
pub struct WebFieldCtx {
    /// The model property and the JSON key: poem-openapi's camelCase (`inStock`).
    pub camel: String,
    /// What a screen calls it: `In stock`.
    pub label: String,
    pub ts_type: String,
    /// `text`, `number`, `checkbox`, `datetime-local`, `textarea` or `select`.
    pub input: &'static str,
    /// A number input that takes decimals (`step="any"`).
    pub decimal: bool,
    /// The input must be filled: not an `Option`, a `Vec`, a map or a checkbox.
    pub required: bool,
    /// A `select`'s options, after an empty one labelled `placeholder`.
    pub options: Vec<WebOption>,
    pub placeholder: &'static str,
    /// The form value of a new, empty form.
    pub empty_form: &'static str,
    /// The form value read out of `{entity}` (the entity's variable) and out of `draft`.
    pub to_form: String,
    pub draft_to_form: String,
    /// The model value read out of `values` (the form).
    pub from_form: String,
    /// What a list or detail screen shows for `{entity}`, as a JSX expression.
    pub display: String,
    /// The model value read out of `dto` (the wire), and the wire value out of `draft`. An `Option`
    /// is an optional property on the wire, `null` in the model.
    pub from_wire: String,
    pub to_wire: String,
    /// A valid value, for the Object Mother: a present one for an `Option`, a non-empty one for a
    /// `Vec` or a map, so the form round-trip tests exercise the conversion.
    pub mother: String,
    pub to_form_helpers: Vec<&'static str>,
    pub from_form_helpers: Vec<&'static str>,
    pub display_helpers: Vec<&'static str>,
    /// An enum VO: the union type (`Status`), its `parseStatus` in `model.ts`.
    pub enum_type: String,
}

/// An enum value object as `model.ts` declares it: `statusValues`, `type Status`, `parseStatus`.
#[derive(Debug, serde::Serialize, PartialEq)]
pub struct WebEnumCtx {
    pub name: String,
    pub values_const: String,
    pub variants: Vec<String>,
    /// The code `parseStatus` rejects with — the backend's `{entity}.invalid_{vo}`.
    pub invalid_code: String,
}

pub fn web_enum_ctx(field: &Field, entity_snake: &str) -> Option<WebEnumCtx> {
    if !is_enum_vo(field) {
        return None;
    }
    let name = field.value_object.clone().unwrap_or_default();
    let vo_snake = crate::generators::naming::pascal_to_snake(&name);
    Some(WebEnumCtx {
        values_const: format!(
            "{}Values",
            crate::generators::naming::snake_to_camel(&vo_snake)
        ),
        variants: field.enum_variants.clone().unwrap_or_default(),
        invalid_code: format!("{entity_snake}.invalid_{vo_snake}"),
        name,
    })
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
    Checkbox,
    DateTime,
    List { numbers: bool },
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
    } else {
        match vo_inner_type(field).as_str() {
            "i64" | "f64" => WebShape::Number,
            "bool" => WebShape::Checkbox,
            "DateTime<Utc>" => WebShape::DateTime,
            _ => WebShape::Text,
        }
    }
}

/// The form text of `{owner}.{camel}`, and the helpers that expression calls.
fn web_to_form(shape: WebShape, optional: bool, access: &str) -> (String, Vec<&'static str>) {
    match shape {
        WebShape::DateTime => (
            format!("toDateTimeLocal({access})"),
            vec!["toDateTimeLocal"],
        ),
        WebShape::List { .. } => (format!("{access}.join(\", \")"), vec![]),
        WebShape::Map => (format!("formatEntries({access})"), vec!["formatEntries"]),
        _ if optional => (format!("textOf({access})"), vec!["textOf"]),
        WebShape::Number => (format!("String({access})"), vec![]),
        _ => (access.to_string(), vec![]),
    }
}

/// What a screen shows for `{owner}.{camel}`, and the helpers that expression calls.
fn web_display(shape: WebShape, optional: bool, access: &str) -> (String, Vec<&'static str>) {
    let (shown, helpers) = match shape {
        WebShape::Checkbox => (format!("{access} ? \"Yes\" : \"No\""), vec![]),
        WebShape::DateTime => (format!("formatDateTime({access})"), vec!["formatDateTime"]),
        WebShape::List { .. } => (format!("{access}.join(\", \")"), vec![]),
        WebShape::Map => (
            format!("formatEntries({access}, \", \")"),
            vec!["formatEntries"],
        ),
        _ => (access.to_string(), vec![]),
    };
    match (optional, shape) {
        (true, WebShape::Checkbox | WebShape::DateTime) => {
            (format!("{access} === null ? \"—\" : {shown}"), helpers)
        }
        (true, _) => (format!("{access} ?? \"—\""), helpers),
        _ => (shown, helpers),
    }
}

pub fn web_field_ctx(field: &Field, entity_ident: &str) -> WebFieldCtx {
    let camel = crate::generators::naming::snake_to_camel(&field.name);
    let words = crate::generators::naming::snake_to_words(&field.name);
    let optional = field.field_type.starts_with("Option<");
    let shape = web_shape(field);
    let vo = field.value_object.clone().unwrap_or_default();
    let variants = field.enum_variants.clone().unwrap_or_default();
    let value = format!("values.{camel}");

    let (base_ts, input, mother) = match shape {
        WebShape::Enum => (
            vo.clone(),
            "select",
            variants
                .first()
                .map_or("\"\"".to_string(), |v| ts_string(v)),
        ),
        WebShape::List { numbers: true } => ("number[]".into(), "text", "[42]".into()),
        WebShape::List { numbers: false } => ("string[]".into(), "text", "[\"example\"]".into()),
        WebShape::Map => (
            "Record<string, string>".into(),
            "textarea",
            "{ key: \"value\" }".into(),
        ),
        WebShape::Number => (
            "number".into(),
            "number",
            if vo_inner_type(field) == "f64" {
                "1.5"
            } else {
                "42"
            }
            .into(),
        ),
        WebShape::Checkbox => (
            "boolean".into(),
            if optional { "select" } else { "checkbox" },
            "true".into(),
        ),
        WebShape::DateTime => ("string".into(), "datetime-local", MOTHER_DATE_TIME.into()),
        WebShape::Text if vo_inner_type(field) == "Uuid" => {
            ("string".into(), "text", "crypto.randomUUID()".into())
        }
        WebShape::Text => ("string".into(), "text", "\"example\"".into()),
    };

    // An enum is parsed by its own `parse{Vo}` (below), a required text or number needs no helper.
    let from_form_helper = match shape {
        WebShape::List { numbers } => Some(if numbers { "splitNumbers" } else { "splitList" }),
        WebShape::Map => Some("parseEntries"),
        WebShape::DateTime if optional => Some("dateTimeOrNull"),
        WebShape::DateTime => Some("fromDateTimeLocal"),
        WebShape::Number if optional => Some("numberOrNull"),
        WebShape::Checkbox if optional => Some("booleanOrNull"),
        WebShape::Text if optional => Some("textOrNull"),
        _ => None,
    };
    let from_form = match (shape, from_form_helper) {
        (WebShape::Enum, _) => format!("parse{vo}({value})"),
        (_, Some(helper)) => format!("{helper}({value})"),
        (WebShape::Number, None) => format!("Number({value})"),
        _ => value.clone(),
    };

    let options = match (shape, input) {
        (WebShape::Enum, _) => variants
            .iter()
            .map(|v| WebOption {
                value: v.clone(),
                label: v.clone(),
            })
            .collect(),
        (_, "select") => vec![
            WebOption {
                value: "true".into(),
                label: "Yes".into(),
            },
            WebOption {
                value: "false".into(),
                label: "No".into(),
            },
        ],
        _ => vec![],
    };

    let (to_form, to_form_helpers) =
        web_to_form(shape, optional, &format!("{entity_ident}.{camel}"));
    let (draft_to_form, _) = web_to_form(shape, optional, &format!("draft.{camel}"));
    let (display, display_helpers) =
        web_display(shape, optional, &format!("{entity_ident}.{camel}"));

    WebFieldCtx {
        label: crate::generators::naming::capitalize(&words),
        ts_type: if optional {
            format!("{base_ts} | null")
        } else {
            base_ts
        },
        input,
        decimal: vo_inner_type(field) == "f64",
        required: !optional
            && !matches!(
                shape,
                WebShape::Checkbox | WebShape::List { .. } | WebShape::Map
            ),
        placeholder: if shape == WebShape::Enum {
            "Choose…"
        } else {
            "—"
        },
        options,
        empty_form: if input == "checkbox" { "false" } else { "\"\"" },
        to_form,
        draft_to_form,
        from_form,
        display,
        from_wire: if shape == WebShape::Enum {
            format!("parse{vo}(dto.{camel})")
        } else if optional {
            format!("dto.{camel} ?? null")
        } else {
            format!("dto.{camel}")
        },
        to_wire: if optional {
            format!("draft.{camel} ?? undefined")
        } else {
            format!("draft.{camel}")
        },
        mother,
        to_form_helpers,
        from_form_helpers: from_form_helper.into_iter().collect(),
        display_helpers,
        enum_type: if shape == WebShape::Enum {
            vo
        } else {
            String::new()
        },
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
        assert!(err.contains("unsupported field type 'CustomType'"));
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
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
            },
            Field {
                name: "price".into(),
                field_type: "i64".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
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
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
            },
            Field {
                name: "bad".into(),
                field_type: "UnknownType".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
                references: None,
            },
        ];
        let err = validate_fields(&fields).unwrap_err();
        assert!(err.contains("unsupported field type 'UnknownType'"));
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
            ("String", Some("V"), "entity.x.value().to_string()"),
            ("i64", Some("V"), "entity.x.value()"),
            ("Uuid", Some("V"), "entity.x.value()"),
            ("DateTime<Utc>", Some("V"), "entity.x.value()"),
            (
                "Option<String>",
                Some("V"),
                "entity.x.as_ref().map(|v| v.value().to_string())",
            ),
            ("Option<i64>", Some("V"), "entity.x.map(|v| v.value())"),
            ("Option<Uuid>", Some("V"), "entity.x.map(|v| v.value())"),
            (
                "Option<DateTime<Utc>>",
                Some("V"),
                "entity.x.map(|v| v.value())",
            ),
            (
                "Vec<String>",
                Some("V"),
                "entity.x.iter().map(|v| v.value().to_string()).collect::<Vec<_>>()",
            ),
            (
                "Vec<i64>",
                Some("V"),
                "entity.x.iter().map(|v| v.value()).collect::<Vec<_>>()",
            ),
            (
                "Vec<Uuid>",
                Some("V"),
                "entity.x.iter().map(|v| v.value()).collect::<Vec<_>>()",
            ),
            (
                "Vec<DateTime<Utc>>",
                Some("V"),
                "entity.x.iter().map(|v| v.value()).collect::<Vec<_>>()",
            ),
            ("String", None, "entity.x.clone()"),
            ("i64", None, "entity.x"),
            ("Uuid", None, "entity.x"),
            ("DateTime<Utc>", None, "entity.x"),
            ("Option<String>", None, "entity.x.clone()"),
            ("Option<i64>", None, "entity.x"),
            ("Vec<String>", None, "entity.x.clone()"),
            ("HashMap<String, String>", None, "entity.x.clone()"),
        ];
        for (ty, vo, expected) in cases {
            assert_eq!(
                field_value_accessor(&field(ty, *vo), "entity"),
                *expected,
                "{ty} / {vo:?}"
            );
        }
        assert_eq!(
            field_value_accessor(&enum_field(), "entity"),
            "entity.x.as_str().to_string()"
        );
        assert_eq!(field_value_accessor(&field("i64", None), "body"), "body.x");
    }

    #[test]
    fn should_describe_a_field_to_the_templates() {
        let shared = [ValueObjectDefinition {
            name: "Email".into(),
            inner_type: "String".into(),
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
            "Age::new(42).expect(\"valid Age\")"
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
                "self.x.unwrap_or_else(|| Name::new(\"example\".to_string()).expect(\"valid Name\"))",
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
        }];
        let local = |ty: &str| field_vo_conversion(&field(ty, Some("V")), "row.", "P", &[]);
        let shared_ = |ty: &str| field_vo_conversion(&field(ty, Some("V")), "row.", "P", &shared);

        assert_eq!(local("String"), "V::new(row.x)?");
        assert_eq!(local("Option<String>"), "row.x.map(V::new).transpose()?");
        assert_eq!(
            local("Vec<String>"),
            "row.x.into_iter().map(V::new).collect::<Result<Vec<_>,_>>()?"
        );
        assert_eq!(
            field_vo_conversion(&enum_field(), "row.", "P", &[]),
            "row.x.parse::<Status>()?"
        );
        assert_eq!(
            shared_("String"),
            "V::new(row.x).map_err(|_| PError::InvalidV)?"
        );
        assert_eq!(
            shared_("Option<String>"),
            "row.x.map(V::new).transpose().map_err(|_| PError::InvalidV)?"
        );
        assert_eq!(
            field_vo_conversion(&field("i64", None), "row.", "P", &[]),
            "row.x"
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
            format!("Joined::new({fixed}).expect(\"valid Joined\")")
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
        assert_eq!((ctx.ts_type.as_str(), ctx.input), ("number", "number"));
        assert!(!ctx.decimal && ctx.required);
        assert_eq!(ctx.to_form, "String(orderItem.unitPrice)");
        assert_eq!(ctx.draft_to_form, "String(draft.unitPrice)");
        assert_eq!(ctx.from_form, "Number(values.unitPrice)");
        assert_eq!(ctx.display, "orderItem.unitPrice");
        assert_eq!(
            (ctx.from_wire.as_str(), ctx.to_wire.as_str()),
            ("dto.unitPrice", "draft.unitPrice")
        );
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
            ("p.x", "values.x")
        );
        assert_eq!(flag.display, "p.x ? \"Yes\" : \"No\"");
    }

    #[test]
    fn should_keep_strings_uuids_and_value_objects_as_text() {
        let id = web_field_ctx(&field("Uuid", None), "p");
        assert_eq!((id.ts_type.as_str(), id.input), ("string", "text"));
        assert_eq!(id.mother, "crypto.randomUUID()");

        let name = web_field_ctx(&field("String", Some("Name")), "p");
        assert_eq!((name.ts_type.as_str(), name.empty_form), ("string", "\"\""));
        assert_eq!(
            (name.mother.as_str(), name.to_form.as_str()),
            ("\"example\"", "p.x")
        );
    }

    #[test]
    fn should_make_an_option_nullable_optional_on_the_wire_and_empty_text_in_the_form() {
        let views = web_field_ctx(&field("Option<i64>", None), "p");

        assert_eq!(views.ts_type, "number | null");
        assert!(!views.required);
        assert_eq!(
            (views.to_form.as_str(), views.from_form.as_str()),
            ("textOf(p.x)", "numberOrNull(values.x)")
        );
        assert_eq!(
            (views.from_wire.as_str(), views.to_wire.as_str()),
            ("dto.x ?? null", "draft.x ?? undefined")
        );
        assert_eq!(views.display, "p.x ?? \"—\"");
        assert_eq!(views.mother, "42", "present, so the round trip converts it");
        assert_eq!(views.to_form_helpers, ["textOf"]);
        assert_eq!(views.from_form_helpers, ["numberOrNull"]);

        let middle = web_field_ctx(&field("Option<String>", Some("MiddleName")), "p");
        assert_eq!(middle.from_form, "textOrNull(values.x)");
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
        assert_eq!(flag.from_form, "booleanOrNull(values.x)");
        assert_eq!(
            flag.display,
            "p.x === null ? \"—\" : p.x ? \"Yes\" : \"No\""
        );
    }

    #[test]
    fn should_edit_a_date_time_in_local_time_and_keep_iso_8601_in_the_model() {
        let at = web_field_ctx(&field("DateTime<Utc>", None), "p");
        assert_eq!(
            (at.ts_type.as_str(), at.input),
            ("string", "datetime-local")
        );
        assert!(at.required);
        assert_eq!(
            (at.to_form.as_str(), at.from_form.as_str()),
            ("toDateTimeLocal(p.x)", "fromDateTimeLocal(values.x)")
        );
        assert_eq!(at.display, "formatDateTime(p.x)");
        assert_eq!(at.display_helpers, ["formatDateTime"]);
        assert_eq!(at.mother, MOTHER_DATE_TIME);

        let expires = web_field_ctx(&field("Option<DateTime<Utc>>", None), "p");
        assert_eq!(expires.from_form, "dateTimeOrNull(values.x)");
        assert_eq!(
            expires.display,
            "p.x === null ? \"—\" : formatDateTime(p.x)"
        );
    }

    #[test]
    fn should_write_a_list_comma_separated_and_a_map_one_entry_per_line() {
        let tags = web_field_ctx(&field("Vec<String>", Some("Tag")), "p");
        assert_eq!((tags.ts_type.as_str(), tags.input), ("string[]", "text"));
        assert!(!tags.required);
        assert_eq!(
            (tags.to_form.as_str(), tags.from_form.as_str()),
            ("p.x.join(\", \")", "splitList(values.x)")
        );
        assert_eq!(tags.mother, "[\"example\"]");

        let scores = web_field_ctx(&field("Vec<i64>", None), "p");
        assert_eq!(
            (scores.ts_type.as_str(), scores.from_form.as_str()),
            ("number[]", "splitNumbers(values.x)")
        );

        let meta = web_field_ctx(&field("HashMap<String, String>", None), "p");
        assert_eq!(
            (meta.ts_type.as_str(), meta.input),
            ("Record<string, string>", "textarea")
        );
        assert_eq!(
            (meta.to_form.as_str(), meta.from_form.as_str()),
            ("formatEntries(p.x)", "parseEntries(values.x)")
        );
        assert_eq!(meta.display, "formatEntries(p.x, \", \")");
    }

    #[test]
    fn should_narrow_an_enum_to_its_union_on_the_way_in() {
        let status = enum_field();
        let ctx = web_field_ctx(&status, "p");

        assert_eq!(
            (ctx.ts_type.as_str(), ctx.input, ctx.enum_type.as_str()),
            ("Status", "select", "Status")
        );
        assert!(ctx.required);
        assert_eq!(ctx.from_wire, format!("parseStatus(dto.{})", ctx.camel));
        assert_eq!(ctx.from_form, format!("parseStatus(values.{})", ctx.camel));
        assert_eq!(
            ctx.options.first().map(|o| o.value.as_str()),
            Some("Active")
        );
        assert_eq!(ctx.mother, "\"Active\"");

        let declared = web_enum_ctx(&status, "order_item").unwrap();
        assert_eq!(declared.values_const, "statusValues");
        assert_eq!(declared.invalid_code, "order_item.invalid_status");
        assert!(web_enum_ctx(&field("String", Some("Name")), "x").is_none());
    }
}
