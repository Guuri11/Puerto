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

    crate::patchers::lib_rs::patch_infra_lib(&dir, "product", true).unwrap();

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
        "use std::sync::Arc;\n\npub struct ProductApi {\n    pub get_product: Arc<GetProductUseCaseImpl>,\n}\n\nimpl ProductApi {}\n",
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
        "use business::application::product::archive_product::ArchiveProductUseCaseImpl;\n\
         use std::sync::Arc;\n\n\
         pub struct ProductApi {\n    pub get_product: Arc<GetProductUseCaseImpl>,\n    \
         /// Wired by bootstrap.rs. Call it from an endpoint in the `#[OpenApi]` impl below.\n    \
         #[allow(dead_code)]\n    pub archive_product: Arc<ArchiveProductUseCaseImpl>,\n}\n\n\
         impl ProductApi {}\n"
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
