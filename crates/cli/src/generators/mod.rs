pub mod application;
pub mod auth;
pub mod bootstrap;
pub mod conflict;
pub mod consistency;
pub mod domain;
pub mod events;
pub mod format;
pub mod infrastructure;
pub mod mail;
pub mod migration;
pub mod naming;
pub mod presentation;
pub mod project;
pub mod render;
pub mod scaffold;
pub mod types;
pub mod use_case;
pub mod web;
pub mod worker;

use crate::puerto_toml::Field;

pub fn effective_fields(fields: &[Field]) -> Vec<Field> {
    if fields.is_empty() {
        vec![Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            value_object: None,
            value_object_kind: None,
            enum_variants: None,
        }]
    } else {
        fields.to_vec()
    }
}
