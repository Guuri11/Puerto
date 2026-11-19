use super::*;

// ── Phase 3.10: Shared VO tests ───────────────────────────────────────────────

#[test]
fn is_shared_vo_detection() {
    use crate::generators::types::is_shared_vo;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let email_field = Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    };
    let local_field = Field {
        name: "name".into(),
        field_type: "String".into(),
        value_object: Some("Name".to_string()),
        ..Default::default()
    };
    let primitive_field = Field {
        name: "age".into(),
        field_type: "i64".into(),
        ..Default::default()
    };

    assert!(is_shared_vo(&email_field, &shared_vos));
    assert!(!is_shared_vo(&local_field, &shared_vos));
    assert!(!is_shared_vo(&primitive_field, &shared_vos));
}

#[test]
fn generate_shared_value_objects_string() {
    use crate::generators::domain::generate_shared_value_objects;
    use crate::puerto_toml::ValueObjectDefinition;

    let vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let result = generate_shared_value_objects(&vos).unwrap();
    assert!(result.contains("pub struct Email"));
    assert!(result.contains("fn new(valor: String)"));
    assert!(result.contains("fn valor(&self) -> &str"));
    assert!(result.contains("ErrorEmail::Invalido"));
    assert!(result.contains("recortado.is_empty()"));
}

#[test]
fn generate_shared_value_objects_numeric() {
    use crate::generators::domain::generate_shared_value_objects;
    use crate::puerto_toml::ValueObjectDefinition;

    let vos = vec![ValueObjectDefinition {
        name: "Amount".to_string(),
        inner_type: "i64".to_string(),
        ..Default::default()
    }];

    let result = generate_shared_value_objects(&vos).unwrap();
    assert!(result.contains("pub struct Amount(i64)"));
    assert!(result.contains("fn new(valor: i64)"));
    assert!(result.contains("fn valor(&self) -> i64"));
}

#[test]
fn generate_shared_errors_combined_output() {
    use crate::generators::domain::generate_shared_errors_combined;
    use crate::puerto_toml::ValueObjectDefinition;

    let vos = vec![
        ValueObjectDefinition {
            name: "Email".to_string(),
            inner_type: "String".to_string(),
            ..Default::default()
        },
        ValueObjectDefinition {
            name: "Money".to_string(),
            inner_type: "i64".to_string(),
            ..Default::default()
        },
    ];

    let result = generate_shared_errors_combined(&vos).unwrap();
    assert!(result.contains("pub enum ErrorEmail"));
    assert!(result.contains("pub enum ErrorMoney"));
    assert!(result.contains("nucleo.email_invalido"));
    assert!(result.contains("nucleo.money_invalido"));
}

#[test]
fn generate_model_with_shared_vo_imports() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let fields = vec![Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    }];

    let result = generate_model("User", "user", &fields, &shared_vos, false).unwrap();
    assert!(result.contains("use nucleo::valores::Email"));
    assert!(result.contains("pub email: Email"));
}

#[test]
fn generate_model_local_vo_uses_local_import() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let fields = vec![Field {
        name: "name".into(),
        field_type: "String".into(),
        value_object: Some("Name".to_string()),
        ..Default::default()
    }];

    let result = generate_model("User", "user", &fields, &shared_vos, false).unwrap();
    assert!(result.contains("use crate::valores::Name"));
    assert!(!result.contains("shared::value_objects::Name"));
}

#[test]
fn generate_create_use_case_shared_vo() {
    use crate::generators::application::generate_create_use_case_impl;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let fields = vec![Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    }];

    let result = generate_create_use_case_impl("User", "user", &fields, &shared_vos).unwrap();
    assert!(result.contains("use nucleo::valores::Email"));
    assert!(result.contains(".map_err(|_| ErrorUser::EmailInvalido)?"));
}

#[test]
fn generate_infra_entity_shared_vo_import_path() {
    use crate::generators::infrastructure::generate_infra_entity;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let fields = vec![Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    }];

    let result = generate_infra_entity("User", "user", &fields, &shared_vos).unwrap();
    assert!(result.contains("use nucleo::valores::Email"));
    assert!(!result.contains("use dominio_user::valores::Email"));
}

#[test]
fn generate_infra_entity_shared_vo_try_from_map_err() {
    use crate::generators::infrastructure::generate_infra_entity;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];

    let fields = vec![Field {
        name: "email".into(),
        field_type: "String".into(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    }];

    let result = generate_infra_entity("User", "user", &fields, &shared_vos).unwrap();
    assert!(result.contains(".map_err(|_| ErrorUser::EmailInvalido)?"));
}

#[test]
fn generate_infra_entity_local_vo_uses_plain_question_mark() {
    use crate::generators::infrastructure::generate_infra_entity;
    use crate::puerto_toml::{Field, ValueObjectDefinition};

    let shared_vos: Vec<ValueObjectDefinition> = vec![];

    let fields = vec![Field {
        name: "name".into(),
        field_type: "String".into(),
        value_object: Some("Name".to_string()),
        ..Default::default()
    }];

    let result = generate_infra_entity("Producto", "producto", &fields, &shared_vos).unwrap();
    assert!(result.contains("Name::new(fila.name)?"));
    assert!(!result.contains(".map_err"));
}

#[test]
fn scaffold_with_shared_vo_writes_shared_files() {
    let dir = temp_dir("scaffold_shared_vo");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "my-app"

[[objeto_valor]]
nombre = "Email"
tipo = "String"
"#,
    )
    .unwrap();

    scaffold::run(
        "User",
        &dir,
        false,
        &[puerto_toml::Field {
            name: "email".into(),
            field_type: "String".into(),
            value_object: Some("Email".to_string()),
            ..Default::default()
        }],
        &[crate::puerto_toml::ValueObjectDefinition {
            name: "Email".to_string(),
            inner_type: "String".to_string(),
            ..Default::default()
        }],
    )
    .unwrap();

    assert!(dir.join("nucleo/src/valores.rs").exists());
    assert!(dir.join("nucleo/src/errores.rs").exists());
    assert!(!dir.join("contextos/shared/dominio/src/mod.rs").exists());
    let shared = fs::read_to_string(dir.join("nucleo/src/lib.rs")).unwrap();
    assert!(shared.contains("pub mod valores;"), "{shared}");

    cleanup(&dir);
}

#[test]
fn scaffold_with_all_shared_vos_does_not_patch_local_value_objects_mod() {
    let dir = temp_dir("scaffold_shared_vo_no_local");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

    fs::write(dir.join("puerto.toml"), "[proyecto]\nnombre = \"my-app\"\n").unwrap();

    scaffold::run(
        "Cliente",
        &dir,
        false,
        &[puerto_toml::Field {
            name: "email".into(),
            field_type: "String".into(),
            value_object: Some("Email".to_string()),
            ..Default::default()
        }],
        &[ValueObjectDefinition {
            name: "Email".to_string(),
            inner_type: "String".to_string(),
            ..Default::default()
        }],
    )
    .unwrap();

    // No local value_objects.rs should exist — all VOs are shared
    assert!(
        !dir.join("contextos/cliente/dominio/src/valores.rs")
            .exists(),
        "local value_objects.rs must not be created when all VOs are shared"
    );
    // Shared files must still be written
    assert!(dir.join("nucleo/src/valores.rs").exists());

    cleanup(&dir);
}

// ── puerto generar objeto-valor ─────────────────────────────────────────────

#[test]
fn value_object_adds_entry_to_puerto_toml() {
    let dir = temp_dir("vo_add_entry");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), "[proyecto]\nnombre = \"my-app\"\n").unwrap();

    puerto_toml::add_value_object(&dir, "Email", "String").unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.value_object.len(), 1);
    assert_eq!(config.value_object[0].name, "Email");
    assert_eq!(config.value_object[0].inner_type, "String");

    cleanup(&dir);
}

#[test]
fn value_object_add_is_idempotent() {
    let dir = temp_dir("vo_idempotent");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), "[proyecto]\nnombre = \"my-app\"\n").unwrap();

    puerto_toml::add_value_object(&dir, "Email", "String").unwrap();
    puerto_toml::add_value_object(&dir, "Email", "String").unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.value_object.len(), 1, "duplicate entry added");

    cleanup(&dir);
}

#[test]
fn value_object_multiple_entries_preserved() {
    let dir = temp_dir("vo_multiple");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), "[proyecto]\nnombre = \"my-app\"\n").unwrap();

    puerto_toml::add_value_object(&dir, "Email", "String").unwrap();
    puerto_toml::add_value_object(&dir, "Money", "i64").unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.value_object.len(), 2);
    assert_eq!(config.value_object[0].name, "Email");
    assert_eq!(config.value_object[1].name, "Money");

    cleanup(&dir);
}

// ── Shared VO type inference ───────────────────────────────────────────────

#[test]
fn infer_shared_vo_string_type_from_name() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "email".to_string(),
        field_type: "Email".to_string(),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "String");
    assert_eq!(result[0].value_object, Some("Email".to_string()));
    assert!(result[0].value_object_kind.is_none());
    assert!(!result[0].unique);
}

#[test]
fn infer_shared_vo_numeric_type_from_name() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Money".to_string(),
        inner_type: "i64".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "price".to_string(),
        field_type: "Money".to_string(),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "i64");
    assert_eq!(result[0].value_object, Some("Money".to_string()));
}

#[test]
fn infer_shared_vo_preserves_unique_flag() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "email".to_string(),
        field_type: "Email".to_string(),
        unique: true,
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "String");
    assert_eq!(result[0].value_object, Some("Email".to_string()));
    assert!(result[0].unique);
}

#[test]
fn infer_shared_vo_leaves_explicit_vo_field_unchanged() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "email".to_string(),
        field_type: "String".to_string(),
        value_object: Some("Email".to_string()),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "String");
    assert_eq!(result[0].value_object, Some("Email".to_string()));
}

#[test]
fn infer_shared_vo_leaves_primitive_field_unchanged() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "title".to_string(),
        field_type: "String".to_string(),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "String");
    assert!(result[0].value_object.is_none());
}

#[test]
fn infer_shared_vo_with_empty_registry_is_identity() {
    let fields = vec![puerto_toml::Field {
        name: "email".to_string(),
        field_type: "Email".to_string(),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &[]);
    assert_eq!(result[0].field_type, "Email");
    assert!(result[0].value_object.is_none());
}

#[test]
fn infer_shared_vo_only_matches_registered_vo_names() {
    let shared_vos = vec![ValueObjectDefinition {
        name: "Email".to_string(),
        inner_type: "String".to_string(),
        ..Default::default()
    }];
    let fields = vec![puerto_toml::Field {
        name: "phone".to_string(),
        field_type: "Phone".to_string(),
        ..Default::default()
    }];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    // Phone not in shared VOs — field_type left as-is (will fail type validation)
    assert_eq!(result[0].field_type, "Phone");
    assert!(result[0].value_object.is_none());
}

#[test]
fn infer_shared_vo_handles_multiple_fields() {
    let shared_vos = vec![
        ValueObjectDefinition {
            name: "Email".to_string(),
            inner_type: "String".to_string(),
            ..Default::default()
        },
        ValueObjectDefinition {
            name: "Money".to_string(),
            inner_type: "i64".to_string(),
            ..Default::default()
        },
    ];
    let fields = vec![
        puerto_toml::Field {
            name: "email".to_string(),
            field_type: "Email".to_string(),
            ..Default::default()
        },
        puerto_toml::Field {
            name: "price".to_string(),
            field_type: "Money".to_string(),
            ..Default::default()
        },
        puerto_toml::Field {
            name: "active".to_string(),
            field_type: "bool".to_string(),
            ..Default::default()
        },
    ];
    let result = puerto_toml::apply_shared_vo_inference(fields, &shared_vos);
    assert_eq!(result[0].field_type, "String");
    assert_eq!(result[0].value_object, Some("Email".to_string()));
    assert_eq!(result[1].field_type, "i64");
    assert_eq!(result[1].value_object, Some("Money".to_string()));
    assert_eq!(result[2].field_type, "bool");
    assert!(result[2].value_object.is_none());
}

// ── T-13: review fixes — value objects, names, migrations ─────────────────

#[test]
fn value_object_command_rejects_invalid_name_type_and_duplicates() {
    let dir = temp_dir("vo_cmd_validation");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);

    assert!(scaffold::run_value_object("email", "String", None, &dir).is_err());
    assert!(scaffold::run_value_object("Email", "Strin", None, &dir).is_err());
    scaffold::run_value_object("Email", "String", None, &dir).unwrap();
    let dup = scaffold::run_value_object("Email", "String", None, &dir);

    assert!(dup.is_err());
    assert!(dup.unwrap_err().to_string().contains("ya está declarad"));
    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.value_object.len(), 1);

    cleanup(&dir);
}

#[test]
fn value_object_command_expands_datetime_shorthand() {
    let dir = temp_dir("vo_cmd_datetime");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);

    scaffold::run_value_object("When", "DateTime", None, &dir).unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.value_object[0].inner_type, "DateTime<Utc>");

    cleanup(&dir);
}

#[test]
fn should_import_uuid_and_chrono_per_shared_value_object_that_wraps_them() {
    // No matrix scenario declares a shared Uuid / DateTime VO, so no snapshot locks this branch.
    let vos = [
        ValueObjectDefinition {
            name: "Ref".into(),
            inner_type: "Uuid".into(),
            ..Default::default()
        },
        ValueObjectDefinition {
            name: "Since".into(),
            inner_type: "DateTime<Utc>".into(),
            ..Default::default()
        },
    ];

    let out = crate::generators::domain::generate_shared_value_objects(&vos).unwrap();

    assert_eq!(
        out,
        "use crate::errores::ErrorRef;\nuse uuid::Uuid;\n\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct Ref(Uuid);\n\nimpl Ref {\n    pub fn new(valor: Uuid) -> Result<Self, ErrorRef> {\n        Ok(Self(valor))\n    }\n\n    pub fn valor(&self) -> Uuid {\n        self.0\n    }\n}\n\nuse crate::errores::ErrorSince;\nuse chrono::{DateTime, Utc};\n\n#[derive(Debug, Clone, PartialEq)]\npub struct Since(DateTime<Utc>);\n\nimpl Since {\n    pub fn new(valor: DateTime<Utc>) -> Result<Self, ErrorSince> {\n        Ok(Self(valor))\n    }\n\n    pub fn valor(&self) -> DateTime<Utc> {\n        self.0\n    }\n}\n"
    );
}

// ── 27.15: an example per shared value object ──────────────────────────────

fn manifest_dir(name: &str, manifest: &str) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), manifest).unwrap();
    dir
}

#[test]
fn value_object_records_its_example_in_puerto_toml() {
    let dir = manifest_dir("vo_example", "[proyecto]\nnombre = \"my-app\"\n");

    scaffold::run_value_object("Email", "String", Some("ana@ejemplo.es"), &dir).unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(
        config.value_object[0].example.as_deref(),
        Some("ana@ejemplo.es")
    );
    assert!(
        fs::read_to_string(dir.join("puerto.toml"))
            .unwrap()
            .contains("ejemplo = \"ana@ejemplo.es\"")
    );
    cleanup(&dir);
}

#[test]
fn value_object_refuses_an_example_that_is_not_of_its_type_and_writes_nothing() {
    let dir = manifest_dir("vo_example_bad", "[proyecto]\nnombre = \"my-app\"\n");

    for (name, ty, example) in [
        ("Edad", "i64", "doce"),
        ("Ratio", "f64", "mucho"),
        ("Activo", "bool", "si"),
        ("Nombre", "String", " "),
        ("Ref", "Uuid", "2b1f…"),
    ] {
        let err = scaffold::run_value_object(name, ty, Some(example), &dir).unwrap_err();
        assert!(err.to_string().contains("ejemplo"), "{name}: {err}");
    }
    assert!(puerto_toml::read(&dir).unwrap().value_object.is_empty());
    cleanup(&dir);
}

#[test]
fn validate_rejects_an_example_that_is_not_of_its_type() {
    let dir = manifest_dir(
        "vo_example_validate",
        "[proyecto]\nnombre = \"my-app\"\n\n[[objeto_valor]]\nnombre = \"Edad\"\ntipo = \"i64\"\nejemplo = \"doce\"\n",
    );

    let err = validate::run_validate(&dir).unwrap_err().to_string();

    assert!(err.contains("Edad") && err.contains("ejemplo"), "{err}");
    cleanup(&dir);
}

#[test]
fn a_field_of_a_shared_value_object_takes_its_example_everywhere_a_test_needs_a_value() {
    let dir = temp_dir("vo_example_everywhere");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = crate::commands::new::new_project(
        Some("vo-example".into()),
        crate::commands::new::NewOptions {
            db: true,
            no_portada: true,
            ..Default::default()
        },
        Some(dir.clone()),
    )
    .unwrap();
    scaffold::run_value_object("Email", "String", Some("ana@ejemplo.es"), &output).unwrap();
    scaffold::run_value_object("Plazas", "i64", Some("12"), &output).unwrap();
    let shared = puerto_toml::read(&output).unwrap().value_object;
    let fields = puerto_toml::apply_shared_vo_inference(
        vec![
            puerto_toml::parse_field_arg("email:Email").unwrap(),
            puerto_toml::parse_field_arg("plazas:Plazas").unwrap(),
        ],
        &shared,
    );

    scaffold::run_scaffold("Cliente", &output, &fields, OverwritePolicy::Ask).unwrap();

    let read = |path: &str| fs::read_to_string(output.join(path)).unwrap();
    let mother = read("contextos/cliente/dominio/src/pruebas/madres.rs");
    assert!(
        mother.contains("Email::new(\"ana@ejemplo.es\".to_string())"),
        "{mother}"
    );
    assert!(mother.contains("Plazas::new(12)"), "{mother}");
    let routes = read("contextos/cliente/rest/src/rutas.rs");
    assert!(
        routes.contains("email: \"ana@ejemplo.es\".to_string(),"),
        "{routes}"
    );
    assert!(routes.contains("plazas: 12,"), "{routes}");
    let seed = read("contextos/cliente/infraestructura/src/pruebas/siembra.rs");
    assert!(
        seed.contains("MadreCliente::aleatorio()"),
        "the mother's values: {seed}"
    );
    let web = read("web/src/pruebas/cliente.ts");
    assert!(web.contains("email: \"ana@ejemplo.es\","), "{web}");
    assert!(web.contains("plazas: 12,"), "{web}");
    assert!(!mother.contains("\"example\""), "{mother}");
    cleanup(&dir);
}
