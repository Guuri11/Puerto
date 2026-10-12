use super::*;

// ── T-36: patchers — exact output (killed mutants) ────────────────────────

#[test]
fn api_rs_patch_separates_declarations_when_the_file_lacks_a_trailing_newline() {
    let dir = temp_dir("api_rs_no_newline");
    cleanup(&dir);
    fs::create_dir_all(dir.join("presentation/src")).unwrap();
    fs::write(dir.join("presentation/src/api.rs"), "pub mod error;").unwrap();

    crate::patchers::api_rs::patch_api_rs(&dir, "product").unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("presentation/src/api.rs")).unwrap(),
        "pub mod error;\npub mod tags;\npub mod product;\n"
    );
    cleanup(&dir);
}

#[test]
fn should_remove_only_the_entity_value_objects_line_when_unpatching_lib_rs() {
    // `customer` loses its local VOs; `product`, declared after it, keeps its own.
    let dir = temp_dir("unpatch_value_objects");
    cleanup(&dir);
    fs::create_dir_all(dir.join("business/src")).unwrap();
    fs::write(
        dir.join("business/src/lib.rs"),
        "pub mod domain {\n    pub mod customer {\n        pub mod model;\n        pub mod value_objects;\n    }\n    pub mod product {\n        pub mod value_objects;\n    }\n}\n",
    )
    .unwrap();

    crate::patchers::lib_rs::unpatch_business_lib_value_objects(&dir, "customer").unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("business/src/lib.rs")).unwrap(),
        "pub mod domain {\n    pub mod customer {\n        pub mod model;\n    }\n    pub mod product {\n        pub mod value_objects;\n    }\n}\n"
    );
    cleanup(&dir);
}

#[test]
fn insert_before_block_end_skips_nested_blocks() {
    let source = "pub mod domain {\n    pub mod greeting {\n        pub mod model;\n    }\n}\n";

    let patched =
        crate::patchers::lib_rs::insert_before_block_end(source, "domain", "    pub mod shared;\n")
            .unwrap();

    assert_eq!(
        patched,
        "pub mod domain {\n    pub mod greeting {\n        pub mod model;\n    }\n    pub mod shared;\n}\n"
    );
}

#[test]
fn infra_lib_patch_appends_on_its_own_line_and_declares_db_once() {
    let dir = temp_dir("infra_lib_patch");
    cleanup(&dir);
    fs::create_dir_all(dir.join("infrastructure/src")).unwrap();
    fs::write(dir.join("infrastructure/src/lib.rs"), "pub mod db;").unwrap();

    crate::patchers::lib_rs::patch_infra_lib(&dir, "product", true, true).unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("infrastructure/src/lib.rs")).unwrap(),
        "pub mod db;\npub mod product {\n    pub mod entity;\n    pub mod repository;\n}\n"
    );
    cleanup(&dir);
}

#[test]
fn value_objects_module_is_declared_per_entity_even_when_another_entity_has_one() {
    let dir = temp_dir("vo_module_per_entity");
    cleanup(&dir);
    fs::create_dir_all(dir.join("business/src")).unwrap();
    fs::write(
        dir.join("business/src/lib.rs"),
        "pub mod domain {\n    pub mod order {\n        pub mod model;\n        pub mod value_objects;\n    }\n    pub mod product {\n        pub mod model;\n    }\n}\n",
    )
    .unwrap();

    crate::patchers::lib_rs::patch_business_lib_value_objects(&dir, "product").unwrap();
    crate::patchers::lib_rs::patch_business_lib_value_objects(&dir, "product").unwrap();

    let lib = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
    let product_block = &lib[lib.find("pub mod product {").unwrap()..];
    assert_eq!(
        product_block.matches("pub mod value_objects;").count(),
        1,
        "{lib}"
    );
    assert_eq!(lib.matches("pub mod value_objects;").count(), 2, "{lib}");
    cleanup(&dir);
}

#[test]
fn routes_patch_appends_the_field_as_the_last_member_of_the_api_struct() {
    let dir = temp_dir("routes_patch_position");
    cleanup(&dir);
    fs::create_dir_all(dir.join("presentation/src/api/product")).unwrap();
    fs::write(
        dir.join("presentation/src/api/product/routes.rs"),
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
        fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap(),
        "use business::domain::product::use_cases::archive_product::ArchiveProductUseCaseTrait;\n\
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
    fs::create_dir_all(dir.join("presentation/src/api/product")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    fn client() {\n        let _ = Mocks {};\n        let api = ProductApi {\n            \
                  get_product: Arc::new(self.get),\n        };\n    }\n}\n";
    fs::write(dir.join("presentation/src/api/product/routes.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Product",
            "product",
            &["archive_product".to_string()],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap();
    assert!(
        after.contains(
            "        let api = ProductApi {\n            get_product: Arc::new(self.get),\n            \
             archive_product: Arc::new(\n                business::domain::product::use_cases::archive_product::mocks::MockArchiveProductUseCase::new(),\n            ),\n        };\n"
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
    fs::create_dir_all(dir.join("presentation/src/api/product")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<dyn GetProductUseCaseTrait>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    #[derive(Default)]\n    struct Mocks {\n        get: MockGetProductUseCase,\n    }\n\n    \
                  impl Mocks {\n        fn client(self) {\n            let api = ProductApi {\n                \
                  get_product: Arc::new(self.get),\n            };\n        }\n    }\n}\n";
    fs::write(dir.join("presentation/src/api/product/routes.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Product",
            "product",
            &["get_product".to_string(), "archive_product".to_string()],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("presentation/src/api/product/routes.rs")).unwrap();
    assert!(
        after.contains(
            // `get_product` is already in the literal (as `self.get`): no second mock for it.
            "    struct Mocks {\n        get: MockGetProductUseCase,\n        \
             archive_product: business::domain::product::use_cases::archive_product::mocks::MockArchiveProductUseCase,\n    }\n"
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

#[test]
fn api_rs_patch_adds_no_blank_lines_to_a_well_formed_or_empty_file() {
    let dir = temp_dir("api_rs_well_formed");
    cleanup(&dir);
    fs::create_dir_all(dir.join("presentation/src")).unwrap();
    let api_rs = dir.join("presentation/src/api.rs");

    fs::write(&api_rs, "pub mod error;\n").unwrap();
    crate::patchers::api_rs::patch_api_rs(&dir, "product").unwrap();
    assert_eq!(
        fs::read_to_string(&api_rs).unwrap(),
        "pub mod error;\npub mod tags;\npub mod product;\n"
    );

    fs::write(&api_rs, "pub mod error;\npub mod tags;").unwrap();
    crate::patchers::api_rs::patch_api_rs(&dir, "product").unwrap();
    assert_eq!(
        fs::read_to_string(&api_rs).unwrap(),
        "pub mod error;\npub mod tags;\npub mod product;\n"
    );

    fs::write(&api_rs, "").unwrap();
    crate::patchers::api_rs::patch_api_rs(&dir, "product").unwrap();
    assert_eq!(
        fs::read_to_string(&api_rs).unwrap(),
        "pub mod tags;\npub mod product;\n"
    );
    cleanup(&dir);
}

#[test]
fn value_objects_check_stops_at_the_end_of_its_own_entity_block() {
    // The entity being patched comes first; a later entity's `value_objects` must not count.
    let dir = temp_dir("vo_module_block_end");
    cleanup(&dir);
    fs::create_dir_all(dir.join("business/src")).unwrap();
    fs::write(
        dir.join("business/src/lib.rs"),
        "pub mod domain {\n    pub mod product {\n        pub mod model;\n    }\n    pub mod order {\n        pub mod value_objects;\n    }\n}\n",
    )
    .unwrap();

    crate::patchers::lib_rs::patch_business_lib_value_objects(&dir, "product").unwrap();

    let lib = fs::read_to_string(dir.join("business/src/lib.rs")).unwrap();
    let product_block =
        &lib[lib.find("pub mod product {").unwrap()..lib.find("pub mod order {").unwrap()];
    assert!(product_block.contains("pub mod value_objects;"), "{lib}");
    cleanup(&dir);
}
