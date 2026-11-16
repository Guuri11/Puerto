// Phase 18 (ADR-0022): an entity refers to another by holding its typed id.
use super::*;

// ── 18.2: the manifest ──────────────────────────────────────────────────────

#[test]
fn should_parse_a_reference_as_a_uuid_naming_its_target() {
    let field = puerto_toml::parse_field_arg("cliente_id:ref:Cliente").unwrap();

    assert_eq!(field.name, "cliente_id");
    assert_eq!(field.field_type, "Uuid");
    assert_eq!(field.references.as_deref(), Some("Cliente"));
    assert!(field.value_object.is_none());
    assert!(!field.unique);
}

#[test]
fn should_parse_an_optional_reference_as_an_optional_uuid() {
    let field = puerto_toml::parse_field_arg("reviewer_id:ref:opt:Cliente").unwrap();

    assert_eq!(field.field_type, "Option<Uuid>");
    assert_eq!(field.references.as_deref(), Some("Cliente"));
}

#[test]
fn should_parse_a_unique_reference_as_one_to_one() {
    let field = puerto_toml::parse_field_arg("cliente_id:ref:Cliente!").unwrap();

    assert!(field.unique);
    assert_eq!(field.references.as_deref(), Some("Cliente"));
}

#[test]
fn should_reject_a_unique_optional_reference() {
    let err = puerto_toml::parse_field_arg("reviewer_id:ref:opt:Cliente!").unwrap_err();

    assert!(err.contains("unique"), "{err}");
}

#[test]
fn should_reject_a_reference_whose_target_is_not_pascal_case() {
    let err = puerto_toml::parse_field_arg("cliente_id:ref:cliente").unwrap_err();

    assert!(err.contains("cliente"), "{err}");
}

#[test]
fn should_keep_a_reference_through_puerto_toml() {
    // Arrange
    let dir = temp_dir("references_round_trip");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("puerto.toml"), "[proyecto]\nnombre = \"shop\"\n").unwrap();
    let field = puerto_toml::parse_field_arg("cliente_id:ref:Cliente").unwrap();

    // Act
    puerto_toml::add_entity(&dir, "Pedido", vec![], false, vec![field]).unwrap();

    // Assert
    let written = fs::read_to_string(dir.join("puerto.toml")).unwrap();
    assert!(written.contains("referencia = \"Cliente\""), "{written}");
    let config = puerto_toml::read(&dir).unwrap();
    assert_eq!(
        config.entity[0].fields[0].references.as_deref(),
        Some("Cliente")
    );

    cleanup(&dir);
}

/// A manifest with a CRUD `Cliente`, a simple `Saludo` and an `Pedido` holding `pedido_field`.
fn validate_pedido_with(name: &str, project_extra: &str, pedido_field: &str) -> Result<(), String> {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        format!(
            r#"[proyecto]
nombre = "shop"
{project_extra}
[[entidad]]
nombre = "Saludo"
casos_de_uso = ["obtener_saludo"]

[[entidad]]
nombre = "Cliente"
casos_de_uso = ["crear_cliente", "obtener_cliente", "listar_clientes", "actualizar_cliente", "borrar_cliente"]

[[entidad]]
nombre = "Pedido"
casos_de_uso = ["crear_pedido", "obtener_pedido", "listar_pedidos", "actualizar_pedido", "borrar_pedido"]

[[entidad.campos]]
{pedido_field}
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
    let result = validate_pedido_with(
        "references_validate_ok",
        "",
        "nombre = \"cliente_id\"\ntipo = \"Uuid\"\nreferencia = \"Cliente\"",
    );

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_refuse_a_reference_to_an_undeclared_entity() {
    let err = validate_pedido_with(
        "references_validate_undeclared",
        "",
        "nombre = \"proveedor_id\"\ntipo = \"Uuid\"\nreferencia = \"Proveedor\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'proveedor_id'") && err.contains("Proveedor"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_to_the_entity_itself() {
    let err = validate_pedido_with(
        "references_validate_self",
        "",
        "nombre = \"parent_id\"\ntipo = \"Option<Uuid>\"\nreferencia = \"Pedido\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'parent_id'") && err.contains("itself"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_to_an_entity_without_crud() {
    let err = validate_pedido_with(
        "references_validate_simple",
        "",
        "nombre = \"saludo_id\"\ntipo = \"Uuid\"\nreferencia = \"Saludo\"",
    )
    .unwrap_err();

    assert!(err.contains("'saludo_id'") && err.contains("CRUD"), "{err}");
}

#[test]
fn should_refuse_a_reference_to_the_user_in_an_auth_project() {
    let err = validate_pedido_with(
        "references_validate_user",
        "auth = \"oidc\"",
        "nombre = \"owner_id\"\ntipo = \"Uuid\"\nreferencia = \"User\"",
    )
    .unwrap_err();

    assert!(err.contains("'owner_id'") && err.contains("User"), "{err}");
}

#[test]
fn should_refuse_a_reference_that_is_not_a_uuid() {
    let err = validate_pedido_with(
        "references_validate_type",
        "",
        "nombre = \"cliente_id\"\ntipo = \"i64\"\nreferencia = \"Cliente\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'cliente_id'") && err.contains("Uuid"),
        "{err}"
    );
}

#[test]
fn should_refuse_a_reference_that_is_also_a_value_object() {
    let err = validate_pedido_with(
        "references_validate_vo",
        "",
        "nombre = \"cliente_id\"\ntipo = \"Uuid\"\nreferencia = \"Cliente\"\nobjeto_valor = \"ClienteRef\"",
    )
    .unwrap_err();

    assert!(
        err.contains("'cliente_id'") && err.contains("value_object"),
        "{err}"
    );
}

// ── 18.2: scaffold refuses before writing ───────────────────────────────────

fn pedido_fields() -> Vec<puerto_toml::Field> {
    vec![puerto_toml::parse_field_arg("cliente_id:ref:Cliente").unwrap()]
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_an_undeclared_entity() {
    // Arrange
    let dir = temp_dir("references_scaffold_undeclared");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), false, &dir).unwrap();

    // Act
    let err = scaffold::run_scaffold("Pedido", &output, &pedido_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("Cliente"), "{err}");
    assert!(!output.join("business/src/domain/pedido").exists());

    cleanup(&dir);
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_a_target_without_code() {
    // Arrange: Cliente is declared, but nothing was generated for it.
    let dir = temp_dir("references_scaffold_no_code");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), true, &dir).unwrap();
    puerto_toml::add_entity(
        &output,
        "Cliente",
        crate::generators::naming::crud_use_cases("cliente"),
        true,
        vec![],
    )
    .unwrap();

    // Act
    let err = scaffold::run_scaffold("Pedido", &output, &pedido_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("puerto generar scaffold Cliente"), "{err}");
    assert!(!output.join("business/src/domain/pedido").exists());

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
    let err = scaffold::run_generate_domain("Pedido", &output, &pedido_fields())
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("Cliente"), "{err}");
    assert!(!output.join("business/src/domain/pedido").exists());

    cleanup(&dir);
}

#[test]
fn should_validate_a_reference_to_an_entity_named_user_outside_an_auth_project() {
    // Arrange: without auth, `Usuario` is an ordinary entity.
    let dir = temp_dir("references_validate_plain_user");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("puerto.toml"),
        r#"[proyecto]
nombre = "shop"

[[entidad]]
nombre = "Usuario"
casos_de_uso = ["crear_usuario", "obtener_usuario", "listar_usuarios", "actualizar_usuario", "borrar_usuario"]

[[entidad]]
nombre = "Pedido"
casos_de_uso = ["crear_pedido"]

[[entidad.campos]]
nombre = "owner_id"
tipo = "Uuid"
referencia = "Usuario"
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
    scaffold::run_scaffold("Cliente", &output, &[], OverwritePolicy::Ask).unwrap();

    // Act
    let result = scaffold::run_scaffold("Pedido", &output, &pedido_fields(), OverwritePolicy::Ask);

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert!(
        output
            .join("contextos/pedido/dominio/src/modelo.rs")
            .exists()
    );
    cleanup(&dir);
}

#[test]
fn should_refuse_to_scaffold_a_reference_to_a_target_whose_table_is_not_created() {
    // Arrange: the model is there, the migration that creates its table is not.
    let dir = temp_dir("references_scaffold_no_table");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), true, &dir).unwrap();
    scaffold::run_scaffold("Cliente", &output, &[], OverwritePolicy::Ask).unwrap();
    for entry in fs::read_dir(output.join("migraciones")).unwrap() {
        let path = entry.unwrap().path();
        if path.to_string_lossy().ends_with("_crear_tabla_cliente.sql") {
            fs::remove_file(path).unwrap();
        }
    }

    // Act
    let err = scaffold::run_scaffold("Pedido", &output, &pedido_fields(), OverwritePolicy::Ask)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("puerto generar scaffold Cliente"), "{err}");
    cleanup(&dir);
}

// ── 18.3: generated backend ─────────────────────────────────────────────────

/// A project with a CRUD `Cliente` and an `Pedido` holding `cliente_id:ref:Cliente` and
/// `reviewer_id:ref:opt:Cliente`. `db`: `--db`; `events`: also `[proyecto] eventos = "db"`.
fn pedido_project(name: &str, db: bool, events: bool) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), db, &dir).unwrap();
    if events {
        let manifest = output.join("puerto.toml");
        let toml = fs::read_to_string(&manifest).unwrap();
        fs::write(
            &manifest,
            toml.replace("db = true", "db = true\neventos = \"db\""),
        )
        .unwrap();
    }
    scaffold::run_scaffold("Cliente", &output, &[], OverwritePolicy::Ask).unwrap();
    let fields = vec![
        puerto_toml::parse_field_arg("cliente_id:ref:Cliente").unwrap(),
        puerto_toml::parse_field_arg("reviewer_id:ref:opt:Cliente").unwrap(),
    ];
    scaffold::run_scaffold("Pedido", &output, &fields, OverwritePolicy::Ask).unwrap();
    (dir, output)
}

fn read(output: &Path, path: &str) -> String {
    fs::read_to_string(output.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn should_hold_the_target_id_type_in_the_domain() {
    let (dir, output) = pedido_project("references_domain", false, false);

    let model = read(&output, "contextos/pedido/dominio/src/modelo.rs");
    assert!(
        model.contains("use dominio_cliente::modelo::IdCliente;"),
        "{model}"
    );
    assert!(model.contains("pub cliente_id: IdCliente,"), "{model}");
    assert!(
        model.contains("pub reviewer_id: Option<IdCliente>,"),
        "{model}"
    );

    for uc in ["crear_pedido", "actualizar_pedido"] {
        let params = read(&output, &format!("contextos/pedido/aplicacion/src/{uc}.rs"));
        assert!(
            params.contains("use dominio_cliente::modelo::IdCliente;"),
            "{uc}"
        );
        assert!(
            params.contains("pub cliente_id: IdCliente,"),
            "{uc}: {params}"
        );
        assert!(
            params.contains("pub reviewer_id: Option<IdCliente>,"),
            "{uc}"
        );
        assert!(!params.contains("uuid::Uuid"), "{uc} imports Uuid");
    }

    let mother = read(&output, "contextos/pedido/dominio/src/pruebas/madres.rs");
    assert!(
        mother.contains("pub fn con_cliente_id(mut self, cliente_id: IdCliente)"),
        "{mother}"
    );
    assert!(
        mother.contains("unwrap_or_else(IdCliente::generar)"),
        "{mother}"
    );
    assert!(!mother.contains("uuid::Uuid"), "{mother}");

    let create = read(&output, "contextos/pedido/aplicacion/src/crear_pedido.rs");
    assert!(
        create.contains("cliente_id: IdCliente::generar(),"),
        "{create}"
    );

    cleanup(&dir);
}

#[test]
fn should_turn_a_missing_reference_into_a_validation_error_on_the_field() {
    let (dir, output) = pedido_project("references_errors", false, false);

    let errors = read(&output, "contextos/pedido/dominio/src/errores.rs");
    assert!(
        errors.contains(
            "ErrorRepositorio::ReferenciaInexistente(field) => {\n                Self::Validacion(format!(\"{field}_no_encontrado\"))"
        ),
        "{errors}"
    );
    assert!(
        errors.contains("\"pedido.validacion.cliente_id_no_encontrado\""),
        "{errors}"
    );
    let cliente_errors = read(&output, "contextos/cliente/dominio/src/errores.rs");
    assert!(
        !cliente_errors.contains("MissingReference"),
        "{cliente_errors}"
    );

    cleanup(&dir);
}

#[test]
fn should_convert_a_reference_at_the_api_edge() {
    let (dir, output) = pedido_project("references_api", false, false);

    let dto = read(&output, "contextos/pedido/rest/src/dto.rs");
    assert!(dto.contains("pub cliente_id: Uuid,"), "{dto}");
    assert!(dto.contains("pub reviewer_id: Option<Uuid>,"), "{dto}");
    assert!(
        dto.contains("cliente_id: entidad.cliente_id.valor(),"),
        "{dto}"
    );
    assert!(
        dto.contains("reviewer_id: entidad.reviewer_id.map(|v| v.valor()),"),
        "{dto}"
    );

    let routes = read(&output, "contextos/pedido/rest/src/rutas.rs");
    assert!(
        routes.contains("use dominio_cliente::modelo::IdCliente;"),
        "{routes}"
    );
    assert!(
        routes.contains("cliente_id: IdCliente::from_uuid(cuerpo.cliente_id),"),
        "{routes}"
    );
    assert!(
        routes.contains("reviewer_id: cuerpo.reviewer_id.map(IdCliente::from_uuid),"),
        "{routes}"
    );
    assert!(
        routes.contains(
            "async fn deberia_responder_422_cuando_no_existe_la_referencia_de_cliente_id()"
        ),
        "{routes}"
    );

    cleanup(&dir);
}

#[test]
fn should_hold_a_uuid_in_the_row_and_the_target_id_in_the_entity() {
    let (dir, output) = pedido_project("references_row", true, false);

    let entity = read(&output, "contextos/pedido/infraestructura/src/fila.rs");
    assert!(
        entity.contains("use dominio_cliente::modelo::IdCliente;"),
        "{entity}"
    );
    assert!(entity.contains("pub cliente_id: Uuid,"), "{entity}");
    assert!(
        entity.contains("pub reviewer_id: Option<Uuid>,"),
        "{entity}"
    );
    assert!(
        entity.contains("cliente_id: IdCliente::from_uuid(fila.cliente_id),"),
        "{entity}"
    );
    assert!(
        entity.contains("reviewer_id: fila.reviewer_id.map(IdCliente::from_uuid),"),
        "{entity}"
    );
    assert!(
        entity.contains("cliente_id: entidad.cliente_id.valor(),"),
        "{entity}"
    );

    cleanup(&dir);
}

#[test]
fn should_create_a_foreign_key_and_an_index_per_reference() {
    let (dir, output) = pedido_project("references_migration", true, false);

    let migration = fs::read_dir(output.join("migraciones"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("_crear_tabla_pedido.sql"))
        .map(|p| fs::read_to_string(p).unwrap())
        .unwrap();
    assert!(
        migration.contains(
            "cliente_id UUID NOT NULL CONSTRAINT pedidos_cliente_id_fkey REFERENCES clientes (id),"
        ),
        "{migration}"
    );
    assert!(
        migration.contains(
            "reviewer_id UUID CONSTRAINT pedidos_reviewer_id_fkey REFERENCES clientes (id),"
        ),
        "{migration}"
    );
    assert!(
        migration.contains("CREATE INDEX pedidos_cliente_id_idx ON pedidos (cliente_id);"),
        "{migration}"
    );
    assert!(
        migration.contains("CREATE INDEX pedidos_reviewer_id_idx ON pedidos (reviewer_id);"),
        "{migration}"
    );

    cleanup(&dir);
}

#[test]
fn should_index_a_unique_reference_only_once() {
    let sql = crate::generators::infrastructure::create_table_sql(
        "pedido",
        &[puerto_toml::parse_field_arg("cliente_id:ref:Cliente!").unwrap()],
    );

    assert!(
        sql.contains("CREATE UNIQUE INDEX pedidos_cliente_id_key"),
        "{sql}"
    );
    assert!(!sql.contains("pedidos_cliente_id_idx"), "{sql}");
}

#[test]
fn should_map_the_foreign_keys_where_the_rows_are_written() {
    let (dir, output) = pedido_project("references_pg_repo", true, false);

    let repo = read(
        &output,
        "contextos/pedido/infraestructura/src/repositorio.rs",
    );
    assert!(
        repo.contains("const REFERENCES: &[ClaveForanea] = &["),
        "{repo}"
    );
    assert!(
        repo.contains("restriccion: \"pedidos_cliente_id_fkey\","),
        "{repo}"
    );
    assert!(repo.contains("campo: \"reviewer_id\","), "{repo}");
    assert!(
        repo.contains("mapear_error_sqlx_con(&e, REFERENCES)"),
        "{repo}"
    );
    // The rows it seeds first get their cliente through Cliente's own seed (ADR-0023 § 5).
    let seed = read(
        &output,
        "contextos/pedido/infraestructura/src/pruebas/siembra.rs",
    );
    assert!(
        seed.contains("cliente_id: infra_cliente::pruebas::siembra::sembrar(pool).await.id,"),
        "{seed}"
    );
    let manifest = read(&output, "contextos/pedido/infraestructura/Cargo.toml");
    assert!(
        manifest.contains("pruebas = [\"dominio_pedido/pruebas\", \"dep:infra_cliente\", \"infra_cliente/pruebas\"]"),
        "{manifest}"
    );
    assert!(
        repo.contains("async fn deberia_rechazar_cliente_id_inexistente(pool: PgPool)"),
        "{repo}"
    );
    assert!(
        repo.contains("async fn deberia_rechazar_reviewer_id_inexistente(pool: PgPool)"),
        "{repo}"
    );
    assert!(
        repo.contains("ErrorRepositorio::ReferenciaInexistente(\"cliente_id\")"),
        "{repo}"
    );
    let cliente = read(
        &output,
        "contextos/cliente/infraestructura/src/pruebas/siembra.rs",
    );
    assert!(
        cliente.contains("pub async fn sembrar(pool: &PgPool) -> Cliente {"),
        "{cliente}"
    );
    let cliente_repo = read(
        &output,
        "contextos/cliente/infraestructura/src/repositorio.rs",
    );
    assert!(!cliente_repo.contains("REFERENCES"), "{cliente_repo}");

    cleanup(&dir);
}

#[test]
fn should_map_the_foreign_keys_in_the_unit_of_work_of_an_events_project() {
    let (dir, output) = pedido_project("references_pg_uow", true, true);

    let uow = read(
        &output,
        "contextos/pedido/infraestructura/src/unit_of_work.rs",
    );
    assert!(
        uow.contains("const REFERENCES: &[ClaveForanea] = &["),
        "{uow}"
    );
    assert!(
        uow.contains("mapear_error_sqlx_con(&e, REFERENCES)"),
        "{uow}"
    );
    assert!(
        uow.contains("async fn deberia_rechazar_cliente_id_inexistente(pool: PgPool)"),
        "{uow}"
    );
    let repo = read(
        &output,
        "contextos/pedido/infraestructura/src/repositorio.rs",
    );
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
    let pedido = |cliente: &str, reviewer: &str| {
        format!(r#"{{"clienteId":"{cliente}","reviewerId":{reviewer},"total":42}}"#)
    };
    let missing = "00000000-0000-4000-8000-000000000000";

    // A cliente nobody created: the foreign key refuses it, on its field.
    let (status, body) = http(port, "POST", "/api/pedidos", &pedido(missing, "null")).unwrap();
    assert_eq!(status, 422, "{body}");
    assert!(
        body.contains("\"codigo\":\"pedido.validacion.cliente_id_no_encontrado\""),
        "{body}"
    );

    // An existing one: created; an unknown reviewer is refused on its own field.
    let (status, body) = http(
        port,
        "POST",
        "/api/clientes",
        r#"{"name":"Ada","email":"ada@example.com"}"#,
    )
    .unwrap();
    assert_eq!(status, 201, "{body}");
    let cliente: serde_json::Value = serde_json::from_str(&body).unwrap();
    let cliente = cliente["id"].as_str().unwrap().to_string();
    let (status, body) = http(port, "POST", "/api/pedidos", &pedido(&cliente, "null")).unwrap();
    assert_eq!(status, 201, "{body}");
    let reviewer = format!("\"{missing}\"");
    let (status, body) = http(port, "POST", "/api/pedidos", &pedido(&cliente, &reviewer)).unwrap();
    assert_eq!(status, 422, "{body}");
    assert!(
        body.contains("\"codigo\":\"pedido.validacion.reviewer_id_no_encontrado\""),
        "{body}"
    );

    // The list filters by cliente (18.4): another cliente's pedido is left out, page and total.
    let (status, body) = http(
        port,
        "POST",
        "/api/clientes",
        r#"{"name":"Grace","email":"grace@example.com"}"#,
    )
    .unwrap();
    assert_eq!(status, 201, "{body}");
    let other: serde_json::Value = serde_json::from_str(&body).unwrap();
    let other = other["id"].as_str().unwrap().to_string();
    let (status, body) = http(port, "POST", "/api/pedidos", &pedido(&other, "null")).unwrap();
    assert_eq!(status, 201, "{body}");
    let (status, body) = http(
        port,
        "GET",
        &format!("/api/pedidos?clienteId={cliente}"),
        "",
    )
    .unwrap();
    assert_eq!(status, 200, "{body}");
    let page: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(page["total"], 1, "{body}");
    assert_eq!(
        page["elementos"][0]["clienteId"],
        cliente.as_str(),
        "{body}"
    );

    drop(api);
    cleanup(&project.dir);
}

#[test]
fn should_test_that_create_reports_a_missing_reference_on_its_field() {
    for (name, events) in [
        ("references_create_test", false),
        ("references_create_test_events", true),
    ] {
        let (dir, output) = pedido_project(name, events, events);

        let create = read(&output, "contextos/pedido/aplicacion/src/crear_pedido.rs");
        for field in ["cliente_id", "reviewer_id"] {
            assert!(
                create.contains(&format!(
                    "async fn deberia_devolver_un_error_de_validacion_cuando_{field}_no_existe()"
                )),
                "eventos = {events}: {create}"
            );
            assert!(create.contains(&format!(
                "ErrorRepositorio::ReferenciaInexistente(\"{field}\")"
            )));
            assert!(create.contains(&format!("\"pedido.validacion.{field}_no_encontrado\"")));
        }

        cleanup(&dir);
    }
}

// ── 18.4: list by reference ─────────────────────────────────────────────────

#[test]
fn should_filter_the_list_by_each_reference_in_the_domain() {
    let (dir, output) = pedido_project("references_filter_domain", false, false);

    let repository = read(&output, "contextos/pedido/dominio/src/repositorio.rs");
    assert!(
        repository.contains("use dominio_cliente::modelo::IdCliente;"),
        "{repository}"
    );
    assert!(
        repository.contains(
            "pub struct FiltroPedido {\n    pub cliente_id: Option<IdCliente>,\n    pub reviewer_id: Option<IdCliente>,\n}"
        ),
        "{repository}"
    );
    assert_eq!(
        repository
            .matches("async fn buscar_todos(&self, pagina: PeticionPagina, filtro: FiltroPedido)")
            .count(),
        2,
        "the port and its mock: {repository}"
    );
    let params = read(&output, "contextos/pedido/aplicacion/src/listar_pedidos.rs");
    assert!(
        params.contains("pub cliente_id: Option<IdCliente>,"),
        "{params}"
    );
    assert!(
        params.contains("pub reviewer_id: Option<IdCliente>,"),
        "{params}"
    );

    // An entity without references keeps today's port.
    let cliente = read(&output, "contextos/cliente/dominio/src/repositorio.rs");
    assert!(!cliente.contains("Filter"), "{cliente}");
    assert!(
        cliente.contains("async fn buscar_todos(&self, pagina: PeticionPagina)"),
        "{cliente}"
    );

    cleanup(&dir);
}

#[test]
fn should_pass_the_filter_from_the_use_case_to_the_repository() {
    let (dir, output) = pedido_project("references_filter_application", false, false);

    let list = read(&output, "contextos/pedido/aplicacion/src/listar_pedidos.rs");
    assert!(
        list.contains(
            "FiltroPedido {\n                    cliente_id: params.cliente_id,\n                    reviewer_id: params.reviewer_id,\n                }"
        ),
        "{list}"
    );
    assert!(
        list.contains("async fn deberia_pasar_el_filtro_al_repositorio()"),
        "{list}"
    );

    let repo = read(
        &output,
        "contextos/pedido/infraestructura/src/repositorio.rs",
    );
    assert!(
        repo.contains(".is_none_or(|id| fila.cliente_id == id)"),
        "{repo}"
    );
    assert!(
        repo.contains(".is_none_or(|id| fila.reviewer_id == Some(id))"),
        "{repo}"
    );
    assert!(
        repo.contains("async fn deberia_listar_solo_pedidos_con_ese_valor_de_cliente_id()"),
        "{repo}"
    );

    cleanup(&dir);
}

#[test]
fn should_take_each_reference_filter_as_a_camel_case_query_parameter() {
    let (dir, output) = pedido_project("references_filter_api", false, false);

    let routes = read(&output, "contextos/pedido/rest/src/rutas.rs");
    assert!(
        routes.contains("#[oai(name = \"clienteId\")] cliente_id: Query<Option<Uuid>>"),
        "{routes}"
    );
    assert!(
        routes.contains("cliente_id: cliente_id.0.map(IdCliente::from_uuid),"),
        "{routes}"
    );
    assert!(
        routes.contains("async fn deberia_pasar_el_filtro_por_reviewer_id_al_caso_de_uso()"),
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
        let (dir, output) = pedido_project(name, true, events);

        let repo = read(
            &output,
            "contextos/pedido/infraestructura/src/repositorio.rs",
        );
        assert!(
            repo.contains(
                "FROM pedidos WHERE borrado = false AND ($3::uuid IS NULL OR cliente_id = $3) AND ($4::uuid IS NULL OR reviewer_id = $4)"
            ),
            "eventos = {events}: {repo}"
        );
        assert!(
            repo.contains(
                "FROM pedidos WHERE borrado = false AND ($1::uuid IS NULL OR cliente_id = $1) AND ($2::uuid IS NULL OR reviewer_id = $2)"
            ),
            "the COUNT, events = {events}: {repo}"
        );
        assert!(
            repo.contains("filtro.cliente_id.map(|id| id.valor())"),
            "{repo}"
        );
        for field in ["cliente_id", "reviewer_id"] {
            assert!(
                repo.contains(&format!(
                    "async fn deberia_listar_solo_pedidos_con_ese_valor_de_{field}(pool: PgPool)"
                )),
                "eventos = {events}: {repo}"
            );
        }

        cleanup(&dir);
    }
}

// ── 18.5: the frontend ──────────────────────────────────────────────────────

const PEDIDO_WEB: &str = "web/src/presentation/features/pedido";

#[test]
fn should_choose_a_reference_from_a_select_of_its_targets() {
    let (dir, output) = pedido_project("references_web_form", false, false);

    let view = read(&output, &format!("{PEDIDO_WEB}/views/pedido-form-view.tsx"));
    assert!(
        view.contains("export interface PedidoFormOptions {\n  clienteId: ReferenceOption[];\n  reviewerId: ReferenceOption[];\n}"),
        "{view}"
    );
    assert!(
        view.contains("<span className=\"fieldset-legend\">Cliente</span>"),
        "{view}"
    );
    assert!(
        view.contains("<option value=\"\">Choose a cliente</option>"),
        "{view}"
    );
    assert!(view.contains("<option value=\"\">None</option>"), "{view}");
    assert!(
        view.contains("{options.clienteId.map((option) => ("),
        "{view}"
    );

    let container = read(
        &output,
        &format!("{PEDIDO_WEB}/containers/pedido-form-container.tsx"),
    );
    assert!(
        container.contains("clienteRepository.list({ limit: MAX_PAGE_LIMIT, offset: 0 })"),
        "{container}"
    );
    // One load per target entity, whatever the number of references to it.
    assert_eq!(
        container.matches("clienteRepository.list(").count(),
        1,
        "{container}"
    );
    assert!(
        container.contains("clienteId: referenceOptions(clientes, (cliente) => cliente.nombre),"),
        "{container}"
    );

    let test = read(
        &output,
        &format!("{PEDIDO_WEB}/containers/pedido-form-container.test.tsx"),
    );
    assert!(test.contains("...referencedBy(draft),"), "{test}");
    assert!(
        test.contains("ClienteMother.random({ id: draft.clienteId }),"),
        "{test}"
    );
    assert!(
        test.contains("test(\"should offer every cliente in the selects that reference one\""),
        "{test}"
    );

    let page = read(&output, "web/src/domain/shared/page.ts");
    assert!(
        page.contains("export const MAX_PAGE_LIMIT = 100;"),
        "{page}"
    );
    let forms = read(&output, "web/src/presentation/shared/forms.ts");
    assert!(
        forms.contains("export function referenceOptions<"),
        "{forms}"
    );

    cleanup(&dir);
}

#[test]
fn should_label_a_target_without_a_string_field_by_its_id() {
    let dir = temp_dir("references_web_label_id");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = new_project_non_interactive(Some("shop".into()), false, &dir).unwrap();
    let weight = vec![puerto_toml::parse_field_arg("weight:i64").unwrap()];
    scaffold::run_scaffold("Pallet", &output, &weight, OverwritePolicy::Ask).unwrap();
    let parcel = vec![puerto_toml::parse_field_arg("pallet_id:ref:Pallet").unwrap()];
    scaffold::run_scaffold("Parcel", &output, &parcel, OverwritePolicy::Ask).unwrap();

    let container = read(
        &output,
        "web/src/presentation/features/parcel/containers/parcel-form-container.tsx",
    );
    assert!(container.contains("(pallet) => pallet.id)"), "{container}");

    cleanup(&dir);
}

#[test]
fn should_link_a_reference_to_its_target_and_say_when_it_is_missing() {
    let (dir, output) = pedido_project("references_web_detail", false, false);

    let detail = read(
        &output,
        &format!("{PEDIDO_WEB}/views/pedido-detail-view.tsx"),
    );
    assert!(
        detail.contains("<ReferenceLink path=\"/clientes\" id={pedido.clienteId} />"),
        "{detail}"
    );
    assert!(
        output
            .join("web/src/presentation/shared/ui/reference-link.tsx")
            .exists()
    );
    let detail_test = read(
        &output,
        &format!("{PEDIDO_WEB}/containers/pedido-detail-container.test.tsx"),
    );
    assert!(
        detail_test.contains("test(\"should link the reviewer to its page\""),
        "{detail_test}"
    );

    let messages = read(&output, "web/src/domain/pedido/messages.ts");
    assert!(
        messages.contains(
            "\"pedido.validacion.cliente_id_no_encontrado\": \"Cliente does not exist\","
        ),
        "{messages}"
    );
    assert!(
        messages.contains("\"pedido.validacion.reviewer_id_no_encontrado\": \"reviewerId\","),
        "{messages}"
    );

    cleanup(&dir);
}

// ── 19.6 (ADR-0023 § 3): each crate names only what its layer may depend on ──────────

/// The project crates the `[dependencies]` of `manifest` name (dev-dependencies left out: the
/// test helpers of another crate are fine there).
fn project_dependencies(manifest: &str) -> Vec<String> {
    let deps = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .split("\n[")
        .next()
        .unwrap_or("");
    deps.lines()
        .filter_map(|l| l.split_once(" = ").map(|(name, _)| name.trim().to_string()))
        .filter(|name| {
            [
                "dominio_",
                "aplicacion_",
                "infra_",
                "rest_",
                "nucleo",
                "cableado",
            ]
            .iter()
            .any(|p| name.starts_with(p))
        })
        .collect()
}

#[test]
fn should_let_each_crate_depend_only_on_what_its_layer_may_use() {
    let (dir, output) = pedido_project("references_dependency_rule", true, true);

    let deps = |c: &str, layer: &str| {
        project_dependencies(&read(&output, &format!("contextos/{c}/{layer}/Cargo.toml")))
    };
    assert_eq!(deps("pedido", "dominio"), ["dominio_cliente"]);
    assert_eq!(
        deps("pedido", "aplicacion"),
        ["dominio_pedido", "dominio_cliente"]
    );
    // `infra_cliente` is optional, for the seeds of the Postgres tests (`pruebas`).
    assert_eq!(
        deps("pedido", "infraestructura"),
        ["dominio_pedido", "dominio_cliente", "infra_cliente"]
    );
    assert!(
        read(&output, "contextos/pedido/infraestructura/Cargo.toml")
            .contains("infra_cliente = { workspace = true, optional = true }")
    );
    assert_eq!(
        deps("pedido", "rest"),
        ["aplicacion_pedido", "dominio_pedido", "dominio_cliente"]
    );
    assert!(deps("cliente", "dominio").is_empty());
    // Only the composition root knows the adapters; it knows no HTTP.
    let cableado = read(&output, "presentacion/cableado/Cargo.toml");
    assert!(
        cableado.contains("infra_pedido = { workspace = true }"),
        "{cableado}"
    );
    assert!(
        !cableado.contains("rest_") && !cableado.contains("poem"),
        "{cableado}"
    );

    cleanup(&dir);
}

#[test]
fn should_refuse_a_reference_that_closes_a_cycle_naming_it() {
    // Arrange: Pedido → Cliente already.
    let (dir, output) = pedido_project("references_cycle", false, false);
    let field = puerto_toml::parse_field_arg("last_pedido_id:ref:opt:Pedido").unwrap();

    // Act
    let err = scaffold::run_scaffold("Cliente", &output, &[field], OverwritePolicy::Force)
        .unwrap_err()
        .to_string();

    // Assert
    assert!(err.contains("Cliente → Pedido → Cliente"), "{err}");
    assert!(
        !read(&output, "contextos/cliente/dominio/src/modelo.rs").contains("last_pedido_id"),
        "nothing is rewritten"
    );
    cleanup(&dir);
}
