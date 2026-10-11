use super::*;

// ── puerto.toml fields ────────────────────────────────────────────────────

#[test]
fn parse_puerto_toml_with_fields() {
    let dir = temp_dir("toml_fields_parse");
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
name = "name"
type = "String"

[[entity.fields]]
name = "price"
type = "i64"

[[entity.fields]]
name = "sku"
type = "String"
unique = true

[[entity.fields]]
name = "description"
type = "Option<String>"
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.entity.len(), 1);
    let product = &config.entity[0];
    assert_eq!(product.name, "Product");
    assert_eq!(product.fields.len(), 4);
    assert_eq!(product.fields[0].name, "name");
    assert_eq!(product.fields[0].field_type, "String");
    assert!(!product.fields[0].unique);
    assert_eq!(product.fields[1].name, "price");
    assert_eq!(product.fields[1].field_type, "i64");
    assert_eq!(product.fields[2].name, "sku");
    assert!(product.fields[2].unique);
    assert_eq!(product.fields[3].name, "description");
    assert_eq!(product.fields[3].field_type, "Option<String>");

    cleanup(&dir);
}

#[test]
fn parse_puerto_toml_without_fields_backwards_compat() {
    let dir = temp_dir("toml_no_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"

[[entity]]
name = "Greeting"
use_cases = ["get_greeting"]
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.entity.len(), 1);
    let greeting = &config.entity[0];
    assert_eq!(greeting.name, "Greeting");
    assert!(greeting.fields.is_empty());

    cleanup(&dir);
}

#[test]
fn add_entity_with_fields() {
    let dir = temp_dir("toml_add_entity_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "my-app"
"#,
    )
    .unwrap();

    let fields = vec![
        puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        puerto_toml::Field {
            name: "quantity".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    puerto_toml::add_entity(
        &dir,
        "Product",
        vec!["create_product".into()],
        false,
        fields,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.entity.len(), 1);
    assert_eq!(config.entity[0].fields.len(), 2);
    assert_eq!(config.entity[0].fields[0].name, "title");
    assert_eq!(config.entity[0].fields[0].field_type, "String");

    cleanup(&dir);
}

#[test]
fn type_registry_resolve_all_types() {
    use crate::generators::types::resolve_type;

    let valid_types = [
        "String",
        "i64",
        "bool",
        "f64",
        "Option<String>",
        "Option<i64>",
        "Option<bool>",
        "Option<f64>",
        "Uuid",
        "DateTime<Utc>",
        "Option<DateTime<Utc>>",
        "Vec<String>",
        "Vec<i64>",
        "HashMap<String, String>",
    ];
    for t in &valid_types {
        assert!(resolve_type(t).is_ok(), "expected '{t}' to resolve");
    }
}

#[test]
fn type_registry_rejects_unknown() {
    use crate::generators::types::resolve_type;
    let err = resolve_type("FooBar").unwrap_err();
    assert!(err.contains("unsupported field type 'FooBar'"));
}

// ── Phase 2: Domain model with custom fields ──────────────────────────────────

#[test]
fn generate_model_backward_compat_no_fields() {
    use crate::generators::domain::generate_model;

    let result = generate_model("Product", "product", &[], &[]).unwrap();
    assert!(result.contains("pub struct ProductProps"));
    assert!(result.contains("pub name: String,"));
    assert!(result.contains("pub struct Product"));
    assert!(result.contains("pub id: Uuid,"));
    assert!(result.contains("pub name: String,"));
    assert!(result.contains("if props.name.trim().is_empty()"));
    assert!(result.contains("ProductError::ValidationError(\"name_empty\""));
    assert!(result.contains("should_create_product_when_name_is_valid"));
    assert!(result.contains("should_reject_product_when_name_is_empty"));
    assert!(result.contains("should_reject_product_when_name_is_only_whitespace"));
}

#[test]
fn generate_model_with_custom_fields() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "description".into(),
            field_type: "Option<String>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_model("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("pub struct ProductProps"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("pub struct Product"));
    assert!(result.contains("pub id: Uuid,"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("if props.title.trim().is_empty()"));
    assert!(result.contains("title_empty"));
    assert!(result.contains("should_create_product_when_fields_are_valid"));
    assert!(result.contains("should_reject_product_when_title_is_empty"));
    assert!(result.contains("should_reject_product_when_title_is_only_whitespace"));
    assert!(
        !result.contains("if props.price.trim().is_empty()"),
        "non-String types should not have empty checks"
    );
    assert!(
        !result.contains("if props.description.trim().is_empty()"),
        "Option<String> types should not have empty checks"
    );
}

#[test]
fn generate_model_with_uuid_field() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::Field;

    let fields = vec![Field {
        name: "category_id".into(),
        field_type: "Uuid".into(),
        unique: false,
        ..Default::default()
    }];
    let result = generate_model("Item", "item", &fields, &[]).unwrap();

    assert!(result.contains("use uuid::Uuid;"));
    assert!(result.contains("pub category_id: Uuid,"));
    assert!(result.contains("should_create_item_when_fields_are_valid"));
}

#[test]
fn generate_model_with_hashmap_field() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::Field;

    let fields = vec![Field {
        name: "metadata".into(),
        field_type: "HashMap<String, String>".into(),
        unique: false,
        ..Default::default()
    }];
    let result = generate_model("Config", "config", &fields, &[]).unwrap();

    assert!(result.contains("use std::collections::HashMap;"));
    assert!(result.contains("pub metadata: HashMap<String, String>,"));
}

#[test]
fn generate_model_multiple_string_fields_has_validations() {
    use crate::generators::domain::generate_model;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "sku".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_model("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("if props.name.trim().is_empty()"));
    assert!(result.contains("name_empty"));
    assert!(result.contains("if props.sku.trim().is_empty()"));
    assert!(result.contains("sku_empty"));
    assert!(result.contains("should_reject_product_when_name_is_empty"));
    assert!(result.contains("should_reject_product_when_sku_is_empty"));
    assert!(
        !result.contains("if props.price.trim().is_empty()"),
        "non-String types should not have empty checks"
    );
}

#[test]
fn generate_mother_backward_compat_no_fields() {
    use crate::generators::domain::generate_mother;

    let result = generate_mother("Product", "product", &[], &[]).unwrap();
    assert!(result.contains("pub struct ProductMother"));
    assert!(result.contains("name: Option<String>,"));
    assert!(result.contains("pub fn with_name(mut self, name: &str) -> Self"));
    assert!(result.contains("pub fn with_empty_name(mut self) -> Self"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("self.name.unwrap_or_else(|| \"example\".to_string())"));
    assert!(result.contains("pub fn random()"));
    assert!(result.contains("pub fn random_vec"));
}

#[test]
fn generate_mother_with_custom_fields() {
    use crate::generators::domain::generate_mother;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "description".into(),
            field_type: "Option<String>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_mother("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("title: Option<String>,"));
    assert!(result.contains("price: Option<i64>,"));
    assert!(result.contains("description: Option<String>,"));
    assert!(result.contains("pub fn with_title(mut self, title: &str) -> Self"));
    assert!(result.contains("self.title = Some(title.to_string())"));
    assert!(result.contains("pub fn with_price(mut self, price: i64) -> Self"));
    assert!(result.contains("self.price = Some(price)"));
    assert!(result.contains("pub fn with_description(mut self, description: &str) -> Self"));
    assert!(result.contains("self.description = Some(description.to_string())"));
    assert!(result.contains("pub fn with_empty_title(mut self) -> Self"));
    assert!(result.contains("self.title = Some(String::new())"));
    assert!(result.contains("self.price.unwrap_or(42)"));
    assert!(result.contains("self.description"));
    assert!(result.contains("pub fn random()"));
    assert!(result.contains("pub fn random_vec"));
}

#[test]
fn scaffold_with_empty_fields_produces_backward_compat_model() {
    let dir = temp_dir("scaffold_empty_fields");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/domain/product/model.rs")).unwrap();
    assert!(content.contains("pub struct ProductProps"));
    assert!(content.contains("pub name: String,"));
    assert!(content.contains("pub struct Product"));
    assert!(content.contains("ProductError::ValidationError(\"name_empty\""));
    assert!(content.contains("should_create_product_when_name_is_valid"));

    let mother =
        fs::read_to_string(dir.join("business/src/tests/mothers/product_mother.rs")).unwrap();
    assert!(mother.contains("pub struct ProductMother"));
    assert!(mother.contains("with_name"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_with_empty_fields_produces_backward_compat_model() {
    let dir = temp_dir("scaffold_crud_empty_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Product", &dir, false, true, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("business/src/domain/product/model.rs")).unwrap();
    assert!(content.contains("pub struct ProductProps"));
    assert!(content.contains("pub name: String,"));

    let mother =
        fs::read_to_string(dir.join("business/src/tests/mothers/product_mother.rs")).unwrap();
    assert!(mother.contains("ProductMother"));
    assert!(mother.contains("with_name"));

    cleanup(&dir);
}

// ── Phase 3: Use case Params with custom fields ───────────────────────────────

#[test]
fn generate_create_use_case_trait_with_custom_fields() {
    use crate::generators::domain::generate_create_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "description".into(),
            field_type: "Option<String>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_create_use_case_trait("Product", "product", &fields).unwrap();

    assert!(result.contains("pub struct CreateProductParams"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("pub trait CreateProductUseCaseTrait"));
    assert!(
        !result.contains("pub name: String,"),
        "should not contain hardcoded 'name' field"
    );
}

#[test]
fn generate_create_use_case_trait_backward_compat_no_fields() {
    use crate::generators::domain::generate_create_use_case_trait;

    let result = generate_create_use_case_trait("Product", "product", &[]).unwrap();

    assert!(result.contains("pub struct CreateProductParams"));
    assert!(result.contains("pub name: String,"));
    assert!(result.contains("pub trait CreateProductUseCaseTrait"));
}

#[test]
fn generate_update_use_case_trait_with_custom_fields() {
    use crate::generators::domain::generate_update_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_update_use_case_trait("Product", "product", &fields).unwrap();

    assert!(result.contains("pub struct UpdateProductParams"));
    assert!(result.contains("pub id: Uuid,"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub trait UpdateProductUseCaseTrait"));
}

/// A `Uuid` field is an ordinary field — `model.tera` puts it in `EntityProps`, so every params
/// struct that feeds `Entity::new` must carry it too. Dropping it (as the generators used to)
/// produced `missing field 'category_id' in initializer of ItemProps` (E0063).
#[test]
fn generate_update_use_case_trait_includes_uuid_fields() {
    use crate::generators::domain::generate_update_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "category_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_update_use_case_trait("Item", "item", &fields).unwrap();

    assert!(result.contains("pub name: String,"));
    assert!(result.contains("pub id: Uuid,"));
    assert!(result.contains("pub category_id: Uuid,"));
    // The template writes `use uuid::Uuid;` itself for `id` — the field must not add a second one.
    assert_eq!(
        result.matches("use uuid::Uuid;").count(),
        1,
        "a Uuid field must not duplicate the import the template already writes (E0252)"
    );
}

#[test]
fn generate_create_use_case_trait_includes_uuid_fields() {
    use crate::generators::domain::generate_create_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "category_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_create_use_case_trait("Item", "item", &fields).unwrap();

    assert!(result.contains("pub name: String,"));
    assert!(result.contains("pub category_id: Uuid,"));
    assert!(
        !result.contains("pub id: Uuid,"),
        "Create params must not carry the system id"
    );
    assert_eq!(result.matches("use uuid::Uuid;").count(), 1);
}

#[test]
fn generate_create_use_case_impl_with_custom_fields() {
    use crate::generators::application::generate_create_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_create_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("CreateProductUseCaseImpl"));
    assert!(result.contains("CreateProductParams"));
    assert!(result.contains("title: params.title,"));
    assert!(result.contains("price: params.price,"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("tracing::info!(\"create_product: start\");"));
}

#[test]
fn generate_update_use_case_impl_with_custom_fields() {
    use crate::generators::application::generate_update_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_update_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("UpdateProductUseCaseImpl"));
    assert!(result.contains("UpdateProductParams"));
    assert!(result.contains("entity.title = params.title;"));
    assert!(result.contains("entity.price = params.price;"));
    assert!(result.contains("if params.title.trim().is_empty()"));
    assert!(
        !result.contains("if params.price.trim().is_empty()"),
        "non-String types should not have empty validation"
    );
    assert!(result.contains("ProductError::ValidationError(\"title_empty\""));
}

#[test]
fn generate_update_use_case_impl_assigns_uuid_fields() {
    use crate::generators::application::generate_update_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "category_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_update_use_case_impl("Item", "item", &fields, &[]).unwrap();

    assert!(result.contains("entity.name = params.name;"));
    assert!(result.contains("entity.category_id = params.category_id;"));
    // `Uuid` belongs to the test module (the impl names no primitive type); at file level it would
    // be an unused import in a non-test build, which `-D warnings` rejects.
    assert!(result.contains("    use uuid::Uuid;"));
    assert!(
        !result.contains("\nuse uuid::Uuid;"),
        "primitive imports must not sit at file level in the impl"
    );
    assert_eq!(result.matches("use uuid::Uuid;").count(), 1);
}

#[test]
fn generate_create_use_case_impl_passes_uuid_fields_to_props() {
    use crate::generators::application::generate_create_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "category_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_create_use_case_impl("Item", "item", &fields, &[]).unwrap();

    assert!(result.contains("category_id: params.category_id,"));
    assert!(result.contains("name: params.name,"));
    // The generated test builds the params, so it needs `Uuid` — inside `mod tests`, once.
    assert!(result.contains("            category_id: Uuid::new_v4(),"));
    assert!(result.contains("    use uuid::Uuid;"));
    assert!(
        !result.contains("\nuse uuid::Uuid;"),
        "primitive imports must not sit at file level in the impl"
    );
}

#[test]
fn scaffold_crud_writes_dynamic_use_case_params() {
    let dir = temp_dir("scaffold_crud_dynamic_params");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[project]
name = "test-app"

[[entity]]
name = "Product"
use_cases = ["create_product", "get_product", "list_product", "update_product", "delete_product"]

[[entity.fields]]
name = "name"
type = "String"

[[entity.fields]]
name = "price"
type = "i64"
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    let fields = config.entity[0].fields.clone();

    let pascal = "Product";
    let snake = "product";

    let create_trait =
        crate::generators::domain::generate_create_use_case_trait(pascal, snake, &fields).unwrap();
    assert!(create_trait.contains("pub name: String,"));
    assert!(create_trait.contains("pub price: i64,"));
    assert!(
        !create_trait.contains("pub id: Uuid,"),
        "Create params should not have id"
    );

    let update_trait =
        crate::generators::domain::generate_update_use_case_trait(pascal, snake, &fields).unwrap();
    assert!(update_trait.contains("pub id: Uuid,"));
    assert!(update_trait.contains("pub name: String,"));
    assert!(update_trait.contains("pub price: i64,"));

    cleanup(&dir);
}

#[test]
fn write_application_files_with_fields_generates_dynamic_impls() {
    let dir = temp_dir("app_files_with_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::create_dir_all(dir.join("business/src/application/product")).unwrap();

    let fields = vec![
        crate::puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        crate::puerto_toml::Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    crate::generators::application::write_application_files(
        "Product",
        "product",
        &dir,
        &fields,
        &[],
    )
    .unwrap();

    let create_content =
        fs::read_to_string(dir.join("business/src/application/product/create_product.rs")).unwrap();
    assert!(create_content.contains("title: params.title,"));
    assert!(create_content.contains("price: params.price,"));
    assert!(create_content.contains("Product::new(ProductProps"));

    let update_content =
        fs::read_to_string(dir.join("business/src/application/product/update_product.rs")).unwrap();
    assert!(update_content.contains("entity.title = params.title;"));
    assert!(update_content.contains("entity.price = params.price;"));
    assert!(update_content.contains("if params.title.trim().is_empty()"));

    cleanup(&dir);
}

#[test]
fn generate_get_use_case_impl_with_custom_fields() {
    use crate::generators::application::generate_get_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_get_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("GetProductUseCaseImpl"));
    assert!(result.contains("GetProductParams"));
    assert!(result.contains("should_return_product_when_id_exists"));
    assert!(result.contains("should_return_not_found_when_id_does_not_exist"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("title: \"example\".to_string(),"));
    assert!(result.contains("price: 42,"));
    assert!(!result.contains("logger"));
}

#[test]
fn generate_get_use_case_impl_backward_compat_no_fields() {
    use crate::generators::application::generate_get_use_case_impl;

    let result = generate_get_use_case_impl("Product", "product", &[], &[]).unwrap();

    assert!(result.contains("GetProductUseCaseImpl"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("name: \"example\".to_string(),"));
    assert!(result.contains("should_return_product_when_id_exists"));
}

#[test]
fn generate_list_use_case_impl_with_custom_fields() {
    use crate::generators::application::generate_list_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_list_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("ListProductUseCaseImpl"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("should_return_all_products"));
    assert!(result.contains("should_return_empty_list_when_no_products_exist"));
    assert!(result.contains("title: \"first\".to_string(),"));
    assert!(result.contains("title: \"second\".to_string(),"));
    assert!(result.contains("price: 42,"));
    assert!(!result.contains("logger"));
}

#[test]
fn generate_delete_use_case_impl_with_custom_fields() {
    use crate::generators::application::generate_delete_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_delete_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("DeleteProductUseCaseImpl"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(result.contains("should_soft_delete_product_when_id_exists"));
    assert!(result.contains("should_return_not_found_when_product_does_not_exist"));
    assert!(result.contains("title: \"example\".to_string(),"));
    assert!(result.contains("price: 42,"));
    assert!(!result.contains("logger"));
}

#[test]
fn generate_create_use_case_impl_with_custom_fields_has_tests() {
    use crate::generators::application::generate_create_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_create_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("#[cfg(test)]"));
    assert!(result.contains("should_create_product_when_fields_are_valid"));
    assert!(result.contains("should_return_error_when_title_is_empty"));
    assert!(!result.contains("logger"));
    assert!(result.contains("MockProductRepository"));
    assert!(result.contains("title: \"example\".to_string(),"));
    assert!(result.contains("price: 42,"));
}

#[test]
fn generate_update_use_case_impl_with_custom_fields_has_tests() {
    use crate::generators::application::generate_update_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_update_use_case_impl("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("#[cfg(test)]"));
    assert!(result.contains("should_update_product_when_params_are_valid"));
    assert!(result.contains("should_return_not_found_when_product_does_not_exist"));
    assert!(result.contains("should_return_error_when_title_is_empty"));
    assert!(result.contains("Product::new(ProductProps"));
    assert!(!result.contains("logger"));
    assert!(result.contains("title: \"original\".to_string(),"));
    assert!(result.contains("title: \"updated\".to_string(),"));
    assert!(result.contains("price: 42,"));
}

#[test]
fn write_application_files_with_fields_generates_dynamic_get_list_delete() {
    let dir = temp_dir("app_files_dynamic_all");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::create_dir_all(dir.join("business/src/application/product")).unwrap();

    let fields = vec![
        crate::puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        crate::puerto_toml::Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    crate::generators::application::write_application_files(
        "Product",
        "product",
        &dir,
        &fields,
        &[],
    )
    .unwrap();

    let get_content =
        fs::read_to_string(dir.join("business/src/application/product/get_product.rs")).unwrap();
    assert!(get_content.contains("GetProductUseCaseImpl"));
    assert!(get_content.contains("should_return_product_when_id_exists"));
    assert!(get_content.contains("Product::new(ProductProps"));
    assert!(get_content.contains("title: \"example\".to_string(),"));

    let list_content =
        fs::read_to_string(dir.join("business/src/application/product/list_product.rs")).unwrap();
    assert!(list_content.contains("ListProductUseCaseImpl"));
    assert!(list_content.contains("should_return_all_products"));
    assert!(list_content.contains("title: \"first\".to_string(),"));

    let delete_content =
        fs::read_to_string(dir.join("business/src/application/product/delete_product.rs")).unwrap();
    assert!(delete_content.contains("DeleteProductUseCaseImpl"));
    assert!(delete_content.contains("should_soft_delete_product_when_id_exists"));
    assert!(delete_content.contains("Product::new(ProductProps"));

    cleanup(&dir);
}

// ── Infrastructure generators ─────────────────────────────────────────────────

#[test]
fn generate_infra_entity_backward_compat() {
    use crate::generators::infrastructure::generate_infra_entity;

    let result = generate_infra_entity("Product", "product", &[], &[]).unwrap();

    assert!(result.contains("pub struct ProductDb"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub name: String"));
    assert!(result.contains("impl TryFrom<ProductDb> for Product"));
    assert!(result.contains("impl From<&Product> for ProductDb"));
    assert!(result.contains("name: row.name"));
    assert!(result.contains("name: entity.name.clone()"));
}

#[test]
fn generate_infra_entity_with_custom_fields() {
    use crate::generators::infrastructure::generate_infra_entity;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "active".into(),
            field_type: "Option<bool>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_infra_entity("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("pub struct ProductDb"));
    assert!(result.contains("pub title: String"));
    assert!(result.contains("pub price: i64"));
    assert!(result.contains("pub active: Option<bool>"));
    assert!(!result.contains("pub name: String"));
    assert!(result.contains("title: row.title"));
    assert!(result.contains("price: row.price"));
    assert!(result.contains("title: entity.title.clone()"));
    assert!(result.contains("price: entity.price,"));
}

#[test]
fn create_table_sql_backward_compat() {
    use crate::generators::infrastructure::create_table_sql;

    let result = create_table_sql("product", &[]);

    assert!(result.contains("CREATE TABLE products"));
    assert!(result.contains("id UUID PRIMARY KEY"));
    assert!(result.contains("name TEXT NOT NULL"));
    assert!(result.contains("created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()"));
    assert!(result.contains("deleted BOOLEAN NOT NULL DEFAULT FALSE"));
    assert!(result.contains("deleted_at TIMESTAMPTZ"));
}

#[test]
fn create_table_sql_with_custom_fields() {
    use crate::generators::infrastructure::create_table_sql;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "description".into(),
            field_type: "Option<String>".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "tags".into(),
            field_type: "Vec<String>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = create_table_sql("product", &fields);

    assert!(result.contains("CREATE TABLE products"));
    assert!(result.contains("title TEXT NOT NULL"));
    assert!(result.contains("price BIGINT NOT NULL"));
    assert!(result.contains("description TEXT,"));
    assert!(result.contains("tags TEXT[] NOT NULL DEFAULT '{}'"));
    assert!(!result.contains("name TEXT"));
}

// ── T-05: `unique = true` reaches the migration ───────────────────────────

#[test]
fn create_table_sql_emits_unique_index_for_unique_fields() {
    use crate::generators::infrastructure::create_table_sql;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "sku".into(),
            field_type: "String".into(),
            unique: true,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            ..Default::default()
        },
    ];
    let result = create_table_sql("product", &fields);

    // The column definition stays uniform; the constraint is a named index so later migrations
    // can drop or replace it by name.
    assert!(result.contains("sku TEXT NOT NULL"));
    assert!(
        result.contains(
            "CREATE UNIQUE INDEX products_sku_key ON products (sku) WHERE deleted = false;"
        ),
        "unique field must produce a unique index, got:\n{result}"
    );
    assert!(!result.contains("price_key"));
}

#[test]
fn create_table_sql_emits_unique_index_for_unique_value_object_field() {
    use crate::generators::infrastructure::create_table_sql;
    use crate::puerto_toml::Field;

    // `sku:Sku:String!` — the VO wraps the column, the constraint applies to the column.
    let fields = vec![Field {
        name: "sku".into(),
        field_type: "String".into(),
        unique: true,
        value_object: Some("Sku".into()),
        ..Default::default()
    }];
    let result = create_table_sql("item", &fields);

    assert!(
        result.contains("CREATE UNIQUE INDEX items_sku_key ON items (sku) WHERE deleted = false;")
    );
}

#[test]
fn create_table_sql_omits_unique_index_when_no_field_is_unique() {
    use crate::generators::infrastructure::create_table_sql;

    let result = create_table_sql("product", &[]);

    assert!(!result.contains("CREATE UNIQUE INDEX"));
}

#[test]
fn generate_crud_infra_db_repository_backward_compat() {
    use crate::generators::infrastructure::generate_crud_infra_db_repository;

    let result = generate_crud_infra_db_repository("Product", "product", &[], &[]).unwrap();

    assert!(result.contains("pub struct PgProductRepository"));
    assert!(result.contains("async fn find_all"));
    assert!(result.contains("async fn find_by_id"));
    assert!(result.contains("async fn save"));
    assert!(result.contains("id, created_at, updated_at, deleted, deleted_at, name"));
    assert!(result.contains("should_persist_and_retrieve_by_id"));
    assert!(result.contains("should_list_all_products_excluding_deleted"));
    assert!(result.contains("ProductProps"));
    assert!(result.contains("name: \"example\".to_string()"));
}

#[test]
fn generate_crud_infra_db_repository_with_custom_fields() {
    use crate::generators::infrastructure::generate_crud_infra_db_repository;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_crud_infra_db_repository("Product", "product", &fields, &[]).unwrap();

    assert!(result.contains("id, created_at, updated_at, deleted, deleted_at, title, price"));
    assert!(result.contains("$1, $2, $3, $4, $5, $6, $7"));
    assert!(result.contains("title = $6, price = $7"));
    assert!(result.contains("db.title"));
    assert!(result.contains("db.price"));
    assert!(result.contains("title: \"example\".to_string()"));
    assert!(result.contains("price: 42"));
    assert!(result.contains("assert_eq!(found.title, entity.title)"));
    assert!(result.contains("assert_eq!(found.price, entity.price)"));
    assert!(result.contains("entity.title = \"updated\".to_string()"));
}

// ── Presentation generators ────────────────────────────────────────────────────

#[test]
fn generate_crud_dto_backward_compat() {
    use crate::generators::presentation::generate_crud_dto;

    let result = generate_crud_dto("Product", "product", &[]).unwrap();

    assert!(result.contains("pub struct ProductDto"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub name: String"));
    assert!(result.contains("pub fn from_domain"));
    assert!(result.contains("name: entity.name.clone()"));
    assert!(result.contains("pub struct CreateProductRequest"));
    assert!(result.contains("pub struct UpdateProductRequest"));
}

#[test]
fn generate_crud_dto_with_custom_fields() {
    use crate::generators::presentation::generate_crud_dto;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "description".into(),
            field_type: "Option<String>".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_crud_dto("Product", "product", &fields).unwrap();

    assert!(result.contains("pub struct ProductDto"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub title: String"));
    assert!(result.contains("pub price: i64"));
    assert!(result.contains("pub description: Option<String>"));
    assert!(!result.contains("pub name: String"));
    assert!(result.contains("title: entity.title.clone()"));
    assert!(result.contains("price: entity.price,"));
    assert!(result.contains("pub struct CreateProductRequest"));
    assert!(result.contains("pub struct UpdateProductRequest"));
}

/// The request DTOs feed the use case params, which feed `EntityProps` — a `Uuid` field has to
/// travel the whole way or the handler cannot build the params (E0063).
#[test]
fn generate_crud_dto_and_routes_carry_uuid_fields() {
    use crate::generators::presentation::{generate_crud_dto, generate_crud_routes};
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "category_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
    ];

    let dto = generate_crud_dto("Product", "product", &fields).unwrap();
    assert!(dto.contains("pub category_id: Uuid"));
    assert!(dto.contains("category_id: entity.category_id,"));
    let create_request = dto
        .split("pub struct CreateProductRequest")
        .nth(1)
        .expect("CreateProductRequest missing");
    assert!(
        create_request.contains("pub category_id: Uuid"),
        "CreateProductRequest must carry the Uuid field"
    );

    let routes = generate_crud_routes("Product", "product", &fields).unwrap();
    assert_eq!(
        routes.matches("category_id: body.category_id,").count(),
        2,
        "both the create and the update handler must pass the Uuid field through"
    );
}

#[test]
fn generate_crud_routes_with_custom_fields() {
    use crate::generators::presentation::generate_crud_routes;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    let result = generate_crud_routes("Product", "product", &fields).unwrap();

    assert!(result.contains("method = \"post\""));
    assert!(result.contains("method = \"get\""));
    assert!(result.contains("method = \"put\""));
    assert!(result.contains("method = \"delete\""));
    assert!(result.contains("pub struct ProductApi"));
    assert!(result.contains("title: body.title.clone()"));
    assert!(result.contains("price: body.price,"));
    assert!(result.contains("id: id.0,"));
}

#[test]
fn write_repository_files_with_fields_generates_dynamic_entity_and_repo() {
    let dir = temp_dir("repo_files_dynamic");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fields = vec![
        crate::puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        crate::puerto_toml::Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    crate::generators::infrastructure::write_repository_files(
        "Product",
        "product",
        &dir,
        true,
        &fields,
        &[],
    )
    .unwrap();

    let entity = fs::read_to_string(dir.join("infrastructure/src/product/entity.rs")).unwrap();
    assert!(entity.contains("pub title: String"));
    assert!(entity.contains("pub price: i64"));
    assert!(!entity.contains("pub name: String"));

    let repo = fs::read_to_string(dir.join("infrastructure/src/product/repository.rs")).unwrap();
    assert!(repo.contains("title, price"));
    assert!(repo.contains("PgProductRepository"));

    cleanup(&dir);
}

#[test]
fn write_presentation_files_with_fields_generates_dynamic_dto_and_routes() {
    let dir = temp_dir("pres_files_dynamic");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    let fields = vec![
        crate::puerto_toml::Field {
            name: "title".into(),
            field_type: "String".into(),
            unique: false,
            ..Default::default()
        },
        crate::puerto_toml::Field {
            name: "price".into(),
            field_type: "i64".into(),
            unique: false,
            ..Default::default()
        },
    ];
    crate::generators::presentation::write_presentation_files("Product", "product", &dir, &fields)
        .unwrap();

    let dto = fs::read_to_string(dir.join("presentation/src/api/product/dto.rs")).unwrap();
    assert!(dto.contains("pub title: String"));
    assert!(dto.contains("pub price: i64"));
    assert!(!dto.contains("pub name: String"));

    let routes = fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap();
    assert!(routes.contains("title: body.title.clone()"));
    assert!(routes.contains("price: body.price,"));

    cleanup(&dir);
}

// ── T-13: review fixes — generated content ────────────────────────────────

#[test]
fn create_impl_logs_start_and_done_without_the_payload() {
    let fields = [scaffold_test_field("title", "String")];

    let content = crate::generators::application::generate_create_use_case_impl(
        "Product",
        "product",
        &fields,
        &[],
    )
    .unwrap();

    assert!(
        content.contains("tracing::info!(\"create_product: start\");"),
        "{content}"
    );
    assert!(content.contains("tracing::info!(id = %entity.id, \"create_product: done\");"));
    // The payload is not logged: a field value is user data (and often personal data).
    assert!(!content.contains("Creating product"), "{content}");
}

#[test]
fn create_test_asserts_an_enum_vo_with_as_str() {
    let fields = [puerto_toml::parse_field_arg("state:State:enum:Open/Closed").unwrap()];

    let content = crate::generators::application::generate_create_use_case_impl(
        "Ticket",
        "ticket",
        &fields,
        &[],
    )
    .unwrap();

    assert!(!content.contains("state.value()"), "{content}");
}

#[test]
fn unique_index_ignores_soft_deleted_rows() {
    // Soft-delete keeps the row: a plain unique index made a deleted SKU impossible to reuse.
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;

    let sql = crate::generators::infrastructure::create_table_sql("product", &[sku]);

    assert!(
        sql.contains(
            "CREATE UNIQUE INDEX products_sku_key ON products (sku) WHERE deleted = false;"
        )
    );
}

#[test]
fn db_entity_maps_hashmap_columns_through_sqlx_json() {
    let fields = [scaffold_test_field("meta", "HashMap<String, String>")];

    let entity = crate::generators::infrastructure::generate_infra_entity(
        "Product",
        "product",
        &fields,
        &[],
    )
    .unwrap();
    let repo = crate::generators::infrastructure::generate_crud_infra_db_repository(
        "Product",
        "product",
        &fields,
        &[],
    )
    .unwrap();

    assert!(entity.contains("use std::collections::HashMap;"));
    assert!(entity.contains("pub meta: Json<HashMap<String, String>>,"));
    assert!(
        repo.contains(r#"meta as \"meta: Json<HashMap<String, String>>\""#),
        "{repo}"
    );
}

#[test]
fn db_project_enables_sqlx_json_and_commits_the_offline_cache() {
    let dir = temp_dir("db_project_sqlx");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("db-project-sqlx", &dir).unwrap();
    crate::generators::project::apply_base_env(&output).unwrap();

    crate::generators::project::apply_db_to_new_project(&output).unwrap();

    let infra = fs::read_to_string(output.join("infrastructure/Cargo.toml")).unwrap();
    assert!(infra.contains("\"json\""), "{infra}");
    // `.cargo/config.toml` sets SQLX_OFFLINE=true: an ignored `.sqlx/` means no clone builds.
    let gitignore = fs::read_to_string(output.join(".gitignore")).unwrap();
    assert!(!gitignore.contains(".sqlx"), "{gitignore}");

    cleanup(&dir);
}

#[test]
fn pg_repository_tests_import_what_their_seed_names() {
    let fields = [
        scaffold_test_field("published_at", "DateTime<Utc>"),
        puerto_toml::parse_field_arg("status:Status:enum:Active/Inactive").unwrap(),
    ];

    let repo = crate::generators::infrastructure::generate_crud_infra_db_repository(
        "Product",
        "product",
        &fields,
        &[],
    )
    .unwrap();

    assert!(repo.contains("use business::domain::product::value_objects::Status;"));
    assert!(
        !repo.contains("Utc::now()"),
        "a ns-precision timestamp cannot round-trip Postgres"
    );
}

#[test]
fn pg_repository_seed_varies_unique_fields() {
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;

    let repo = crate::generators::infrastructure::generate_crud_infra_db_repository(
        "Product",
        "product",
        &[sku],
        &[],
    )
    .unwrap();

    assert!(repo.contains("sku: Uuid::new_v4().to_string(),"), "{repo}");
}
