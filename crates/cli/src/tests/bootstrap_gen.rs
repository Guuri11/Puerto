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

    let content = crate::generators::bootstrap::generate_bootstrap_content(&entities);

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

    let content = crate::generators::bootstrap::generate_bootstrap_content(&entities);

    assert!(content.contains("let product_api = ProductApi { create_product, get_product };"));
}
