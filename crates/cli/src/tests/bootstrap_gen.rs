use super::*;

// ── T-03: bootstrap with a use-case-less entity ───────────────────────────

#[test]
fn bootstrap_with_entity_without_use_cases_is_syntactically_valid() {
    // An entity can reach puerto.toml with no use cases (hand-edited manifest; `puerto validate`
    // only warns). Joining an empty use-case list used to emit `EntityApi { , … }`.
    let entities = vec![puerto_toml::Entity {
        name: "Invoice".to_string(),
        use_cases: vec![],
        db: false,
        fields: vec![],
    }];

    let content =
        crate::generators::bootstrap::generate_bootstrap_content(&entities, false).unwrap();

    assert!(!content.contains("{ ,"));
    assert!(content.contains("let invoice_api = InvoiceApi {};"));
}

#[test]
fn bootstrap_with_use_cases_lists_them_in_the_api_initializer() {
    let entities = vec![puerto_toml::Entity {
        name: "Product".to_string(),
        use_cases: vec!["create_product".into(), "get_product".into()],
        db: false,
        fields: vec![],
    }];

    let content =
        crate::generators::bootstrap::generate_bootstrap_content(&entities, false).unwrap();

    assert!(content.contains("let product_api = ProductApi { create_product, get_product };"));
}

// ── Phase 9.1: the OpenAPI spec export ────────────────────────────────────

fn product_with_crud() -> Vec<puerto_toml::Entity> {
    vec![puerto_toml::Entity {
        name: "Product".to_string(),
        use_cases: vec!["create_product".into(), "list_products".into()],
        db: false,
        fields: vec![],
    }]
}

#[test]
fn web_bootstrap_exports_the_spec_from_mocked_use_cases() {
    let content =
        crate::generators::bootstrap::generate_bootstrap_content(&product_with_crud(), true)
            .unwrap();

    assert!(content.contains("fn openapi_spec_matches_docs_api_openapi_json()"));
    assert!(content.contains(
        "use business::domain::product::use_cases::list_products::mocks::MockListProductsUseCase;"
    ));
    assert!(content.contains("list_products: Arc::new(MockListProductsUseCase::new())"));
    assert!(content.contains("UPDATE_OPENAPI"));
    assert!(content.contains("docs/api/openapi.json"));
}

#[test]
fn server_and_spec_test_build_the_openapi_service_the_same_way() {
    // Two `OpenApiService::new` calls could disagree (title, version, API order) and the exported
    // spec would describe an API the server does not serve.
    let content =
        crate::generators::bootstrap::generate_bootstrap_content(&product_with_crud(), true)
            .unwrap();

    assert_eq!(
        content.matches("OpenApiService::new(").count(),
        1,
        "{content}"
    );
    assert_eq!(
        content.matches("openapi_service(product_api)").count(),
        2,
        "{content}"
    );
}

#[test]
fn bootstrap_without_web_has_no_spec_test() {
    let content =
        crate::generators::bootstrap::generate_bootstrap_content(&product_with_crud(), false)
            .unwrap();

    assert!(!content.contains("#[cfg(test)]"), "{content}");
    assert!(!content.contains("mocks::"), "{content}");
}

#[test]
fn spec_test_of_an_api_without_entities_uses_the_empty_api() {
    let content = crate::generators::bootstrap::generate_bootstrap_content(&[], true).unwrap();

    assert!(content.contains("openapi_service(()).spec()"), "{content}");
}

#[test]
fn no_web_project_bootstrap_drops_the_spec_test() {
    let dir = temp_dir("no_web_bootstrap");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("no-web-bootstrap", &dir).unwrap();

    scaffold::apply_no_web(&output).unwrap();

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(!bootstrap.contains("openapi_spec_matches"), "{bootstrap}");
    cleanup(&dir);
}

#[test]
fn web_project_syncs_the_spec_into_typescript_types() {
    let dir = temp_dir("web_sync");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("web-sync", &dir).unwrap();

    let makefile = fs::read_to_string(output.join("Makefile")).unwrap();
    assert!(makefile.contains("web/sync:"), "{makefile}");
    assert!(makefile.contains("UPDATE_OPENAPI=1"), "{makefile}");
    assert!(makefile.contains("run openapi:types"), "{makefile}");

    let package = fs::read_to_string(output.join("web/package.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&package).unwrap();
    let script = json["scripts"]["openapi:types"].as_str().unwrap();
    assert!(script.contains("../docs/api/openapi.json"), "{script}");
    assert!(
        script.contains("src/infrastructure/http/schema.d.ts"),
        "{script}"
    );
    assert!(json["dependencies"]["openapi-fetch"].is_string());
    assert!(json["devDependencies"]["openapi-typescript"].is_string());
    cleanup(&dir);
}
