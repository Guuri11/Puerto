use super::*;

// ── Phase 8: puerto validate ─────────────────────────────────────────────────

#[test]
fn validate_valid_config() {
    let dir = temp_dir("validate_valid");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product", "get_product"]

[[entity.fields]]
name = "title"
type = "String"

[[entity.fields]]
name = "price"
type = "i64"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_ok());

    cleanup(&dir);
}

#[test]
fn validate_rejects_unknown_field_type() {
    let dir = temp_dir("validate_bad_type");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "title"
type = "FancyType"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unsupported field type 'FancyType'"),
        "expected type error, got: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_rejects_invalid_entity_name() {
    let dir = temp_dir("validate_bad_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "lowercase_product"
use_cases = ["create_product"]
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("PascalCase"),
        "expected PascalCase error, got: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_rejects_invalid_use_case_name() {
    let dir = temp_dir("validate_bad_usecase");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["CreateProduct"]
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("snake_case"),
        "expected snake_case error, got: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_rejects_duplicate_field_name() {
    let dir = temp_dir("validate_dup_field");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "title"
type = "String"

[[entity.fields]]
name = "title"
type = "i64"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate field name 'title'"),
        "expected duplicate error, got: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_warns_on_option_unique_field() {
    let dir = temp_dir("validate_opt_unique");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "email"
type = "Option<String>"
unique = true
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_ok());

    cleanup(&dir);
}

#[test]
fn validate_rejects_duplicate_entity_name() {
    let dir = temp_dir("validate_dup_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity]]
name = "Product"
use_cases = ["get_product"]
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate entity name 'Product'"),
        "expected duplicate entity error, got: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_rejects_invalid_field_name() {
    let dir = temp_dir("validate_bad_field_name");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "BadName"
type = "String"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("snake_case"),
        "expected snake_case error, got: {err}"
    );

    cleanup(&dir);
}

// ── T-08: manifest / filesystem drift ─────────────────────────────────────

/// Declares an extra entity in puerto.toml without generating a single file for it.
fn declare_phantom_entity(base: &Path, name: &str) {
    puerto_toml::add_entity(
        base,
        name,
        vec![format!("create_{}", name.to_lowercase())],
        false,
        vec![],
    )
    .unwrap();
}

#[test]
fn validate_fails_on_declared_but_unscaffolded_entity() {
    let dir = temp_dir("drift_validate");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-validate", &dir).unwrap();

    declare_phantom_entity(&output, "Invoice");

    let result = validate::run_validate(&output);

    assert!(result.is_err(), "a phantom entity must fail validation");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invoice") && err.contains("puerto generate scaffold Invoice"),
        "error should name the entity and the fix: {err}"
    );

    cleanup(&dir);
}

#[test]
fn validate_passes_on_a_freshly_generated_project() {
    // The demo Greeting entity must satisfy the same consistency pass.
    let dir = temp_dir("drift_validate_clean");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-clean", &dir).unwrap();

    assert!(validate::run_validate(&output).is_ok());

    cleanup(&dir);
}

#[test]
fn validate_warns_but_passes_on_an_undeclared_domain_module() {
    let dir = temp_dir("drift_orphan");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-orphan", &dir).unwrap();

    fs::create_dir_all(output.join("business/src/domain/ghost")).unwrap();

    // Orphan code compiles — it is simply invisible to Puerto. A warning, not an error.
    assert!(validate::run_validate(&output).is_ok());
    let orphans = crate::generators::consistency::orphan_entity_dirs(
        &output,
        &puerto_toml::read(&output).unwrap(),
    );
    assert_eq!(orphans, vec!["ghost".to_string()]);

    cleanup(&dir);
}

#[test]
fn bootstrap_command_refuses_when_declared_code_is_missing() {
    let dir = temp_dir("drift_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-bootstrap", &dir).unwrap();

    let before =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    declare_phantom_entity(&output, "Invoice");

    let result = scaffold::run_bootstrap_command(&output, false);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("--allow-missing"), "got: {err}");
    assert_eq!(
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap(),
        before,
        "a refused command must not have written anything"
    );

    cleanup(&dir);
}

#[test]
fn bootstrap_command_writes_anyway_with_allow_missing() {
    let dir = temp_dir("drift_bootstrap_allow");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-bootstrap-allow", &dir).unwrap();

    declare_phantom_entity(&output, "Invoice");

    scaffold::run_bootstrap_command(&output, true).unwrap();

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(bootstrap.contains("InvoiceApi"));

    cleanup(&dir);
}

#[test]
fn scaffolding_the_declared_entity_clears_the_drift() {
    let dir = temp_dir("drift_cleared");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("drift-cleared", &dir).unwrap();

    declare_phantom_entity(&output, "Invoice");
    scaffold::run_scaffold("Invoice", &output, &[], OverwritePolicy::Ask).unwrap();

    assert!(validate::run_validate(&output).is_ok());
    assert!(scaffold::run_bootstrap_command(&output, false).is_ok());

    cleanup(&dir);
}

// ── T-12: name validation gaps ────────────────────────────────────────────

#[test]
fn parse_field_arg_rejects_system_field_names() {
    for name in crate::validation::SYSTEM_FIELDS {
        let arg = format!("{name}:String");
        let result = puerto_toml::parse_field_arg(&arg);
        assert!(result.is_err(), "'{arg}' must be rejected");
        assert!(result.unwrap_err().contains("reserved"));
    }
}

#[test]
fn parse_field_arg_rejects_rust_keyword_field_names() {
    let result = puerto_toml::parse_field_arg("type:String");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Rust keyword"));
}

#[test]
fn parse_field_arg_rejects_leading_underscore_field_name() {
    assert!(puerto_toml::parse_field_arg("_hidden:String").is_err());
}

#[test]
fn parse_field_arg_rejects_duplicate_enum_variants() {
    let result = puerto_toml::parse_field_arg("status:Status:enum:Active/Active");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("duplicate enum variant"));
}

#[test]
fn validate_rejects_hand_edited_system_field_name() {
    let dir = temp_dir("validate_system_field");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "created_at"
type = "DateTime<Utc>"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("reserved"));

    cleanup(&dir);
}

#[test]
fn validate_rejects_entity_name_that_collides_with_generated_code() {
    let dir = temp_dir("validate_reserved_entity");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Error"
use_cases = ["create_error"]
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("collides"));

    cleanup(&dir);
}

#[test]
fn validate_rejects_conflicting_value_object_inner_types() {
    let dir = temp_dir("validate_vo_conflict");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "code"
type = "String"
value_object = "Code"

[[entity.fields]]
name = "other_code"
type = "i64"
value_object = "Code"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("declared twice"),
        "one value_objects.rs cannot define Code as both String and i64"
    );

    cleanup(&dir);
}

#[test]
fn scaffold_refuses_an_entity_name_that_collides_with_generated_code() {
    let dir = temp_dir("scaffold_reserved_name");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("reserved-name", &dir).unwrap();

    let result = scaffold::run_scaffold("Tracing", &output, &[], OverwritePolicy::Ask);

    assert!(result.is_err());
    assert!(
        !output.join("business/src/domain/tracing").is_dir(),
        "nothing may be written for a refused name"
    );

    cleanup(&dir);
}

#[test]
fn model_does_not_reimport_chrono_or_uuid_for_field_types() {
    // model.tera imports chrono/uuid for the system fields; a DateTime/Uuid field must not
    // re-import them (E0252: the name `DateTime` is defined multiple times).
    let fields = vec![
        puerto_toml::Field {
            name: "released_at".into(),
            field_type: "DateTime<Utc>".into(),
            ..Default::default()
        },
        puerto_toml::Field {
            name: "external_id".into(),
            field_type: "Uuid".into(),
            ..Default::default()
        },
    ];

    let model =
        crate::generators::domain::generate_model("Product", "product", &fields, &[]).unwrap();

    assert_eq!(model.matches("use chrono::{DateTime, Utc};").count(), 1);
    assert_eq!(model.matches("use uuid::Uuid;").count(), 1);
}

#[test]
fn scaffold_rejects_sql_reserved_field_names_in_a_database_project() {
    let dir = temp_dir("sql_reserved_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs_with_project_db(&dir);

    let result = scaffold::run_scaffold(
        "Product",
        &dir,
        &[scaffold_test_field("desc", "String")],
        OverwritePolicy::Ask,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQL"));
    assert!(!dir.join("business/src/domain/product").exists());

    cleanup(&dir);
}

#[test]
fn scaffold_accepts_sql_reserved_field_names_without_a_database() {
    let dir = temp_dir("sql_reserved_no_db");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);

    let result = scaffold::run_scaffold(
        "Product",
        &dir,
        &[scaffold_test_field("order", "String")],
        OverwritePolicy::Ask,
    );

    assert!(result.is_ok(), "{result:?}");
    cleanup(&dir);
}

#[test]
fn validate_rejects_sql_reserved_field_names_in_a_database_project() {
    let dir = temp_dir("validate_sql_reserved");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"
db = true

[[entity]]
name = "Product"
use_cases = ["create_product"]

[[entity.fields]]
name = "user"
type = "String"
"#,
    )
    .unwrap();

    let result = validate::run_validate(&dir);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SQL"));
    cleanup(&dir);
}

#[test]
fn duplicate_field_names_are_rejected() {
    let fields = vec![
        scaffold_test_field("name", "String"),
        scaffold_test_field("name", "i64"),
    ];

    let result = crate::validation::validate_unique_field_names(&fields);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("'name'"));
}

#[test]
fn project_name_must_be_a_valid_package_name() {
    use crate::commands::new::validate_project_name;
    assert!(validate_project_name("Bad Name").is_err());
    assert!(validate_project_name("MyApp").is_err());
    assert!(validate_project_name("1app").is_err());
    assert!(validate_project_name("my-app").is_ok());
    assert!(validate_project_name("my_app2").is_ok());
}

// ── Phase 16.4: auth ─────────────────────────────────────────────────────────

fn validate_manifest(name: &str, manifest: &str) -> Result<(), String> {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), manifest).unwrap();
    let result = validate::run_validate(&dir).map_err(|e| e.to_string());
    cleanup(&dir);
    result
}

#[test]
fn validate_accepts_public_and_roles_in_an_auth_project() {
    let result = validate_manifest(
        "validate_auth_ok",
        r#"[project]
name = "my-app"
auth = "oidc"

[[entity]]
name = "Product"
use_cases = ["create_product", "list_products"]
public = ["list_products"]
roles = { create_product = ["admin"] }
"#,
    );

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_rejects_an_unknown_auth_mode() {
    let result = validate_manifest(
        "validate_auth_mode",
        "[project]\nname = \"my-app\"\nauth = \"auth0\"\n",
    );

    assert!(result.unwrap_err().contains("unknown auth mode 'auth0'"));
}

#[test]
fn validate_rejects_local_auth_without_a_database() {
    let result = validate_manifest(
        "validate_auth_local",
        "[project]\nname = \"my-app\"\nauth = \"local\"\n",
    );

    assert!(result.unwrap_err().contains("needs project.db = true"));
}

#[test]
fn validate_rejects_public_and_roles_without_auth() {
    let result = validate_manifest(
        "validate_auth_none",
        r#"[project]
name = "my-app"

[[entity]]
name = "Product"
use_cases = ["list_products"]
public = ["list_products"]
"#,
    );

    assert!(
        result
            .unwrap_err()
            .contains("declares public or roles, but project.auth is not set")
    );
}

#[test]
fn validate_rejects_the_user_entity_in_an_auth_project() {
    let result = validate_manifest(
        "validate_auth_user",
        r#"[project]
name = "my-app"
auth = "oidc"

[[entity]]
name = "User"
use_cases = ["get_user"]
"#,
    );

    assert!(result.unwrap_err().contains("an auth project already uses"));
}

#[test]
fn validate_rejects_roles_naming_another_use_case() {
    let result = validate_manifest(
        "validate_auth_roles",
        r#"[project]
name = "my-app"
auth = "oidc"

[[entity]]
name = "Product"
use_cases = ["create_product"]
roles = { delete_product = ["admin"] }
"#,
    );

    assert!(
        result
            .unwrap_err()
            .contains("roles: 'delete_product' is not one of the entity's use cases")
    );
}

#[test]
fn auth_access_survives_a_write_of_puerto_toml() {
    let dir = temp_dir("auth_toml_roundtrip");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"
auth = "oidc"

[[entity]]
name = "Product"
use_cases = ["create_product", "list_products"]
public = ["list_products"]
roles = { create_product = ["admin", "editor"] }

[[entity.fields]]
name = "title"
type = "String"
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    puerto_toml::write(&dir, &config).unwrap();
    let written = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    let reread = puerto_toml::read(&dir).unwrap();

    assert_eq!(reread.project.auth.as_deref(), Some("oidc"));
    assert_eq!(reread.entity[0].public, vec!["list_products"]);
    assert_eq!(
        reread.entity[0].roles.get("create_product"),
        Some(&vec!["admin".to_string(), "editor".to_string()])
    );
    assert_eq!(reread.entity[0].fields.len(), 1, "{written}");
    cleanup(&dir);
}
