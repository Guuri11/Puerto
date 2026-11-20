pub mod application;
pub mod auth;
pub mod blocks;
pub mod bootstrap;
pub mod conflict;
pub mod consistency;
pub mod crates;
pub mod domain;
pub mod events;
pub mod format;
pub mod infrastructure;
pub mod layout;
pub mod mail;
pub mod merge;
pub mod migration;
pub mod naming;
pub mod ports;
pub mod presentation;
pub mod project;
pub mod render;
pub mod scaffold;
pub mod transition;
pub mod types;
pub mod use_case;
pub mod web;
pub mod worker;

use crate::puerto_toml::Field;

pub fn effective_fields(fields: &[Field]) -> Vec<Field> {
    if fields.is_empty() {
        vec![Field {
            name: "nombre".into(),
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
            owner: false,
        }]
    } else {
        fields.to_vec()
    }
}
