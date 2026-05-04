use crate::puerto_toml::Field;

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
            },
            Field {
                name: "price".into(),
                field_type: "i64".into(),
                unique: false,
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
            },
            Field {
                name: "bad".into(),
                field_type: "UnknownType".into(),
                unique: false,
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
            },
            Field {
                name: "other_id".into(),
                field_type: "Uuid".into(),
                unique: false,
            },
        ];
        let imports = collect_imports(&fields);
        assert_eq!(imports.len(), 1);
    }
}
