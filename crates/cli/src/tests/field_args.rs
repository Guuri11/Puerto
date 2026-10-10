use super::*;

// ── Phase 7: CLI field arguments ──────────────────────────────────────────────

#[test]
fn parse_field_arg_basic() {
    let field = puerto_toml::parse_field_arg("title:String").unwrap();
    assert_eq!(field.name, "title");
    assert_eq!(field.field_type, "String");
    assert!(!field.unique);
}

#[test]
fn parse_field_arg_with_unique() {
    let field = puerto_toml::parse_field_arg("sku:String!").unwrap();
    assert_eq!(field.name, "sku");
    assert_eq!(field.field_type, "String");
    assert!(field.unique);
}

#[test]
fn parse_field_arg_option_primitive() {
    let field = puerto_toml::parse_field_arg("description:opt:String").unwrap();
    assert_eq!(field.name, "description");
    assert_eq!(field.field_type, "Option<String>");
    assert!(!field.unique);
    assert!(field.value_object.is_none());
}

#[test]
fn parse_field_arg_vec_primitive() {
    let field = puerto_toml::parse_field_arg("tags:vec:String").unwrap();
    assert_eq!(field.name, "tags");
    assert_eq!(field.field_type, "Vec<String>");
    assert!(field.value_object.is_none());
}

#[test]
fn parse_field_arg_map_shorthand() {
    let field = puerto_toml::parse_field_arg("metadata:map").unwrap();
    assert_eq!(field.name, "metadata");
    assert_eq!(field.field_type, "HashMap<String, String>");
}

#[test]
fn parse_field_arg_datetime_shorthand() {
    // Not `created_at` — that is a system field every entity already has.
    let field = puerto_toml::parse_field_arg("published_at:DateTime").unwrap();
    assert_eq!(field.name, "published_at");
    assert_eq!(field.field_type, "DateTime<Utc>");
}

#[test]
fn parse_field_arg_string_vo() {
    let field = puerto_toml::parse_field_arg("name:Name:String").unwrap();
    assert_eq!(field.name, "name");
    assert_eq!(field.field_type, "String");
    assert_eq!(field.value_object, Some("Name".to_string()));
    assert!(!field.unique);
}

#[test]
fn parse_field_arg_numeric_vo() {
    let field = puerto_toml::parse_field_arg("age:Age:i64").unwrap();
    assert_eq!(field.name, "age");
    assert_eq!(field.field_type, "i64");
    assert_eq!(field.value_object, Some("Age".to_string()));
}

#[test]
fn parse_field_arg_float_vo() {
    let field = puerto_toml::parse_field_arg("height:Height:f64").unwrap();
    assert_eq!(field.name, "height");
    assert_eq!(field.field_type, "f64");
    assert_eq!(field.value_object, Some("Height".to_string()));
}

#[test]
fn parse_field_arg_unique_vo() {
    let field = puerto_toml::parse_field_arg("sku:Sku:String!").unwrap();
    assert_eq!(field.name, "sku");
    assert_eq!(field.field_type, "String");
    assert_eq!(field.value_object, Some("Sku".to_string()));
    assert!(field.unique);
}

#[test]
fn parse_field_arg_option_vo() {
    let field = puerto_toml::parse_field_arg("middle_name:MiddleName:opt:String").unwrap();
    assert_eq!(field.name, "middle_name");
    assert_eq!(field.field_type, "Option<String>");
    assert_eq!(field.value_object, Some("MiddleName".to_string()));
    assert!(field.value_object_kind.is_none());
}

#[test]
fn parse_field_arg_vec_vo() {
    let field = puerto_toml::parse_field_arg("tags:Tag:vec:String").unwrap();
    assert_eq!(field.name, "tags");
    assert_eq!(field.field_type, "Vec<String>");
    assert_eq!(field.value_object, Some("Tag".to_string()));
}

#[test]
fn parse_field_arg_enum_vo() {
    let field =
        puerto_toml::parse_field_arg("status:Status:enum:Pending/Confirmed/Cancelled").unwrap();
    assert_eq!(field.name, "status");
    assert_eq!(field.field_type, "String");
    assert_eq!(field.value_object, Some("Status".to_string()));
    assert_eq!(field.value_object_kind, Some("enum".to_string()));
    assert_eq!(
        field.enum_variants,
        Some(vec![
            "Pending".to_string(),
            "Confirmed".to_string(),
            "Cancelled".to_string()
        ])
    );
}

#[test]
fn parse_field_arg_vo_with_datetime_shorthand() {
    let field = puerto_toml::parse_field_arg("occurred:OccurredAt:DateTime").unwrap();
    assert_eq!(field.field_type, "DateTime<Utc>");
    assert_eq!(field.value_object, Some("OccurredAt".to_string()));
}

#[test]
fn parse_field_arg_option_vo_with_datetime_shorthand() {
    let field = puerto_toml::parse_field_arg("scheduled:ScheduledAt:opt:DateTime").unwrap();
    assert_eq!(field.field_type, "Option<DateTime<Utc>>");
    assert_eq!(field.value_object, Some("ScheduledAt".to_string()));
}

#[test]
fn parse_field_arg_rejects_unknown_keyword() {
    let err = puerto_toml::parse_field_arg("tags:unknown:String").unwrap_err();
    assert!(err.contains("not a known keyword") || err.contains("PascalCase VO name"));
}

#[test]
fn parse_field_arg_rejects_empty_enum_variants() {
    let err = puerto_toml::parse_field_arg("status:Status:enum:").unwrap_err();
    assert!(err.contains("Enum variants cannot be empty") || err.contains("PascalCase"));
}

#[test]
fn parse_field_arg_rejects_lowercase_vo_name() {
    let err = puerto_toml::parse_field_arg("name:myName:String").unwrap_err();
    assert!(err.contains("PascalCase") || err.contains("not a known keyword"));
}

#[test]
fn parse_field_arg_uuid() {
    let field = puerto_toml::parse_field_arg("category_id:Uuid").unwrap();
    assert_eq!(field.name, "category_id");
    assert_eq!(field.field_type, "Uuid");
}

#[test]
fn parse_field_arg_uuid_unique() {
    // Not `id` — that is a system field every entity already has.
    let field = puerto_toml::parse_field_arg("external_id:Uuid!").unwrap();
    assert_eq!(field.name, "external_id");
    assert_eq!(field.field_type, "Uuid");
    assert!(field.unique);
}

#[test]
fn parse_field_arg_rejects_no_colon() {
    let err = puerto_toml::parse_field_arg("titleString").unwrap_err();
    assert!(err.contains("invalid field argument"));
    assert!(err.contains("Expected format: name:Type"));
}

#[test]
fn parse_field_arg_rejects_empty_type() {
    let err = puerto_toml::parse_field_arg("title:").unwrap_err();
    assert!(err.contains("Type cannot be empty"));
}

#[test]
fn parse_field_arg_rejects_uppercase_name() {
    let err = puerto_toml::parse_field_arg("Title:String").unwrap_err();
    assert!(err.contains("invalid field name"));
    assert!(err.contains("snake_case"));
}

#[test]
fn parse_field_arg_rejects_name_starting_with_digit() {
    let err = puerto_toml::parse_field_arg("1title:String").unwrap_err();
    assert!(err.contains("invalid field name"));
}

#[test]
fn parse_field_arg_rejects_leading_underscore_name() {
    // A leading `_` was accepted by the predicate while its own error message and the docs both
    // said "snake_case, not starting with a digit" — and `_private` reads as "deliberately
    // unused" to every Rust reader.
    assert!(puerto_toml::parse_field_arg("_private:String").is_err());
}

#[test]
fn parse_field_arg_accepts_name_with_digits() {
    let field = puerto_toml::parse_field_arg("address2:String").unwrap();
    assert_eq!(field.name, "address2");
}

#[test]
fn validate_cli_fields_against_type_registry() {
    let fields = vec![
        puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        puerto_toml::Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    assert!(crate::generators::types::validate_fields(&fields).is_ok());
}

#[test]
fn validate_cli_fields_rejects_unknown_type() {
    let fields = vec![puerto_toml::Field {
        name: "title".into(),
        field_type: "UnknownType".into(),
        unique: false,
        ..Default::default()
    }];
    assert!(crate::generators::types::validate_fields(&fields).is_err());
}
