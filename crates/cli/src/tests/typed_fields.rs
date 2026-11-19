use super::*;

// ── puerto.toml fields ────────────────────────────────────────────────────

#[test]
fn parse_puerto_toml_with_fields() {
    let dir = temp_dir("toml_fields_parse");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "my-app"

[[entidad]]
nombre = "Producto"
casos_de_uso = ["crear_producto"]

[[entidad.campos]]
nombre = "name"
tipo = "String"

[[entidad.campos]]
nombre = "price"
tipo = "i64"

[[entidad.campos]]
nombre = "sku"
tipo = "String"
unico = true

[[entidad.campos]]
nombre = "description"
tipo = "Option<String>"
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.entity.len(), 1);
    let producto = &config.entity[0];
    assert_eq!(producto.name, "Producto");
    assert_eq!(producto.fields.len(), 4);
    assert_eq!(producto.fields[0].name, "name");
    assert_eq!(producto.fields[0].field_type, "String");
    assert!(!producto.fields[0].unique);
    assert_eq!(producto.fields[1].name, "price");
    assert_eq!(producto.fields[1].field_type, "i64");
    assert_eq!(producto.fields[2].name, "sku");
    assert!(producto.fields[2].unique);
    assert_eq!(producto.fields[3].name, "description");
    assert_eq!(producto.fields[3].field_type, "Option<String>");

    cleanup(&dir);
}

#[test]
fn parse_puerto_toml_without_fields_backwards_compat() {
    let dir = temp_dir("toml_no_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "my-app"

[[entidad]]
nombre = "Saludo"
casos_de_uso = ["obtener_saludo"]
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(config.entity.len(), 1);
    let saludo = &config.entity[0];
    assert_eq!(saludo.name, "Saludo");
    assert!(saludo.fields.is_empty());

    cleanup(&dir);
}

#[test]
fn add_entity_with_fields() {
    let dir = temp_dir("toml_add_entity_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "my-app"
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
        "Producto",
        vec!["crear_producto".into()],
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
    assert!(err.contains("tipo de campo no admitido 'FooBar'"));
}

// ── Phase 2: Domain model with custom fields ──────────────────────────────────

#[test]
fn generate_model_backward_compat_no_fields() {
    use crate::generators::domain::generate_model;

    let result = generate_model("Producto", "producto", &[], &[], false).unwrap();
    assert!(result.contains("pub struct PropsProducto"));
    assert!(result.contains("pub nombre: String,"));
    assert!(result.contains("pub struct Producto"));
    assert!(result.contains("pub id: IdProducto,"));
    assert!(result.contains("pub nombre: String,"));
    assert!(result.contains("if props.nombre.trim().is_empty()"));
    assert!(result.contains("ErrorProducto::Validacion(\"nombre_vacio\""));
    assert!(result.contains("deberia_crear_producto_cuando_el_nombre_es_valido"));
    assert!(result.contains("deberia_rechazar_producto_cuando_nombre_es_una_cadena_vacia"));
    assert!(result.contains("deberia_rechazar_producto_cuando_nombre_solo_tiene_espacios"));
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
    let result = generate_model("Producto", "producto", &fields, &[], false).unwrap();

    assert!(result.contains("pub struct PropsProducto"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("pub struct Producto"));
    assert!(result.contains("pub id: IdProducto,"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("if props.title.trim().is_empty()"));
    assert!(result.contains("title_vacio"));
    assert!(result.contains("deberia_crear_producto_cuando_los_campos_son_validos"));
    assert!(result.contains("deberia_rechazar_producto_cuando_title_es_una_cadena_vacia"));
    assert!(result.contains("deberia_rechazar_producto_cuando_title_solo_tiene_espacios"));
    // canasta wrote its rules twice (`new` and the update use case): both go through `validar`.
    assert!(result.contains("fn validar(props: &PropsProducto) -> Result<(), ErrorProducto> {"));
    assert!(result.contains(
        "pub fn actualizar(&mut self, props: PropsProducto) -> Result<(), ErrorProducto> {"
    ));
    assert_eq!(
        result.matches("Self::validar(&props)?;").count(),
        2,
        "{result}"
    );
    assert!(result.contains("self.title = props.title;"));
    assert!(result.contains("self.actualizado_en = chrono::Utc::now();"));
    assert!(
        result
            .contains("fn deberia_rechazar_actualizar_producto_cuando_title_es_una_cadena_vacia()")
    );
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
        name: "categoria_id".into(),
        field_type: "Uuid".into(),
        unique: false,
        ..Default::default()
    }];
    let result = generate_model("Item", "item", &fields, &[], false).unwrap();

    assert!(result.contains("use uuid::Uuid;"));
    assert!(result.contains("pub categoria_id: Uuid,"));
    assert!(result.contains("deberia_crear_item_cuando_los_campos_son_validos"));
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
    let result = generate_model("Config", "config", &fields, &[], false).unwrap();

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
    let result = generate_model("Producto", "producto", &fields, &[], false).unwrap();

    assert!(result.contains("if props.name.trim().is_empty()"));
    assert!(result.contains("name_vacio"));
    assert!(result.contains("if props.sku.trim().is_empty()"));
    assert!(result.contains("sku_vacio"));
    assert!(result.contains("deberia_rechazar_producto_cuando_name_es_una_cadena_vacia"));
    assert!(result.contains("deberia_rechazar_producto_cuando_sku_es_una_cadena_vacia"));
    assert!(
        !result.contains("if props.price.trim().is_empty()"),
        "non-String types should not have empty checks"
    );
}

#[test]
fn generate_mother_backward_compat_no_fields() {
    use crate::generators::domain::generate_mother;

    let result = generate_mother("Producto", "producto", &[], &[]).unwrap();
    assert!(result.contains("pub struct MadreProducto"));
    assert!(result.contains("nombre: Option<String>,"));
    assert!(result.contains("pub fn con_nombre(mut self, nombre: &str) -> Self"));
    assert!(result.contains("pub fn con_nombre_vacio(mut self) -> Self"));
    assert!(result.contains("Producto::new(PropsProducto"));
    assert!(result.contains("self.nombre.unwrap_or_else(|| \"example\".to_string())"));
    assert!(result.contains("pub fn aleatorio()"));
    assert!(result.contains("pub fn aleatorios"));
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
    let result = generate_mother("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("title: Option<String>,"));
    assert!(result.contains("price: Option<i64>,"));
    assert!(result.contains("description: Option<String>,"));
    assert!(result.contains("pub fn con_title(mut self, title: &str) -> Self"));
    assert!(result.contains("self.title = Some(title.to_string())"));
    assert!(result.contains("pub fn con_price(mut self, price: i64) -> Self"));
    assert!(result.contains("self.price = Some(price)"));
    assert!(result.contains("pub fn con_description(mut self, description: &str) -> Self"));
    assert!(result.contains("self.description = Some(description.to_string())"));
    assert!(result.contains("pub fn con_title_vacio(mut self) -> Self"));
    assert!(result.contains("self.title = Some(String::new())"));
    assert!(result.contains("self.price.unwrap_or(42)"));
    assert!(result.contains("self.description"));
    assert!(result.contains("pub fn aleatorio()"));
    assert!(result.contains("pub fn aleatorios"));
}

#[test]
fn scaffold_with_empty_fields_produces_backward_compat_model() {
    let dir = temp_dir("scaffold_empty_fields");
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/producto/dominio/src/modelo.rs")).unwrap();
    assert!(content.contains("pub struct PropsProducto"));
    assert!(content.contains("pub nombre: String,"));
    assert!(content.contains("pub struct Producto"));
    assert!(content.contains("ErrorProducto::Validacion(\"nombre_vacio\""));
    assert!(content.contains("deberia_crear_producto_cuando_el_nombre_es_valido"));

    let mother =
        fs::read_to_string(dir.join("contextos/producto/dominio/src/pruebas/madres.rs")).unwrap();
    assert!(mother.contains("pub struct MadreProducto"));
    assert!(mother.contains("con_nombre"));

    cleanup(&dir);
}

#[test]
fn scaffold_crud_with_empty_fields_produces_backward_compat_model() {
    let dir = temp_dir("scaffold_crud_empty_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);
    scaffold::run("Producto", &dir, false, &[], &[]).unwrap();

    let content = fs::read_to_string(dir.join("contextos/producto/dominio/src/modelo.rs")).unwrap();
    assert!(content.contains("pub struct PropsProducto"));
    assert!(content.contains("pub nombre: String,"));

    let mother =
        fs::read_to_string(dir.join("contextos/producto/dominio/src/pruebas/madres.rs")).unwrap();
    assert!(mother.contains("MadreProducto"));
    assert!(mother.contains("con_nombre"));

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
    let result = generate_create_use_case_trait("Producto", "producto", &fields).unwrap();

    assert!(result.contains("pub struct ParamsCrearProducto"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub description: Option<String>,"));
    assert!(result.contains("pub trait CrearProducto"));
    assert!(
        !result.contains("pub name: String,"),
        "should not contain hardcoded 'name' field"
    );
}

#[test]
fn generate_create_use_case_trait_backward_compat_no_fields() {
    use crate::generators::domain::generate_create_use_case_trait;

    let result = generate_create_use_case_trait("Producto", "producto", &[]).unwrap();

    assert!(result.contains("pub struct ParamsCrearProducto"));
    assert!(result.contains("pub nombre: String,"));
    assert!(result.contains("pub trait CrearProducto"));
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
    let result = generate_update_use_case_trait("Producto", "producto", &fields).unwrap();

    assert!(result.contains("pub struct ParamsActualizarProducto"));
    assert!(result.contains("pub id: IdProducto,"));
    assert!(result.contains("pub title: String,"));
    assert!(result.contains("pub price: i64,"));
    assert!(result.contains("pub trait ActualizarProducto"));
}

/// A `Uuid` field is an ordinary field — `model.tera` puts it in `EntityProps`, so every params
/// struct that feeds `Entity::new` must carry it too. Dropping it (as the generators used to)
/// produced `missing field 'categoria_id' in initializer of ItemProps` (E0063).
#[test]
fn generate_update_use_case_trait_includes_uuid_fields() {
    use crate::generators::domain::generate_update_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "categoria_id".into(),
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
    assert!(result.contains("pub id: IdItem,"));
    assert!(result.contains("pub categoria_id: Uuid,"));
    // The id is `ItemId`; the `Uuid` field brings the one `use uuid::Uuid;` (no duplicate: E0252).
    assert_eq!(result.matches("use uuid::Uuid;").count(), 1);
}

#[test]
fn generate_create_use_case_trait_includes_uuid_fields() {
    use crate::generators::domain::generate_create_use_case_trait;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "categoria_id".into(),
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
    assert!(result.contains("pub categoria_id: Uuid,"));
    assert!(
        !result.contains("pub id: "),
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
    let result = generate_create_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("CrearProductoImpl"));
    assert!(result.contains("ParamsCrearProducto"));
    assert!(result.contains("title: params.title,"));
    assert!(result.contains("price: params.price,"));
    assert!(result.contains("Producto::new(PropsProducto"));
    assert!(result.contains(
        "#[tracing::instrument(name = \"crear_producto\", skip_all, err(level = \"warn\"))]"
    ));
}

/// ADR-0026: the soft delete is the entity's own rule, not the use case's.
#[test]
fn model_without_events_marks_its_own_soft_delete() {
    use crate::generators::domain::generate_model;

    let model = generate_model("Producto", "producto", &[], &[], false).unwrap();

    assert!(model.contains("pub fn borrar(&mut self) {"), "{model}");
    assert!(model.contains("self.borrado = true;"), "{model}");
    assert!(!model.contains("pub fn crear("), "{model}");
    assert!(!model.contains("Evento"), "{model}");

    let delete = crate::generators::application::generate_delete_use_case_impl(
        "Producto",
        "producto",
        &[],
        &[],
    )
    .unwrap();
    assert!(delete.contains("entidad.borrar();"), "{delete}");
    assert!(!delete.contains("borrado = true"), "{delete}");
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
    let result = generate_update_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("ActualizarProductoImpl"));
    assert!(result.contains("ParamsActualizarProducto"));
    // The model's rules, not a copy of them: `entidad.actualizar` runs what `new` runs.
    assert!(result.contains(".actualizar(PropsProducto {"), "{result}");
    assert!(result.contains("title: params.title,"));
    assert!(result.contains("price: params.price,"));
    let implementation = &result[..result.find("#[cfg(test)]").unwrap()];
    assert!(!implementation.contains("trim().is_empty()"), "{result}");
    assert!(!implementation.contains("entidad.title ="), "{result}");
}

#[test]
fn generate_update_use_case_impl_assigns_uuid_fields() {
    use crate::generators::application::generate_update_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "categoria_id".into(),
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

    assert!(result.contains("name: params.name,"));
    assert!(result.contains("categoria_id: params.categoria_id,"));
    // The impl names no primitive type and the tests write no literal (the mother's values): a
    // `Uuid` import would be unused, which `-D warnings` rejects.
    assert!(!result.contains("use uuid::Uuid;"), "{result}");
}

#[test]
fn generate_create_use_case_impl_passes_uuid_fields_to_props() {
    use crate::generators::application::generate_create_use_case_impl;
    use crate::puerto_toml::Field;

    let fields = vec![
        Field {
            name: "categoria_id".into(),
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

    assert!(result.contains("categoria_id: params.categoria_id,"));
    assert!(result.contains("name: params.name,"));
    // The generated test builds the params from the mother's props: no `Uuid` literal, no import.
    assert!(result.contains("            categoria_id: props.categoria_id,"));
    assert!(!result.contains("use uuid::Uuid;"));
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
        r#"[proyecto]
nombre = "test-app"

[[entidad]]
nombre = "Producto"
casos_de_uso = ["crear_producto", "obtener_producto", "listar_productos", "actualizar_producto", "borrar_producto"]

[[entidad.campos]]
nombre = "name"
tipo = "String"

[[entidad.campos]]
nombre = "price"
tipo = "i64"
"#,
    )
    .unwrap();

    let config = puerto_toml::read(&dir).unwrap();
    let fields = config.entity[0].fields.clone();

    let pascal = "Producto";
    let snake = "producto";

    let create_trait =
        crate::generators::domain::generate_create_use_case_trait(pascal, snake, &fields).unwrap();
    assert!(create_trait.contains("pub name: String,"));
    assert!(create_trait.contains("pub price: i64,"));
    assert!(
        !create_trait.contains("pub id: "),
        "Create params should not have id"
    );

    let update_trait =
        crate::generators::domain::generate_update_use_case_trait(pascal, snake, &fields).unwrap();
    assert!(update_trait.contains("pub id: IdProducto,"));
    assert!(update_trait.contains("pub name: String,"));
    assert!(update_trait.contains("pub price: i64,"));

    cleanup(&dir);
}

#[test]
fn write_application_files_with_fields_generates_dynamic_impls() {
    let dir = temp_dir("app_files_with_fields");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

    fs::create_dir_all(dir.join("business/src/application/producto")).unwrap();

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
        "Producto",
        "producto",
        &dir,
        &fields,
        &[],
    )
    .unwrap();

    let create_content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/crear_producto.rs"))
            .unwrap();
    assert!(create_content.contains("title: params.title,"));
    assert!(create_content.contains("price: params.price,"));
    assert!(create_content.contains("Producto::new(PropsProducto"));

    let update_content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/actualizar_producto.rs"))
            .unwrap();
    assert!(update_content.contains(".actualizar(PropsProducto {"));
    assert!(update_content.contains("title: params.title,"));
    assert!(update_content.contains("price: params.price,"));

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
    let result = generate_get_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("ObtenerProductoImpl"));
    assert!(result.contains("ParamsObtenerProducto"));
    assert!(result.contains("deberia_devolver_producto_cuando_el_id_existe"));
    assert!(result.contains("deberia_devolver_no_encontrado_cuando_el_id_no_existe"));
    assert!(result.contains("MadreProducto::aleatorio"));
    assert!(!result.contains("logger"));
}

#[test]
fn generate_get_use_case_impl_backward_compat_no_fields() {
    use crate::generators::application::generate_get_use_case_impl;

    let result = generate_get_use_case_impl("Producto", "producto", &[], &[]).unwrap();

    assert!(result.contains("ObtenerProductoImpl"));
    assert!(result.contains("MadreProducto::aleatorio"));
    assert!(result.contains("deberia_devolver_producto_cuando_el_id_existe"));
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
    let result = generate_list_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("ListarProductosImpl"));
    assert!(result.contains("MadreProducto::aleatorio"));
    assert!(result.contains("deberia_devolver_la_pagina_de_productos"));
    assert!(result.contains("deberia_devolver_una_pagina_vacia_cuando_no_hay_productos"));
    assert!(result.contains("MadreProducto::aleatorios(2)"));
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
    let result = generate_delete_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("BorrarProductoImpl"));
    assert!(result.contains("MadreProducto::aleatorio"));
    assert!(result.contains("deberia_hacer_borrado_logico_de_producto_cuando_el_id_existe"));
    assert!(result.contains("deberia_devolver_no_encontrado_cuando_producto_no_existe"));
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
    let result = generate_create_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("#[cfg(test)]"));
    assert!(result.contains("deberia_crear_producto_cuando_los_campos_son_validos"));
    assert!(result.contains("deberia_devolver_error_cuando_title_es_una_cadena_vacia"));
    assert!(!result.contains("logger"));
    assert!(result.contains("MockRepositorioProducto"));
    assert!(result.contains("let props = MadreProducto::new().construir_props();"));
    assert!(result.contains("title: props.title,"));
    assert!(result.contains("price: props.price,"));
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
    let result = generate_update_use_case_impl("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("#[cfg(test)]"));
    assert!(result.contains("deberia_actualizar_producto_cuando_los_parametros_son_validos"));
    assert!(result.contains("deberia_devolver_no_encontrado_cuando_producto_no_existe"));
    assert!(result.contains("deberia_devolver_error_cuando_title_es_una_cadena_vacia"));
    assert!(result.contains("MadreProducto::aleatorio"));
    assert!(!result.contains("logger"));
    assert!(result.contains("entidad.title = \"original\".to_string();"));
    assert!(result.contains("title: props.title,"));
    assert!(result.contains("price: props.price,"));
}

#[test]
fn write_application_files_with_fields_generates_dynamic_get_list_delete() {
    let dir = temp_dir("app_files_dynamic_all");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

    fs::create_dir_all(dir.join("business/src/application/producto")).unwrap();

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
        "Producto",
        "producto",
        &dir,
        &fields,
        &[],
    )
    .unwrap();

    let get_content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/obtener_producto.rs"))
            .unwrap();
    assert!(get_content.contains("ObtenerProductoImpl"));
    assert!(get_content.contains("deberia_devolver_producto_cuando_el_id_existe"));
    assert!(get_content.contains("MadreProducto::aleatorio"));

    let list_content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/listar_productos.rs"))
            .unwrap();
    assert!(list_content.contains("ListarProductosImpl"));
    assert!(list_content.contains("deberia_devolver_la_pagina_de_productos"));
    assert!(list_content.contains("MadreProducto::aleatorios(2)"));

    let delete_content =
        fs::read_to_string(dir.join("contextos/producto/aplicacion/src/borrar_producto.rs"))
            .unwrap();
    assert!(delete_content.contains("BorrarProductoImpl"));
    assert!(
        delete_content.contains("deberia_hacer_borrado_logico_de_producto_cuando_el_id_existe")
    );
    assert!(delete_content.contains("MadreProducto::aleatorio"));

    cleanup(&dir);
}

// ── Infrastructure generators ─────────────────────────────────────────────────

#[test]
fn generate_infra_entity_backward_compat() {
    use crate::generators::infrastructure::generate_infra_entity;

    let result = generate_infra_entity("Producto", "producto", &[], &[]).unwrap();

    assert!(result.contains("pub struct FilaProducto"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub nombre: String"));
    assert!(result.contains("impl TryFrom<FilaProducto> for Producto"));
    assert!(result.contains("impl From<&Producto> for FilaProducto"));
    assert!(result.contains("nombre: fila.nombre"));
    assert!(result.contains("nombre: entidad.nombre.clone()"));
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
    let result = generate_infra_entity("Producto", "producto", &fields, &[]).unwrap();

    assert!(result.contains("pub struct FilaProducto"));
    assert!(result.contains("pub title: String"));
    assert!(result.contains("pub price: i64"));
    assert!(result.contains("pub active: Option<bool>"));
    assert!(!result.contains("pub name: String"));
    assert!(result.contains("title: fila.title"));
    assert!(result.contains("price: fila.price"));
    assert!(result.contains("title: entidad.title.clone()"));
    assert!(result.contains("price: entidad.price,"));
}

#[test]
fn create_table_sql_backward_compat() {
    use crate::generators::infrastructure::create_table_sql;

    let result = create_table_sql("producto", &[]);

    assert!(result.contains("CREATE TABLE productos"));
    assert!(result.contains("id UUID PRIMARY KEY"));
    assert!(result.contains("nombre TEXT NOT NULL"));
    assert!(result.contains("creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()"));
    assert!(result.contains("borrado BOOLEAN NOT NULL DEFAULT FALSE"));
    assert!(result.contains("borrado_en TIMESTAMPTZ"));
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
    let result = create_table_sql("producto", &fields);

    assert!(result.contains("CREATE TABLE productos"));
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
    let result = create_table_sql("producto", &fields);

    // The column definition stays uniform; the constraint is a named index so later migrations
    // can drop or replace it by name.
    assert!(result.contains("sku TEXT NOT NULL"));
    assert!(
        result.contains(
            "CREATE UNIQUE INDEX productos_sku_key ON productos (sku) WHERE borrado = false;"
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
    let result = create_table_sql("articulo", &fields);

    assert!(result.contains(
        "CREATE UNIQUE INDEX articulos_sku_key ON articulos (sku) WHERE borrado = false;"
    ));
}

#[test]
fn create_table_sql_omits_unique_index_when_no_field_is_unique() {
    use crate::generators::infrastructure::create_table_sql;

    let result = create_table_sql("producto", &[]);

    assert!(!result.contains("CREATE UNIQUE INDEX"));
}

#[test]
fn generate_crud_infra_db_repository_backward_compat() {
    use crate::generators::infrastructure::generate_crud_infra_db_repository;

    let result =
        generate_crud_infra_db_repository("Producto", "producto", &[], &[], false).unwrap();

    assert!(result.contains("pub struct RepositorioProductoPostgres"));
    assert!(result.contains("async fn buscar_todos"));
    assert!(result.contains("async fn buscar_por_id"));
    assert!(result.contains("async fn guardar"));
    assert!(result.contains("id, creado_en, actualizado_en, borrado, borrado_en, nombre"));
    assert!(result.contains("deberia_guardar_y_recuperar_por_id"));
    assert!(result.contains("deberia_listar_productos_sin_las_filas_borradas"));
    let seed = crate::generators::infrastructure::generate_pg_seed(
        "Producto",
        "producto",
        &[],
        &[],
        false,
    )
    .unwrap();
    // Nothing a stored row needs of its own: the mother's entity as it is.
    assert!(seed.contains("MadreProducto::aleatorio()"), "{seed}");
    assert!(!seed.contains("PropsProducto"), "{seed}");
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
    let result =
        generate_crud_infra_db_repository("Producto", "producto", &fields, &[], false).unwrap();

    assert!(result.contains("id, creado_en, actualizado_en, borrado, borrado_en, title, price"));
    assert!(result.contains("$1, $2, $3, $4, $5, $6, $7"));
    assert!(result.contains("title = $6, price = $7"));
    assert!(result.contains("db.title"));
    assert!(result.contains("db.price"));
    let seed = crate::generators::infrastructure::generate_pg_seed(
        "Producto",
        "producto",
        &fields,
        &[],
        false,
    )
    .unwrap();
    assert!(seed.contains("MadreProducto::aleatorio()"), "{seed}");
    assert!(!seed.contains("price: 42"), "{seed}");
    assert!(result.contains("assert_eq!(encontrado.title, entidad.title)"));
    assert!(result.contains("assert_eq!(encontrado.price, entidad.price)"));
    assert!(result.contains("entidad.title = \"updated\".to_string()"));
}

// ── Presentation generators ────────────────────────────────────────────────────

#[test]
fn generate_crud_dto_backward_compat() {
    use crate::generators::presentation::generate_crud_dto;

    let result = generate_crud_dto("Producto", "producto", &[]).unwrap();

    assert!(result.contains("pub struct DtoProducto"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub nombre: String"));
    assert!(result.contains("pub fn from_dominio"));
    assert!(result.contains("nombre: entidad.nombre.clone()"));
    assert!(result.contains("pub struct PeticionCrearProducto"));
    assert!(result.contains("pub struct PeticionActualizarProducto"));
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
    let result = generate_crud_dto("Producto", "producto", &fields).unwrap();

    assert!(result.contains("pub struct DtoProducto"));
    assert!(result.contains("pub id: Uuid"));
    assert!(result.contains("pub title: String"));
    assert!(result.contains("pub price: i64"));
    assert!(result.contains("pub description: Option<String>"));
    assert!(!result.contains("pub name: String"));
    assert!(result.contains("title: entidad.title.clone()"));
    assert!(result.contains("price: entidad.price,"));
    assert!(result.contains("pub struct PeticionCrearProducto"));
    assert!(result.contains("pub struct PeticionActualizarProducto"));
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
            name: "categoria_id".into(),
            field_type: "Uuid".into(),
            unique: false,
            ..Default::default()
        },
    ];

    let dto = generate_crud_dto("Producto", "producto", &fields).unwrap();
    assert!(dto.contains("pub categoria_id: Uuid"));
    assert!(dto.contains("categoria_id: entidad.categoria_id,"));
    let create_request = dto
        .split("pub struct PeticionCrearProducto")
        .nth(1)
        .expect("PeticionCrearProducto missing");
    assert!(
        create_request.contains("pub categoria_id: Uuid"),
        "PeticionCrearProducto must carry the Uuid field"
    );

    let routes = generate_crud_routes("Producto", "producto", &fields).unwrap();
    assert_eq!(
        routes.matches("categoria_id: cuerpo.categoria_id,").count(),
        2,
        "both the crear and the actualizar manejador must pass the Uuid field through"
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
    let result = generate_crud_routes("Producto", "producto", &fields).unwrap();

    assert!(result.contains("method = \"post\""));
    assert!(result.contains("method = \"get\""));
    assert!(result.contains("method = \"put\""));
    assert!(result.contains("method = \"delete\""));
    assert!(result.contains("pub struct ApiProducto"));
    assert!(result.contains("title: cuerpo.title.clone()"));
    assert!(result.contains("price: cuerpo.price,"));
    assert!(result.contains("id: IdProducto::from_uuid(id.0),"));
}

#[test]
fn write_repository_files_with_fields_generates_dynamic_entity_and_repo() {
    let dir = temp_dir("repo_files_dynamic");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

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
        "Producto",
        "producto",
        &dir,
        true,
        &fields,
        &[],
    )
    .unwrap();

    let entity =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/src/fila.rs")).unwrap();
    assert!(entity.contains("pub title: String"));
    assert!(entity.contains("pub price: i64"));
    assert!(!entity.contains("pub name: String"));

    let repo =
        fs::read_to_string(dir.join("contextos/producto/infraestructura/src/repositorio.rs"))
            .unwrap();
    assert!(repo.contains("title, price"));
    assert!(repo.contains("RepositorioProductoPostgres"));

    cleanup(&dir);
}

#[test]
fn write_presentation_files_with_fields_generates_dynamic_dto_and_routes() {
    let dir = temp_dir("pres_files_dynamic");
    cleanup(&dir);
    setup_puerto_stubs(&dir);

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
    crate::generators::presentation::write_presentation_files(
        "Producto", "producto", &dir, &fields,
    )
    .unwrap();

    let dto = fs::read_to_string(dir.join("contextos/producto/rest/src/dto.rs")).unwrap();
    assert!(dto.contains("pub title: String"));
    assert!(dto.contains("pub price: i64"));
    assert!(!dto.contains("pub name: String"));

    let routes = fs::read_to_string(dir.join("contextos/producto/rest/src/rutas.rs")).unwrap();
    assert!(routes.contains("title: cuerpo.title.clone()"));
    assert!(routes.contains("price: cuerpo.price,"));

    cleanup(&dir);
}

// ── T-13: review fixes — generated content ────────────────────────────────

#[test]
fn create_impl_logs_through_its_span_without_the_payload() {
    let fields = [scaffold_test_field("title", "String")];

    let content = crate::generators::application::generate_create_use_case_impl(
        "Producto",
        "producto",
        &fields,
        &[],
    )
    .unwrap();

    // `skip_all`: the span records no argument; `err` logs the error code once.
    assert!(
        content.contains(
            "#[tracing::instrument(name = \"crear_producto\", skip_all, err(level = \"warn\"))]"
        ),
        "{content}"
    );
    assert!(!content.contains("crear_producto: inicio"), "{content}");
    // The payload is not logged: a field value is user data (and often personal data).
    assert!(!content.contains("Creating producto"), "{content}");
}

#[test]
fn create_test_asserts_an_enum_vo_with_as_str() {
    let fields = [puerto_toml::parse_field_arg("state:State:enum:Open/Closed").unwrap()];

    let content = crate::generators::application::generate_create_use_case_impl(
        "Billete",
        "billete",
        &fields,
        &[],
    )
    .unwrap();

    assert!(!content.contains("state.valor()"), "{content}");
}

#[test]
fn unique_index_ignores_soft_deleted_rows() {
    // Soft-delete keeps the row: a plain unique index made a deleted SKU impossible to reuse.
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;

    let sql = crate::generators::infrastructure::create_table_sql("producto", &[sku]);

    assert!(sql.contains(
        "CREATE UNIQUE INDEX productos_sku_key ON productos (sku) WHERE borrado = false;"
    ));
}

#[test]
fn db_entity_maps_hashmap_columns_through_sqlx_json() {
    let fields = [scaffold_test_field("meta", "HashMap<String, String>")];

    let entity = crate::generators::infrastructure::generate_infra_entity(
        "Producto",
        "producto",
        &fields,
        &[],
    )
    .unwrap();
    let repo = crate::generators::infrastructure::generate_crud_infra_db_repository(
        "Producto",
        "producto",
        &fields,
        &[],
        false,
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

    let root = fs::read_to_string(output.join("Cargo.toml")).unwrap();
    assert!(root.contains("\"json\""), "{root}");
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

    let repo = crate::generators::infrastructure::generate_pg_seed(
        "Producto",
        "producto",
        &fields,
        &[],
        false,
    )
    .unwrap();

    // The timestamp is the seed's own whole second; the enum comes from the mother, unnamed here.
    assert!(
        repo.contains("published_at: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),"),
        "{repo}"
    );
    assert!(
        repo.contains("..MadreProducto::new().construir_props()"),
        "{repo}"
    );
    assert!(!repo.contains("valores::Status"), "{repo}");
    assert!(
        !repo.contains("Utc::now()"),
        "a ns-precision timestamp cannot round-trip Postgres"
    );
}

#[test]
fn pg_repository_seed_varies_unique_fields() {
    let mut sku = scaffold_test_field("sku", "String");
    sku.unique = true;

    let repo = crate::generators::infrastructure::generate_pg_seed(
        "Producto",
        "producto",
        &[sku],
        &[],
        false,
    )
    .unwrap();

    // From the mother (27.2), where a rule of the field can be met: a 36-char uuid broke any
    // length rule of its value object.
    assert!(repo.contains("sku: MadreProducto::sku_unico(),"), "{repo}");
    assert!(
        repo.contains("use dominio_producto::pruebas::madres::MadreProducto;"),
        "{repo}"
    );
    assert!(!repo.contains("use uuid::Uuid;"), "{repo}");
}

#[test]
fn pg_repository_seed_wraps_the_mothers_unique_value_in_its_value_object() {
    let sku = crate::puerto_toml::parse_field_arg("sku:Sku:String!").unwrap();

    let repo = crate::generators::infrastructure::generate_pg_seed(
        "Producto",
        "producto",
        &[sku],
        &[],
        false,
    )
    .unwrap();

    assert!(
        repo.contains("sku: Sku::new(MadreProducto::sku_unico())"),
        "{repo}"
    );
}

#[test]
fn mother_gives_a_fresh_value_per_call_for_each_unique_field() {
    let fields = [
        crate::puerto_toml::parse_field_arg("sku:Sku:String!").unwrap(),
        crate::puerto_toml::parse_field_arg("codigo:i64!").unwrap(),
        crate::puerto_toml::parse_field_arg("nombre:String").unwrap(),
    ];

    let mother =
        crate::generators::domain::generate_mother("Producto", "producto", &fields, &[]).unwrap();

    // The primitive the seed wraps: 12 hex characters by default, short enough for most rules.
    assert!(
        mother.contains(
            "pub fn sku_unico() -> String {\n        uuid::Uuid::new_v4().simple().to_string()[..12].to_string()\n    }"
        ),
        "{mother}"
    );
    assert!(
        mother.contains("pub fn codigo_unico() -> i64 {"),
        "{mother}"
    );
    assert!(!mother.contains("nombre_unico"), "{mother}");
}
