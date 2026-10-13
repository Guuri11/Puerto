// Phase 18 (ADR-0022): an entity refers to another by holding its typed id.
use super::*;

// ── 18.2: the manifest ──────────────────────────────────────────────────────

#[test]
fn should_parse_a_reference_as_a_uuid_naming_its_target() {
    let field = puerto_toml::parse_field_arg("customer_id:ref:Customer").unwrap();

    assert_eq!(field.name, "customer_id");
    assert_eq!(field.field_type, "Uuid");
    assert_eq!(field.references.as_deref(), Some("Customer"));
    assert!(field.value_object.is_none());
    assert!(!field.unique);
}

#[test]
fn should_parse_an_optional_reference_as_an_optional_uuid() {
    let field = puerto_toml::parse_field_arg("reviewer_id:ref:opt:Customer").unwrap();

    assert_eq!(field.field_type, "Option<Uuid>");
    assert_eq!(field.references.as_deref(), Some("Customer"));
}

#[test]
fn should_parse_a_unique_reference_as_one_to_one() {
    let field = puerto_toml::parse_field_arg("customer_id:ref:Customer!").unwrap();

    assert!(field.unique);
    assert_eq!(field.references.as_deref(), Some("Customer"));
}

#[test]
fn should_reject_a_unique_optional_reference() {
    let err = puerto_toml::parse_field_arg("reviewer_id:ref:opt:Customer!").unwrap_err();

    assert!(err.contains("unique"), "{err}");
}

#[test]
fn should_reject_a_reference_whose_target_is_not_pascal_case() {
    let err = puerto_toml::parse_field_arg("customer_id:ref:customer").unwrap_err();

    assert!(err.contains("customer"), "{err}");
}

#[test]
fn should_keep_a_reference_through_puerto_toml() {
    // Arrange
    let dir = temp_dir("references_round_trip");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), "[project]\nname = \"shop\"\n").unwrap();
    let field = puerto_toml::parse_field_arg("customer_id:ref:Customer").unwrap();

    // Act
    puerto_toml::add_entity(&dir, "Order", vec![], false, vec![field]).unwrap();

    // Assert
    let written = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(written.contains("references = \"Customer\""), "{written}");
    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(
        config.entity[0].fields[0].references.as_deref(),
        Some("Customer")
    );

    cleanup(&dir);
}

/// A manifest with a CRUD `Customer`, a simple `Greeting` and an `Order` holding `order_field`.
fn validate_order_with(name: &str, project_extra: &str, order_field: &str) -> Result<(), String> {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        format!(
            r#"[project]
name = "shop"
{project_extra}
[[entity]]
name = "Greeting"
use_cases = ["get_greeting"]

[[entity]]
name = "Customer"
use_cases = ["create_customer", "get_customer", "list_customers", "update_customer", "delete_customer"]

[[entity]]
name = "Order"
use_cases = ["create_order", "get_order", "list_orders", "update_order", "delete_order"]

[[entity.fields]]
{order_field}
"#
        ),
    )
    .unwrap();
    let result = validate::run_validate(&dir).map_err(|e| e.to_string());
    cleanup(&dir);
    result
}

#[test]
fn should_validate_a_reference_to_a_declared_crud_entity() {
    let result = validate_order_with(
        "references_validate_ok",
        "",
        "name = \"customer_id\"\ntype = \"Uuid\"\nreferences = \"Customer\"",
    );

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_refuse_a_reference_to_an_undeclared_entity() {
    let err = validate_order_with(
        "references_validate_undeclared",
        "",
        "name = \"supplier_id\"\ntype = \"Uuid\"\nreferences = \"Supplier\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'supplier_id'") && err.contains("Supplier"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_to_the_entity_itself() {
    let err = validate_order_with(
        "references_validate_self",
        "",
        "name = \"parent_id\"\ntype = \"Option<Uuid>\"\nreferences = \"Order\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'parent_id'") && err.contains("itself"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_to_an_entity_without_crud() {
    let err = validate_order_with(
        "references_validate_simple",
        "",
        "name = \"greeting_id\"\ntype = \"Uuid\"\nreferences = \"Greeting\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'greeting_id'") && err.contains("CRUD"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_to_the_user_in_an_auth_project() {
    let err = validate_order_with(
        "references_validate_user",
        "auth = \"oidc\"",
        "name = \"owner_id\"\ntype = \"Uuid\"\nreferences = \"User\"",
    )
    .unwrap_err();

    assert!(err.contains("'owner_id'") && err.contains("User"), "{err}");
}

#[test]
fn should_refuse_a_reference_that_is_not_a_uuid() {
    let err = validate_order_with(
        "references_validate_type",
        "",
        "name = \"customer_id\"\ntype = \"i64\"\nreferences = \"Customer\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'customer_id'") && err.contains("Uuid"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_that_is_also_a_value_object() {
    let err = validate_order_with(
        "references_validate_vo",
        "",
        "name = \"customer_id\"\ntype = \"Uuid\"\nreferences = \"Customer\"\nvalue_object = \"CustomerRef\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'customer_id'") && err.contains("value_object"),
        "{err}"
    );
}

// ── 18.2: scaffold refuses before writing ───────────────────────────────────

fn order_fields() -> Vec<puerto_toml::Field> {
    vec![puerto_toml::parse_field_arg("customer_id:ref:Customer").unwrap()]
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_an_undeclared_entity() {
    // Arrange
    let dir = temp_dir("references_scaffold_undeclared");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), false, &dir).unwrap();

    // Act
    let err = scaffold::run_scaffold("Order", &output, &order_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("Customer"), "{err}");
    assert!(!output.join("business/src/domain/order").exists());

    cleanup(&dir);
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_a_target_without_code() {
    // Arrange: Customer is declared, but nothing was generated for it.
    let dir = temp_dir("references_scaffold_no_code");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), true, &dir).unwrap();
    puerto_toml::add_entity(
        &output,
        "Customer",
        crate::generators::naming::crud_use_cases("customer"),
        true,
        vec![],
    )
    .unwrap();

    // Act
    let err = scaffold::run_scaffold("Order", &output, &order_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("puerto generate scaffold Customer"), "{err}");
    assert!(!output.join("business/src/domain/order").exists());

    cleanup(&dir);
}

#[test]
fn should_refuse_to_add_the_domain_of_a_reference_to_an_undeclared_entity() {
    // Arrange
    let dir = temp_dir("references_domain_undeclared");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), false, &dir).unwrap();

    // Act
    let err = scaffold::run_generate_domain("Order", &output, &order_fields())
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("Customer"), "{err}");
    assert!(!output.join("business/src/domain/order").exists());

    cleanup(&dir);
}

#[test]
fn should_validate_a_reference_to_an_entity_named_user_outside_an_auth_project() {
    // Arrange: without auth, `User` is an ordinary entity.
    let dir = temp_dir("references_validate_plain_user");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "shop"

[[entity]]
name = "User"
use_cases = ["create_user", "get_user", "list_users", "update_user", "delete_user"]

[[entity]]
name = "Order"
use_cases = ["create_order"]

[[entity.fields]]
name = "owner_id"
type = "Uuid"
references = "User"
"#,
    )
    .unwrap();

    // Act
    let result = validate::run_validate(&dir);

    // Assert
    assert!(result.is_ok(), "{result:?}");
    cleanup(&dir);
}

#[test]
fn should_scaffold_a_reference_to_a_target_with_code() {
    // Arrange
    let dir = temp_dir("references_scaffold_ok");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), false, &dir).unwrap();
    scaffold::run_scaffold("Customer", &output, &[], OverwritePolicy::Ask).unwrap();

    // Act
    let result = scaffold::run_scaffold("Order", &output, &order_fields(), OverwritePolicy::Ask);

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert!(output.join("business/src/domain/order/model.rs").exists());
    cleanup(&dir);
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_a_target_whose_table_is_not_created() {
    // Arrange: the model is there, the migration that creates its table is not.
    let dir = temp_dir("references_scaffold_no_table");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), true, &dir).unwrap();
    scaffold::run_scaffold("Customer", &output, &[], OverwritePolicy::Ask).unwrap();
    for entry in fs::read_dir(output.join("infrastructure/migrations")).unwrap() {
        let path = entry.unwrap().path();
        if path.to_string_lossy().ends_with("_create_customer_table.sql") {
            fs::remove_file(path).unwrap();
        }
    }

    // Act
    let err = scaffold::run_scaffold("Order", &output, &order_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("puerto generate scaffold Customer"), "{err}");
    cleanup(&dir);
}
