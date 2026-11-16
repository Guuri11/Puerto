use super::*;

// ── T-36: patchers — exact output (killed mutants) ────────────────────────

#[test]
fn routes_patch_appends_the_field_as_the_last_member_of_the_api_struct() {
    let dir = temp_dir("routes_patch_position");
    cleanup(&dir);
    fs::create_dir_all(dir.join("contextos/producto/rest/src")).unwrap();
    fs::write(
        dir.join("contextos/producto/rest/src/rutas.rs"),
        "use std::sync::Arc;\n\npub struct ApiProducto {\n    pub obtener_producto: Arc<dyn ObtenerProducto>,\n}\n\nimpl ApiProducto {}\n",
    )
    .unwrap();

    crate::patchers::routes_rs::patch_routes_use_cases(
        &dir,
        "Producto",
        "producto",
        &["archive_producto".to_string()],
    )
    .unwrap();

    assert_eq!(
        fs::read_to_string(dir.join("contextos/producto/rest/src/rutas.rs")).unwrap(),
        "use aplicacion_producto::archive_producto::ArchiveProducto;\n\
         use std::sync::Arc;\n\n\
         pub struct ApiProducto {\n    pub obtener_producto: Arc<dyn ObtenerProducto>,\n    \
         /// Lo cablea bootstrap.rs. Llámalo desde un endpoint en el impl `#[OpenApi]` de abajo.\n    \
         #[allow(dead_code)]\n    pub archive_producto: Arc<dyn ArchiveProducto>,\n}\n\n\
         impl ApiProducto {}\n"
    );
    cleanup(&dir);
}

#[test]
fn routes_patch_gives_the_route_tests_api_literal_an_unexpecting_mock() {
    // The route tests build `{Entity}Api` with a struct literal: without the new field it stops
    // compiling (E0063). The mock is spelled out in full — its module only exists under test.
    let dir = temp_dir("routes_patch_test_literal");
    cleanup(&dir);
    fs::create_dir_all(dir.join("contextos/producto/rest/src")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ApiProducto {\n    pub obtener_producto: Arc<dyn ObtenerProducto>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    fn client() {\n        let _ = Mocks {};\n        let api = ApiProducto {\n            \
                  obtener_producto: Arc::new(self.obtener),\n        };\n    }\n}\n";
    fs::write(dir.join("contextos/producto/rest/src/rutas.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Producto",
            "producto",
            &["archive_producto".to_string()],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(
        after.contains(
            "        let api = ApiProducto {\n            obtener_producto: Arc::new(self.obtener),\n            \
             archive_producto: Arc::new(\n                aplicacion_producto::archive_producto::dobles::MockArchiveProducto::new(),\n            ),\n        };\n"
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
    fs::create_dir_all(dir.join("contextos/producto/rest/src")).unwrap();
    let before = "use std::sync::Arc;\n\npub struct ApiProducto {\n    pub obtener_producto: Arc<dyn ObtenerProducto>,\n}\n\n\
                  #[cfg(test)]\nmod tests {\n    #[derive(Default)]\n    struct Dobles {\n        obtener: MockObtenerProducto,\n    }\n\n    \
                  impl Dobles {\n        fn client(self) {\n            let api = ApiProducto {\n                \
                  obtener_producto: Arc::new(self.obtener),\n            };\n        }\n    }\n}\n";
    fs::write(dir.join("contextos/producto/rest/src/rutas.rs"), before).unwrap();

    for _ in 0..2 {
        crate::patchers::routes_rs::patch_routes_use_cases(
            &dir,
            "Producto",
            "producto",
            &[
                "obtener_producto".to_string(),
                "archive_producto".to_string(),
            ],
        )
        .unwrap();
    }

    let after = fs::read_to_string(dir.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(
        after.contains(
            // `get_producto` is already in the literal (as `self.obtener`): no second mock for it.
            "    struct Dobles {\n        obtener: MockObtenerProducto,\n        \
             archive_producto: aplicacion_producto::archive_producto::dobles::MockArchiveProducto,\n    }\n"
        ),
        "{after}"
    );
    assert!(
        after.contains(
            "            let api = ApiProducto {\n                obtener_producto: Arc::new(self.obtener),\n                \
             archive_producto: Arc::new(self.archive_producto),\n            };\n"
        ),
        "{after}"
    );
    cleanup(&dir);
}
