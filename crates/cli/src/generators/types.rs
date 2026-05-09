use crate::puerto_toml::{Field, ValueObjectDefinition};

#[derive(Debug)]
#[allow(dead_code)]
pub struct TypeMapping {
    pub rust_type: &'static str,
    pub sql_type: &'static str,
    pub sql_nullable: bool,
    pub openapi_type: &'static str,
    pub openapi_format: Option<&'static str>,
    pub default_expr: &'static str,
    pub needs_import: Option<&'static str>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn validate_fields(fields: &[Field]) -> Result<(), String> {
    for field in fields {
        resolve_type(&field.field_type)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn collect_imports(fields: &[Field]) -> Vec<&'static str> {
    let mut imports: Vec<&'static str> = vec![];
    for field in fields {
        if let Some(imp) = resolve_type(&field.field_type)
            .ok()
            .and_then(|m| m.needs_import)
        {
            if !imports.contains(&imp) {
                imports.push(imp);
            }
        }
    }
    imports
}

#[allow(dead_code)]
pub fn is_value_object(field: &Field) -> bool {
    field.value_object.is_some()
}

#[allow(dead_code)]
pub fn is_option_vo(field: &Field) -> bool {
    field.value_object.is_some() && field.field_type.starts_with("Option<")
}

#[allow(dead_code)]
pub fn is_vec_vo(field: &Field) -> bool {
    field.value_object.is_some() && field.field_type.starts_with("Vec<")
}

#[allow(dead_code)]
pub fn is_enum_vo(field: &Field) -> bool {
    field.value_object_kind.as_deref() == Some("enum")
}

#[allow(dead_code)]
pub fn is_shared_vo(field: &Field, shared_vos: &[ValueObjectDefinition]) -> bool {
    field
        .value_object
        .as_deref()
        .is_some_and(|vo| shared_vos.iter().any(|d| d.name == vo))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn vo_error_import_path(_shared_vos: &[ValueObjectDefinition]) -> String {
    "crate::domain::shared::errors".to_string()
}

#[allow(dead_code)]
pub fn vo_name(field: &Field) -> Option<&str> {
    field.value_object.as_deref()
}

#[allow(dead_code)]
pub fn vo_inner_type(field: &Field) -> String {
    if field.field_type.starts_with("Option<") {
        field.field_type[7..field.field_type.len() - 1].to_string()
    } else if field.field_type.starts_with("Vec<") {
        field.field_type[4..field.field_type.len() - 1].to_string()
    } else {
        field.field_type.clone()
    }
}

#[allow(dead_code)]
pub fn field_rust_type(field: &Field) -> String {
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

#[allow(dead_code)]
pub fn field_value_accessor(field: &Field, prefix: &str) -> String {
    if is_enum_vo(field) {
        format!("{}.{}.as_str().to_string()", prefix, field.name)
    } else if is_option_vo(field) {
        let inner = vo_inner_type(field);
        if inner == "String" {
            format!(
                "{}.{}.as_ref().map(|v| v.value().to_string())",
                prefix, field.name
            )
        } else {
            format!("{}.{}.map(|v| v.value())", prefix, field.name)
        }
    } else if is_vec_vo(field) {
        let inner = vo_inner_type(field);
        if inner == "String" {
            format!(
                "{}.{}.iter().map(|v| v.value().to_string()).collect::<Vec<_>>()",
                prefix, field.name
            )
        } else {
            format!(
                "{}.{}.iter().map(|v| v.value()).collect::<Vec<_>>()",
                prefix, field.name
            )
        }
    } else if is_value_object(field) {
        match field.field_type.as_str() {
            "String" => format!("{}.{}.value().to_string()", prefix, field.name),
            _ => format!("{}.{}.value()", prefix, field.name),
        }
    } else if field_needs_clone(&field.field_type) {
        format!("{}.{}.clone()", prefix, field.name)
    } else {
        format!("{}.{}", prefix, field.name)
    }
}

#[allow(dead_code)]
pub fn field_vo_constructor(
    field: &Field,
    param_prefix: &str,
    pascal: &str,
    shared_vos: &[ValueObjectDefinition],
) -> String {
    if field.value_object.is_none() && !is_enum_vo(field) {
        return String::new();
    }
    let is_shared = is_shared_vo(field, shared_vos);
    let error_variant = format!("Invalid{}", field.value_object.as_deref().unwrap_or(""));
    if is_enum_vo(field) {
        let vo = field.value_object.as_deref().unwrap();
        if is_shared {
            format!(
                "let {} = {}::from_str(&{}{}).map_err(|_| {}Error::{})?;",
                field.name, vo, param_prefix, field.name, pascal, error_variant
            )
        } else {
            format!(
                "let {} = {}::from_str(&{}{})?;",
                field.name, vo, param_prefix, field.name
            )
        }
    } else if is_option_vo(field) {
        let vo = field.value_object.as_deref().unwrap();
        if is_shared {
            format!(
                "let {} = {}{}.map({}::new).transpose().map_err(|_| {}Error::{})?;",
                field.name, param_prefix, field.name, vo, pascal, error_variant
            )
        } else {
            format!(
                "let {} = {}{}.map({}::new).transpose()?;",
                field.name, param_prefix, field.name, vo
            )
        }
    } else if is_vec_vo(field) {
        let vo = field.value_object.as_deref().unwrap();
        if is_shared {
            format!(
                "let {}: Vec<{}> = {}{}.into_iter().map({}::new).collect::<Result<Vec<_>,_>>().map_err(|_| {}Error::{})?;",
                field.name, vo, param_prefix, field.name, vo, pascal, error_variant
            )
        } else {
            format!(
                "let {}: Vec<{}> = {}{}.into_iter().map({}::new).collect::<Result<Vec<_>,_>>()?;",
                field.name, vo, param_prefix, field.name, vo
            )
        }
    } else if is_value_object(field) {
        let vo = field.value_object.as_deref().unwrap();
        if is_shared {
            format!(
                "let {} = {}::new({}{}).map_err(|_| {}Error::{})?;",
                field.name, vo, param_prefix, field.name, pascal, error_variant
            )
        } else {
            format!(
                "let {} = {}::new({}{})?;",
                field.name, vo, param_prefix, field.name
            )
        }
    } else {
        String::new()
    }
}

fn field_needs_clone(field_type: &str) -> bool {
    matches!(
        field_type,
        "String" | "Option<String>" | "Vec<String>" | "Vec<i64>" | "HashMap<String, String>"
    )
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
            },
            Field {
                name: "price".into(),
                field_type: "i64".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
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
            },
            Field {
                name: "bad".into(),
                field_type: "UnknownType".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
            },
        ];
        let err = validate_fields(&fields).unwrap_err();
        assert!(err.contains("unsupported field type 'UnknownType'"));
    }

    #[test]
    fn collect_imports_deduplicates() {
        use crate::puerto_toml::Field;
        let fields = vec![
            Field {
                name: "id".into(),
                field_type: "Uuid".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
            },
            Field {
                name: "other_id".into(),
                field_type: "Uuid".into(),
                unique: false,
                value_object: None,
                value_object_kind: None,
                enum_variants: None,
            },
        ];
        let imports = collect_imports(&fields);
        assert_eq!(imports.len(), 1);
    }
}
