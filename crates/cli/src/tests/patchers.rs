use super::*;

// ── T-36: patchers — exact output (killed mutants) ────────────────────────

#[test]
fn routes_patch_appends_the_field_as_the_last_member_of_the_api_struct() {
    let dir = temp_dir("routes_patch_position");
    cleanup(&dir);
    fs::create_dir_all(dir.join("contextos/product/rest/src")).unwrap();
    fs::write(
        dir.join("contextos/product/rest/src/rutas.rs"),
        "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n}\n\nimpl ProductApi {}\n",
    )
    .unwrap();

    crate::patchers::routes_rs::patch_routes_use_cases(
        &dir,
        "Product",
        "product",
        &["archive_product".to_string()],
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("contextos/product/rest/src/rutas.rs")).unwrap(),
        "use aplicacion_product::archive_product::ArchiveProductUseCaseTrait;\n\
         use std::sync::Arc;\n\n\
         pub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n    \
         /// Wired by bootstrap.rs. Call it from an endpoint in the `#[OpenApi]` impl below.\n    \
         #[allow(dead_code)]\n    pub archive_product: Arc<dyn ArchiveProductUseCaseTrait>,\n}\n\n\
         impl ProductApi {}\n"
    );
    cleanup(&dir);
}

#[test]
fn routes_patch_gives_the_route_tests_api_literal_an_unexpecting_mock() {
    // The route tests build `{Entity}Api` with a struct literal: without the new field it stops
    // compiling (E0063). The mock is spelled out in full — its module only exists under test.
    let dir = temp_dir("routes_patch_test_literal");
    cleanup(&dir);
    fs::create_dir_all(dir.join("contextos/product/rest/src")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    fn client() {\n        let _ = Mocks {};\n        let api = ProductApi {\n            \
                  get_product: Arc::new(self.get),\n        };\n    }\n}\n";
    fs::write(dir.join("contextos/product/rest/src/rutas.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Product",
            "product",
            &["archive_product".to_string()],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("contextos/product/rest/src/rutas.rs")).unwrap();
    assert!(
        after.contains(
            "        let api = ProductApi {\n            get_product: Arc::new(self.get),\n            \
             archive_product: Arc::new(\n                aplicacion_product::archive_product::mocks::MockArchiveProductUseCase::new(),\n            ),\n        };\n"
        ),
        "{after}"
    );
    cleanup(&dir);
}

#[test]
fn routes_patch_gives_the_route_tests_mocks_a_field_for_the_custom_use_case() {
    // Phase 15.1: a route test for the custom use case's endpoint sets expectations on
    // `mocks.<action>` (the `poem-route-test` snippet), like the CRUD tests do on `mocks.create`.
    let dir = temp_dir("routes_patch_mocks_field");
    cleanup(&dir);
    fs::create_dir_all(dir.join("contextos/product/rest/src")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    #[derive(Default)]\n    struct Mocks {\n        get: MockGetProductUseCase,\n    }\n\n    \
                  impl Mocks {\n        fn client(self) {\n            let api = ProductApi {\n                \
                  get_product: Arc::new(self.get),\n            };\n        }\n    }\n}\n";
    fs::write(dir.join("contextos/product/rest/src/rutas.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Product",
            "product",
            &["get_product".to_string(), "archive_product".to_string()],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("contextos/product/rest/src/rutas.rs")).unwrap();
    assert!(
        after.contains(
            // `get_product` is already in the literal (as `self.get`): no second mock for it.
            "    struct Mocks {\n        get: MockGetProductUseCase,\n        \
             archive_product: aplicacion_product::archive_product::mocks::MockArchiveProductUseCase,\n    }\n"
        ),
        "{after}"
    );
    assert!(
        after.contains(
            "            let api = ProductApi {\n                get_product: Arc::new(self.get),\n                \
             archive_product: Arc::new(self.archive_product),\n            };\n"
        ),
        "{after}"
    );
    cleanup(&dir);
}
