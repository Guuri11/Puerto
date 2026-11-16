use super::*;
use crate::generators::naming::{capitalize, snake_to_camel, snake_to_kebab, snake_to_words};
use crate::generators::web::{generate_web_dependencies, generate_web_routes, web_entity_files};

// ── Phase 9.2: the CRUD slice of an entity in `web/` ──────────────────────

/// A fresh `puerto nuevo` project (web by default) in its own temp dir.
fn web_project(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    (dir, output)
}

fn fields(args: &[&str]) -> Vec<puerto_toml::Field> {
    args.iter()
        .map(|a| puerto_toml::parse_field_arg(a).unwrap())
        .collect()
}

fn read(base: &Path, path: &str) -> String {
    fs::read_to_string(base.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn should_name_frontend_identifiers_after_the_snake_case_name() {
    assert_eq!(snake_to_kebab("linea_pedido"), "linea-pedido");
    assert_eq!(snake_to_camel("linea_pedido"), "lineaPedido");
    assert_eq!(snake_to_camel("in_stock"), "inStock");
    assert_eq!(snake_to_camel("producto"), "producto");
    assert_eq!(snake_to_words("lineas_pedido"), "lineas pedido");
    assert_eq!(capitalize("lineas pedido"), "Lineas pedido");
    assert_eq!(capitalize(""), "");
}

#[test]
fn template_generated_web_files_are_exactly_what_the_generator_writes_for_no_entity() {
    // `puerto nuevo` copies `core/generated/*` verbatim, like `bootstrap.rs`; the first scaffold
    // rewrites them. A template copy that drifts from the generator ships a different shell.
    let (dir, output) = web_project("web_template_generated");
    let config = puerto_toml::read(&output).unwrap();
    let generated = "web/src/presentation/core/generated";

    assert_eq!(
        read(&output, &format!("{generated}/dependencies.ts")),
        generate_web_dependencies(&config).unwrap(),
        "template/{generated}/dependencies.ts is stale"
    );
    assert_eq!(
        read(&output, &format!("{generated}/routes.tsx")),
        generate_web_routes(&config).unwrap(),
        "template/{generated}/routes.tsx is stale"
    );
    cleanup(&dir);
}

#[test]
fn scaffold_in_a_web_project_writes_the_whole_slice_and_no_frontend_use_case() {
    let (dir, output) = web_project("web_scaffold_slice");

    scaffold::run_scaffold(
        "LineaPedido",
        &output,
        &fields(&["title:String", "quantity:i64"]),
        OverwritePolicy::Ask,
    )
    .unwrap();

    for file in web_entity_files("linea_pedido") {
        assert!(output.join(&file).exists(), "{} missing", file.display());
    }
    assert!(
        output
            .join("web/src/infrastructure/http/client.ts")
            .exists()
    );
    assert!(!output.join("web/src/application/linea-pedido").exists());
    assert!(!output.join("web/src/application/linea_pedido").exists());
    cleanup(&dir);
}

#[test]
fn scaffold_names_files_in_kebab_case_routes_after_the_plural_and_variables_in_camel_case() {
    let (dir, output) = web_project("web_scaffold_names");

    scaffold::run_scaffold("LineaPedido", &output, &[], OverwritePolicy::Ask).unwrap();

    let routes = read(&output, "web/src/presentation/core/generated/routes.tsx");
    assert!(
        routes.contains(r#"<Route path="/lineas_pedido/:id/edit">"#),
        "{routes}"
    );
    assert!(routes.contains(r#"<Route path="/lineas_pedido/new">"#));
    assert!(routes.contains(r#"{ label: "Lineas pedido", href: "/lineas_pedido" }"#));
    assert!(routes.contains("../../features/linea-pedido/containers/linea-pedido-list-container"));

    let dependencies = read(
        &output,
        "web/src/presentation/core/generated/dependencies.ts",
    );
    assert!(dependencies.contains("lineaPedidoRepository: lineaPedidoRepositoryHttp(api),"));

    let adapter = read(
        &output,
        "web/src/infrastructure/linea-pedido/linea-pedido-repository-http.ts",
    );
    assert!(
        adapter.contains(r#"api.GET("/lineas_pedido/{id}""#),
        "{adapter}"
    );
    assert!(adapter.contains(r#"components["schemas"]["DtoLineaPedido"]"#));
    cleanup(&dir);
}

#[test]
fn scaffold_maps_every_field_to_its_type_input_and_wire_name() {
    let (dir, output) = web_project("web_scaffold_fields");

    scaffold::run_scaffold(
        "Producto",
        &output,
        &fields(&[
            "title:String",
            "price:i64",
            "rate:f64",
            "in_stock:bool",
            "ref_id:Uuid",
            "sku:Sku:String",
        ]),
        OverwritePolicy::Ask,
    )
    .unwrap();

    let model = read(&output, "web/src/domain/producto/model.ts");
    for property in [
        "title: string;",
        "price: number;",
        "rate: number;",
        "inStock: boolean;",
        "refId: string;",
        "sku: string;",
    ] {
        assert!(model.contains(property), "{property} in:\n{model}");
    }

    let form = read(
        &output,
        "web/src/presentation/features/producto/views/producto-form-view.tsx",
    );
    assert!(form.contains("price: Number(values.price),"));
    assert!(form.contains("inStock: producto.inStock,"));
    assert!(form.contains(r#"step="any""#), "an f64 takes decimals");
    assert_eq!(
        form.matches(r#"step="any""#).count(),
        1,
        "only the f64 does"
    );
    assert!(form.contains(r#"type="checkbox""#));
    assert!(form.contains("<span className=\"fieldset-legend\">Ref id</span>"));
    cleanup(&dir);
}

#[test]
fn scaffold_in_a_project_without_web_writes_nothing_under_web() {
    let dir = temp_dir("web_scaffold_non_web");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    setup_puerto_stubs(&dir);

    scaffold::run_scaffold("Producto", &dir, &[], OverwritePolicy::Ask).unwrap();

    assert!(!dir.join("web").exists());
    cleanup(&dir);
}

#[test]
fn generated_files_list_every_crud_entity_and_skip_saludo() {
    let (dir, output) = web_project("web_generated_lists");

    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    scaffold::run_scaffold("Categoria", &output, &[], OverwritePolicy::Ask).unwrap();

    let routes = read(&output, "web/src/presentation/core/generated/routes.tsx");
    let dependencies = read(
        &output,
        "web/src/presentation/core/generated/dependencies.ts",
    );
    for text in [&routes, &dependencies] {
        assert!(text.contains("Producto") && text.contains("Categoria"));
        assert!(
            !text.contains("Saludo"),
            "Saludo has no CRUD endpoints:\n{text}"
        );
    }
    cleanup(&dir);
}

#[test]
fn every_field_type_reaches_the_frontend_with_its_own_input() {
    let (dir, output) = web_project("web_every_type");

    scaffold::run_scaffold(
        "Event",
        &output,
        &fields(&[
            "summary:opt:String",
            "flag:opt:bool",
            "starts_at:DateTime",
            "tags:vec:String",
            "meta:map",
            "status:Status:enum:Draft/Published",
        ]),
        OverwritePolicy::Ask,
    )
    .unwrap();

    let model = read(&output, "web/src/domain/event/model.ts");
    for line in [
        "summary: string | null;",
        "flag: boolean | null;",
        "startsAt: string;",
        "tags: string[];",
        "meta: Record<string, string>;",
        "status: Status;",
        r#"export const statusValues = ["Draft", "Published"] as const;"#,
        r#"throw new EventError("event.status_invalido");"#,
    ] {
        assert!(model.contains(line), "{line} in:\n{model}");
    }
    let adapter = read(
        &output,
        "web/src/infrastructure/event/event-repository-http.ts",
    );
    assert!(
        adapter.contains("summary: dto.summary ?? null,"),
        "{adapter}"
    );
    assert!(adapter.contains("summary: draft.summary ?? undefined,"));
    assert!(adapter.contains("status: parseStatus(dto.status),"));

    let form = read(
        &output,
        "web/src/presentation/features/event/views/event-form-view.tsx",
    );
    assert!(form.contains(r#"type="datetime-local""#));
    assert!(form.contains("<textarea"));
    assert!(form.contains(r#"<option value="Published">Published</option>"#));
    assert!(
        form.contains(r#"<option value="true">Yes</option>"#),
        "optional bool: a select"
    );
    assert!(form.contains(r#"from "../../../shared/forms";"#));
    assert!(validate::run_validate(&output).is_ok());
    cleanup(&dir);
}

#[test]
fn every_enum_of_an_entity_gets_its_values_once() {
    let (dir, output) = web_project("web_two_enums");

    scaffold::run_scaffold(
        "Task",
        &output,
        &fields(&[
            "status:Status:enum:Open/Closed",
            "priority:Priority:enum:Low/High",
        ]),
        OverwritePolicy::Ask,
    )
    .unwrap();

    let model = read(&output, "web/src/domain/task/model.ts");
    for line in [
        r#"export const statusValues = ["Open", "Closed"] as const;"#,
        r#"export const priorityValues = ["Low", "High"] as const;"#,
    ] {
        assert_eq!(model.matches(line).count(), 1, "{line} in:\n{model}");
    }
    cleanup(&dir);
}

#[test]
fn generate_use_case_leaves_web_untouched() {
    let (dir, output) = web_project("web_use_case_untouched");
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    let before = generated_output::project_tree(&output.join("web"));

    scaffold::run_use_case("Producto", "archive_producto", &output).unwrap();

    assert_eq!(generated_output::project_tree(&output.join("web")), before);
    cleanup(&dir);
}

#[test]
fn rescaffold_without_force_refuses_when_only_web_files_exist() {
    let (dir, output) = web_project("web_rescaffold_guard");
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    // Only the frontend slice is left: the guard must still see it.
    for file in crate::generators::conflict::entity_files("producto", false, false) {
        let _ = fs::remove_file(output.join(file));
    }
    let form = output.join("web/src/presentation/features/producto/views/producto-form-view.tsx");
    fs::write(&form, "// hand-written\n").unwrap();

    let result = scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask);

    assert!(
        result.is_err(),
        "the hand-written web file must not be overwritten"
    );
    assert_eq!(fs::read_to_string(&form).unwrap(), "// hand-written\n");
    cleanup(&dir);
}

#[test]
fn generate_web_is_the_last_step_of_the_layered_flow() {
    // `generate presentation` regenerates `core/generated/*` with the entity, whose screens do not
    // exist yet: the drift warning points at `generate web`, which writes them.
    let (dir, output) = web_project("web_generate_web");
    let factura = fields(&["amount:i64"]);
    scaffold::run_generate_domain("Factura", &output, &factura).unwrap();
    scaffold::run_generate_application("Factura", &output).unwrap();
    scaffold::run_generate_repository("Factura", &output).unwrap();
    scaffold::run_generate_presentation("Factura", &output).unwrap();
    assert!(
        validate::run_validate(&output).is_err(),
        "the screens are missing"
    );

    scaffold::run_generate_web("Factura", &output, OverwritePolicy::Ask).unwrap();

    for file in web_entity_files("factura") {
        assert!(output.join(&file).exists(), "{} missing", file.display());
    }
    assert!(validate::run_validate(&output).is_ok());
    let again = scaffold::run_generate_web("Factura", &output, OverwritePolicy::Ask);
    assert!(
        again.is_err(),
        "written files are the user's: --force to overwrite"
    );
    cleanup(&dir);
}

#[test]
fn generate_web_refuses_an_undeclared_entity_a_non_crud_one_and_a_project_without_web() {
    let (dir, output) = web_project("web_generate_web_refusals");

    let undeclared = scaffold::run_generate_web("Factura", &output, OverwritePolicy::Ask);
    let saludo = scaffold::run_generate_web("Saludo", &output, OverwritePolicy::Ask);
    scaffold::apply_no_web(&output).unwrap();
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    let non_web = scaffold::run_generate_web("Producto", &output, OverwritePolicy::Ask);

    assert!(
        undeclared
            .unwrap_err()
            .to_string()
            .contains("not found in puerto.toml")
    );
    assert!(
        saludo
            .unwrap_err()
            .to_string()
            .contains("no CRUD endpoints")
    );
    assert!(non_web.unwrap_err().to_string().contains("no frontend"));
    assert!(!output.join("web").exists());
    cleanup(&dir);
}

#[test]
fn the_api_client_is_written_once_and_never_overwritten() {
    let (dir, output) = web_project("web_client_once");
    let client = output.join("web/src/infrastructure/http/client.ts");
    assert!(
        !client.exists(),
        "a fresh project has no schema.d.ts for it to import"
    );

    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    fs::write(&client, "// customised\n").unwrap();
    scaffold::run_scaffold("Categoria", &output, &[], OverwritePolicy::Ask).unwrap();

    assert_eq!(fs::read_to_string(&client).unwrap(), "// customised\n");
    cleanup(&dir);
}

#[test]
fn validate_and_bootstrap_report_a_crud_entity_without_its_screens() {
    let (dir, output) = web_project("web_validate_drift");
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    fs::remove_dir_all(output.join("web/src/presentation/features/producto")).unwrap();

    let validated = validate::run_validate(&output);
    let bootstrap = scaffold::run_bootstrap_command(&output, false);

    for err in [validated.unwrap_err(), bootstrap.unwrap_err()] {
        let err = err.to_string();
        assert!(
            err.contains("web/src/presentation/features/producto/containers")
                && err.contains("puerto generar web Producto"),
            "{err}"
        );
    }
    cleanup(&dir);
}

#[test]
fn web_agents_rules_describe_the_slice_and_the_sync_step() {
    let web_agents = TEMPLATE_DIR_WEB_AGENTS;
    assert!(web_agents.contains("make web/sync"));
    assert!(web_agents.contains("puerto generar web"));
    assert!(web_agents.contains("test-support/"));
}

/// The frontend rules shipped to every web project (ADR-0010).
const TEMPLATE_DIR_WEB_AGENTS: &str = include_str!("../../template/web/AGENTS.md");

// ── Phase 15.1: routes the user writes ────────────────────────────────────

#[test]
fn new_web_project_has_a_routes_file_of_its_own() {
    // `core/generated/routes.tsx` is rewritten by every scaffold: a screen the user adds needs a
    // route table puerto never touches (the `web-route` snippet goes there).
    let (dir, output) = web_project("web_custom_routes");

    let custom = read(&output, "web/src/presentation/core/routes.tsx");
    assert!(custom.contains("export const customRoutes: ReactElement[] = [];"));
    assert!(!custom.contains("AUTO-GENERATED"));
    let generated = read(&output, "web/src/presentation/core/generated/routes.tsx");
    assert!(generated.contains("import { customRoutes } from \"../routes\";"));
    assert!(generated.contains("      {customRoutes}\n"));

    cleanup(&dir);
}

#[test]
fn generated_routes_match_custom_routes_before_the_entities() {
    let (dir, output) = web_project("web_custom_routes_pedido");
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();

    let generated = read(&output, "web/src/presentation/core/generated/routes.tsx");
    let custom = generated.find("{customRoutes}").unwrap();
    assert!(custom < generated.find("<Route path=\"/productos\">").unwrap());
    assert!(custom > generated.find("<HomePage").unwrap());

    cleanup(&dir);
}

// ── Phase 15.3: the form says what went wrong, next to the field ─────────

#[test]
fn messages_cover_every_code_of_errors_rs_and_name_the_field_it_is_about() {
    let (dir, output) = web_project("web_messages");
    let args = fields(&[
        "title:String",
        "sku:Sku:String",
        "status:Status:enum:Draft/Live",
        "note:opt:String",
        "count:i64",
    ]);
    scaffold::run_scaffold("Producto", &output, &args, OverwritePolicy::Ask).unwrap();

    let messages = read(&output, "web/src/domain/producto/messages.ts");
    let errors = read(&output, "contextos/producto/dominio/src/errores.rs");
    for line in errors.lines().filter(|l| l.contains("#[error(\"producto.")) {
        let code = line.split('"').nth(1).unwrap();
        if code.ends_with("{0}") {
            continue; // the field-specific validation codes are below
        }
        assert!(
            messages.contains(&format!("\"{code}\": \"")),
            "{code} has no message"
        );
    }
    assert!(messages.contains("\"producto.validacion.title_vacio\": \"Title is required\","));
    assert!(messages.contains("\"producto.validacion.title_vacio\": \"title\","));
    assert!(messages.contains("\"producto.sku_invalido\": \"Sku is not valid\","));
    assert!(messages.contains("\"producto.status_invalido\": \"status\","));
    assert!(
        messages
            .contains("\"producto.duplicado\": \"A producto with these values already exists\",")
    );
    assert!(!messages.contains("note_vacio"));
    assert!(
        messages.contains("return { field: FIELDS[code] ?? null, text: TEXTS[code] ?? code };")
    );

    let view = read(
        &output,
        "web/src/presentation/features/producto/views/producto-form-view.tsx",
    );
    assert!(view.contains("<FieldError problem={problem} field=\"title\" />"));
    assert!(view.contains("aria-invalid={problem?.field === \"status\"}"));
    let container = read(
        &output,
        "web/src/presentation/features/producto/containers/producto-form-container.tsx",
    );
    assert!(
        container
            .contains("const problem = failure === null ? null : toProblem(errorCode(failure));")
    );
    assert!(container.contains(
        "{problem?.field === null && <ErrorAlert error={failure} message={problem.text} />}"
    ));

    cleanup(&dir);
}

#[test]
fn messages_of_an_entity_without_field_codes_map_no_field() {
    let (dir, output) = web_project("web_messages_no_fields");
    scaffold::run_scaffold(
        "Counter",
        &output,
        &fields(&["count:i64"]),
        OverwritePolicy::Ask,
    )
    .unwrap();

    let messages = read(&output, "web/src/domain/counter/messages.ts");
    assert!(messages.contains("const FIELDS: Record<string, keyof CounterDraft> = {};"));

    cleanup(&dir);
}
