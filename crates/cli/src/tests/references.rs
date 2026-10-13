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
        if path
            .to_string_lossy()
            .ends_with("_create_customer_table.sql")
        {
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

// ── 18.3: generated backend ─────────────────────────────────────────────────

/// A project with a CRUD `Customer` and an `Order` holding `customer_id:ref:Customer` and
/// `reviewer_id:ref:opt:Customer`. `db`: `--db`; `events`: also `[project] events = "db"`.
fn order_project(name: &str, db: bool, events: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), db, &dir).unwrap();
    if events {
        let manifest = output.join("puerto.toml");
        let toml = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            toml.replace("db = true", "db = true\nevents = \"db\""),
        )
        .unwrap();
    }
    scaffold::run_scaffold("Customer", &output, &[], OverwritePolicy::Ask).unwrap();
    let fields = vec![
        puerto_toml::parse_field_arg("customer_id:ref:Customer").unwrap(),
        puerto_toml::parse_field_arg("reviewer_id:ref:opt:Customer").unwrap(),
    ];
    scaffold::run_scaffold("Order", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

fn read(output: &Path, path: &str) -> String {
    fs::read_to_string(output.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn should_hold_the_target_id_type_in_the_domain() {
    let (dir, output) = order_project("references_domain", false, false);

    let model = read(&output, "business/src/domain/order/model.rs");
    assert!(
        model.contains("use crate::domain::customer::model::CustomerId;"),
        "{model}"
    );
    assert!(model.contains("pub customer_id: CustomerId,"), "{model}");
    assert!(
        model.contains("pub reviewer_id: Option<CustomerId>,"),
        "{model}"
    );

    for uc in ["create_order", "update_order"] {
        let params = read(
            &output,
            &format!("business/src/domain/order/use_cases/{uc}.rs"),
        );
        assert!(
            params.contains("use crate::domain::customer::model::CustomerId;"),
            "{uc}"
        );
        assert!(
            params.contains("pub customer_id: CustomerId,"),
            "{uc}: {params}"
        );
        assert!(
            params.contains("pub reviewer_id: Option<CustomerId>,"),
            "{uc}"
        );
        assert!(!params.contains("uuid::Uuid"), "{uc} imports Uuid");
    }

    let mother = read(&output, "business/src/tests/mothers/order_mother.rs");
    assert!(
        mother.contains("pub fn with_customer_id(mut self, customer_id: CustomerId)"),
        "{mother}"
    );
    assert!(
        mother.contains("unwrap_or_else(CustomerId::generate)"),
        "{mother}"
    );
    assert!(!mother.contains("uuid::Uuid"), "{mother}");

    let create = read(&output, "business/src/application/order/create_order.rs");
    assert!(
        create.contains("customer_id: CustomerId::generate(),"),
        "{create}"
    );

    cleanup(&dir);
}

#[test]
fn should_turn_a_missing_reference_into_a_validation_error_on_the_field() {
    let (dir, output) = order_project("references_errors", false, false);

    let errors = read(&output, "business/src/domain/order/errors.rs");
    assert!(
        errors.contains(
            "RepositoryError::MissingReference(field) => {\n                Self::ValidationError(format!(\"{field}_not_found\"))"
        ),
        "{errors}"
    );
    assert!(
        errors.contains("\"order.validation_error.customer_id_not_found\""),
        "{errors}"
    );
    let customer_errors = read(&output, "business/src/domain/customer/errors.rs");
    assert!(
        !customer_errors.contains("MissingReference"),
        "{customer_errors}"
    );

    cleanup(&dir);
}

#[test]
fn should_convert_a_reference_at_the_api_edge() {
    let (dir, output) = order_project("references_api", false, false);

    let dto = read(&output, "presentation/src/api/order/dto.rs");
    assert!(dto.contains("pub customer_id: Uuid,"), "{dto}");
    assert!(dto.contains("pub reviewer_id: Option<Uuid>,"), "{dto}");
    assert!(
        dto.contains("customer_id: entity.customer_id.value(),"),
        "{dto}"
    );
    assert!(
        dto.contains("reviewer_id: entity.reviewer_id.map(|v| v.value()),"),
        "{dto}"
    );

    let routes = read(&output, "presentation/src/api/order/routes.rs");
    assert!(
        routes.contains("use business::domain::customer::model::CustomerId;"),
        "{routes}"
    );
    assert!(
        routes.contains("customer_id: CustomerId::from_uuid(body.customer_id),"),
        "{routes}"
    );
    assert!(
        routes.contains("reviewer_id: body.reviewer_id.map(CustomerId::from_uuid),"),
        "{routes}"
    );
    assert!(
        routes
            .contains("async fn should_respond_422_when_a_referenced_customer_id_does_not_exist()"),
        "{routes}"
    );

    cleanup(&dir);
}

#[test]
fn should_hold_a_uuid_in_the_row_and_the_target_id_in_the_entity() {
    let (dir, output) = order_project("references_row", true, false);

    let entity = read(&output, "infrastructure/src/order/entity.rs");
    assert!(
        entity.contains("use business::domain::customer::model::CustomerId;"),
        "{entity}"
    );
    assert!(entity.contains("pub customer_id: Uuid,"), "{entity}");
    assert!(
        entity.contains("pub reviewer_id: Option<Uuid>,"),
        "{entity}"
    );
    assert!(
        entity.contains("customer_id: CustomerId::from_uuid(row.customer_id),"),
        "{entity}"
    );
    assert!(
        entity.contains("reviewer_id: row.reviewer_id.map(CustomerId::from_uuid),"),
        "{entity}"
    );
    assert!(
        entity.contains("customer_id: entity.customer_id.value(),"),
        "{entity}"
    );

    cleanup(&dir);
}

#[test]
fn should_create_a_foreign_key_and_an_index_per_reference() {
    let (dir, output) = order_project("references_migration", true, false);

    let migration = fs::read_dir(output.join("infrastructure/migrations"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("_create_order_table.sql"))
        .map(|p| fs::read_to_string(p).unwrap())
        .unwrap();
    assert!(
        migration.contains(
            "customer_id UUID NOT NULL CONSTRAINT orders_customer_id_fkey REFERENCES customers (id),"
        ),
        "{migration}"
    );
    assert!(
        migration.contains(
            "reviewer_id UUID CONSTRAINT orders_reviewer_id_fkey REFERENCES customers (id),"
        ),
        "{migration}"
    );
    assert!(
        migration.contains("CREATE INDEX orders_customer_id_idx ON orders (customer_id);"),
        "{migration}"
    );
    assert!(
        migration.contains("CREATE INDEX orders_reviewer_id_idx ON orders (reviewer_id);"),
        "{migration}"
    );

    cleanup(&dir);
}

#[test]
fn should_index_a_unique_reference_only_once() {
    let sql = crate::generators::infrastructure::create_table_sql(
        "order",
        &[puerto_toml::parse_field_arg("customer_id:ref:Customer!").unwrap()],
    );

    assert!(
        sql.contains("CREATE UNIQUE INDEX orders_customer_id_key"),
        "{sql}"
    );
    assert!(!sql.contains("orders_customer_id_idx"), "{sql}");
}

#[test]
fn should_map_the_foreign_keys_where_the_rows_are_written() {
    let (dir, output) = order_project("references_pg_repo", true, false);

    let repo = read(&output, "infrastructure/src/order/repository.rs");
    assert!(
        repo.contains("const REFERENCES: &[ForeignKey] = &["),
        "{repo}"
    );
    assert!(
        repo.contains("constraint: \"orders_customer_id_fkey\","),
        "{repo}"
    );
    assert!(repo.contains("field: \"reviewer_id\","), "{repo}");
    assert!(
        repo.contains("map_sqlx_error_with(&e, REFERENCES)"),
        "{repo}"
    );
    // The rows it seeds first get their customer through Customer's own seed.
    assert!(
        repo.contains(
            "customer_id: crate::customer::repository::integration_tests::seed(pool).await.id,"
        ),
        "{repo}"
    );
    assert!(
        repo.contains("async fn should_reject_a_missing_customer_id(pool: PgPool)"),
        "{repo}"
    );
    assert!(
        repo.contains("async fn should_reject_a_missing_reviewer_id(pool: PgPool)"),
        "{repo}"
    );
    assert!(
        repo.contains("RepositoryError::MissingReference(\"customer_id\")"),
        "{repo}"
    );
    let customer = read(&output, "infrastructure/src/customer/repository.rs");
    assert!(
        customer.contains("pub(crate) mod integration_tests {"),
        "{customer}"
    );
    assert!(
        customer.contains("pub(crate) async fn seed(pool: &PgPool) -> Customer {"),
        "{customer}"
    );
    assert!(!customer.contains("REFERENCES"), "{customer}");

    cleanup(&dir);
}

#[test]
fn should_map_the_foreign_keys_in_the_unit_of_work_of_an_events_project() {
    let (dir, output) = order_project("references_pg_uow", true, true);

    let uow = read(&output, "infrastructure/src/order/unit_of_work.rs");
    assert!(
        uow.contains("const REFERENCES: &[ForeignKey] = &["),
        "{uow}"
    );
    assert!(uow.contains("map_sqlx_error_with(&e, REFERENCES)"), "{uow}");
    assert!(
        uow.contains("async fn should_reject_a_missing_customer_id(pool: PgPool)"),
        "{uow}"
    );
    let repo = read(&output, "infrastructure/src/order/repository.rs");
    assert!(
        !repo.contains("REFERENCES"),
        "the repository only reads: {repo}"
    );

    cleanup(&dir);
}

// ── 18.3: end to end (`make test/db`) ──────────────────────────────────────

#[test]
#[ignore = "slow: needs Postgres — run via `make test/db`"]
fn postgres_references_project_answers_422_on_the_field_of_a_missing_reference() {
    use super::compile_matrix::REFERENCES_DB;
    use super::worker_e2e::{build_scenario_project, free_port, http, wait_until};

    const PROJECT: &str = "e2e-references";
    let Some(project) = build_scenario_project(
        &REFERENCES_DB,
        PROJECT,
        "puerto_matrix_references_e2e",
        &[],
        |_| {},
    ) else {
        eprintln!("PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)");
        return;
    };
    let port = free_port();
    let api = project.spawn(
        PROJECT,
        &project.dir.join("api.log"),
        &[("SERVICE_PORT", port.to_string())],
    );
    wait_until("the API to be ready", || {
        http(port, "GET", "/health/ready", "").is_some_and(|(s, _)| s == 200)
    });
    let order = |customer: &str, reviewer: &str| {
        format!(r#"{{"customerId":"{customer}","reviewerId":{reviewer},"total":42}}"#)
    };
    let missing = "00000000-0000-4000-8000-000000000000";

    // A customer nobody created: the foreign key refuses it, on its field.
    let (status, body) = http(port, "POST", "/api/orders", &order(missing, "null")).unwrap();
    assert_eq!(status, 422, "{body}");
    assert!(
        body.contains("\"name\":\"order.validation_error.customer_id_not_found\""),
        "{body}"
    );

    // An existing one: created; an unknown reviewer is refused on its own field.
    let (status, body) = http(
        port,
        "POST",
        "/api/customers",
        r#"{"name":"Ada","email":"ada@example.com"}"#,
    )
    .unwrap();
    assert_eq!(status, 201, "{body}");
    let customer: serde_json::Value = serde_json::from_str(&body).unwrap();
    let customer = customer["id"].as_str().unwrap().to_string();
    let (status, body) = http(port, "POST", "/api/orders", &order(&customer, "null")).unwrap();
    assert_eq!(status, 201, "{body}");
    let reviewer = format!("\"{missing}\"");
    let (status, body) = http(port, "POST", "/api/orders", &order(&customer, &reviewer)).unwrap();
    assert_eq!(status, 422, "{body}");
    assert!(
        body.contains("\"name\":\"order.validation_error.reviewer_id_not_found\""),
        "{body}"
    );

    // The list filters by customer (18.4): another customer's order is left out, page and total.
    let (status, body) = http(
        port,
        "POST",
        "/api/customers",
        r#"{"name":"Grace","email":"grace@example.com"}"#,
    )
    .unwrap();
    assert_eq!(status, 201, "{body}");
    let other: serde_json::Value = serde_json::from_str(&body).unwrap();
    let other = other["id"].as_str().unwrap().to_string();
    let (status, body) = http(port, "POST", "/api/orders", &order(&other, "null")).unwrap();
    assert_eq!(status, 201, "{body}");
    let (status, body) = http(
        port,
        "GET",
        &format!("/api/orders?customerId={customer}"),
        "",
    )
    .unwrap();
    assert_eq!(status, 200, "{body}");
    let page: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(page["total"], 1, "{body}");
    assert_eq!(page["items"][0]["customerId"], customer.as_str(), "{body}");

    drop(api);
    cleanup(&project.dir);
}

#[test]
fn should_test_that_create_reports_a_missing_reference_on_its_field() {
    for (name, events) in [
        ("references_create_test", false),
        ("references_create_test_events", true),
    ] {
        let (dir, output) = order_project(name, events, events);

        let create = read(&output, "business/src/application/order/create_order.rs");
        for field in ["customer_id", "reviewer_id"] {
            assert!(
                create.contains(&format!(
                    "async fn should_return_a_validation_error_when_the_{field}_does_not_exist()"
                )),
                "events = {events}: {create}"
            );
            assert!(create.contains(&format!("RepositoryError::MissingReference(\"{field}\")")));
            assert!(create.contains(&format!("\"order.validation_error.{field}_not_found\"")));
        }

        cleanup(&dir);
    }
}

// ── 18.4: list by reference ─────────────────────────────────────────────────

#[test]
fn should_filter_the_list_by_each_reference_in_the_domain() {
    let (dir, output) = order_project("references_filter_domain", false, false);

    let repository = read(&output, "business/src/domain/order/repository.rs");
    assert!(
        repository.contains("use crate::domain::customer::model::CustomerId;"),
        "{repository}"
    );
    assert!(
        repository.contains(
            "pub struct OrderFilter {\n    pub customer_id: Option<CustomerId>,\n    pub reviewer_id: Option<CustomerId>,\n}"
        ),
        "{repository}"
    );
    assert_eq!(
        repository
            .matches("async fn find_all(&self, page: PageRequest, filter: OrderFilter)")
            .count(),
        2,
        "the port and its mock: {repository}"
    );
    let params = read(
        &output,
        "business/src/domain/order/use_cases/list_orders.rs",
    );
    assert!(
        params.contains("pub customer_id: Option<CustomerId>,"),
        "{params}"
    );
    assert!(
        params.contains("pub reviewer_id: Option<CustomerId>,"),
        "{params}"
    );

    // An entity without references keeps today's port.
    let customer = read(&output, "business/src/domain/customer/repository.rs");
    assert!(!customer.contains("Filter"), "{customer}");
    assert!(
        customer.contains("async fn find_all(&self, page: PageRequest)"),
        "{customer}"
    );

    cleanup(&dir);
}

#[test]
fn should_pass_the_filter_from_the_use_case_to_the_repository() {
    let (dir, output) = order_project("references_filter_application", false, false);

    let list = read(&output, "business/src/application/order/list_orders.rs");
    assert!(
        list.contains(
            "OrderFilter {\n                    customer_id: params.customer_id,\n                    reviewer_id: params.reviewer_id,\n                }"
        ),
        "{list}"
    );
    assert!(
        list.contains("async fn should_pass_the_filter_to_the_repository()"),
        "{list}"
    );

    let repo = read(&output, "infrastructure/src/order/repository.rs");
    assert!(
        repo.contains(".is_none_or(|id| row.customer_id == id)"),
        "{repo}"
    );
    assert!(
        repo.contains(".is_none_or(|id| row.reviewer_id == Some(id))"),
        "{repo}"
    );
    assert!(
        repo.contains("async fn should_list_only_the_orders_of_the_requested_customer_id()"),
        "{repo}"
    );

    cleanup(&dir);
}

#[test]
fn should_take_each_reference_filter_as_a_camel_case_query_parameter() {
    let (dir, output) = order_project("references_filter_api", false, false);

    let routes = read(&output, "presentation/src/api/order/routes.rs");
    assert!(
        routes.contains("#[oai(name = \"customerId\")] customer_id: Query<Option<Uuid>>"),
        "{routes}"
    );
    assert!(
        routes.contains("customer_id: customer_id.0.map(CustomerId::from_uuid),"),
        "{routes}"
    );
    assert!(
        routes.contains("async fn should_pass_the_reviewer_id_filter_to_the_use_case()"),
        "{routes}"
    );
    assert!(routes.contains("?reviewerId={reviewer_id}"), "{routes}");

    cleanup(&dir);
}

#[test]
fn should_filter_the_page_and_its_total_in_postgres() {
    for (name, events) in [
        ("references_filter_pg", false),
        ("references_filter_pg_events", true),
    ] {
        let (dir, output) = order_project(name, true, events);

        let repo = read(&output, "infrastructure/src/order/repository.rs");
        assert!(
            repo.contains(
                "FROM orders WHERE deleted = false AND ($3::uuid IS NULL OR customer_id = $3) AND ($4::uuid IS NULL OR reviewer_id = $4)"
            ),
            "events = {events}: {repo}"
        );
        assert!(
            repo.contains(
                "FROM orders WHERE deleted = false AND ($1::uuid IS NULL OR customer_id = $1) AND ($2::uuid IS NULL OR reviewer_id = $2)"
            ),
            "the COUNT, events = {events}: {repo}"
        );
        assert!(
            repo.contains("filter.customer_id.map(|id| id.value())"),
            "{repo}"
        );
        for field in ["customer_id", "reviewer_id"] {
            assert!(
                repo.contains(&format!(
                    "async fn should_list_only_the_orders_of_the_requested_{field}(pool: PgPool)"
                )),
                "events = {events}: {repo}"
            );
        }

        cleanup(&dir);
    }
}
