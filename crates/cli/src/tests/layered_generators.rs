use super::*;

// ── puerto generate domain / application / repository / presentation ─────

#[test]
fn generate_domain_creates_domain_files_and_mother() {
    let dir = temp_dir("gen_domain");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    scaffold::run_generate_domain("Widget", &output, &[]).unwrap();

    assert!(output.join("business/src/domain/widget/model.rs").exists());
    assert!(output.join("business/src/domain/widget/errors.rs").exists());
    assert!(
        output
            .join("business/src/domain/widget/repository.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/domain/widget/use_cases/create_widget.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/domain/widget/use_cases/delete_widget.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/tests/mothers/widget_mother.rs")
            .exists()
    );

    let lib = fs::read_to_string(output.join("business/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod widget"));
    assert!(lib.contains("pub mod widget_mother;"));

    let toml = fs::read_to_string(output.join("puerto.toml")).unwrap();
    assert!(toml.contains("Widget"));

    cleanup(&dir);
}

// ── T-06: `generate domain` accepts typed fields ──────────────────────────

#[test]
fn generate_domain_writes_cli_fields_into_model_and_manifest() {
    let dir = temp_dir("gen_domain_fields");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    let fields = vec![
        puerto_toml::parse_field_arg("amount:i64").unwrap(),
        puerto_toml::parse_field_arg("status:Status:enum:Draft/Paid").unwrap(),
    ];
    scaffold::run_generate_domain("Invoice", &output, &fields).unwrap();

    let model = fs::read_to_string(output.join("business/src/domain/invoice/model.rs")).unwrap();
    assert!(model.contains("pub amount: i64"), "got:\n{model}");
    assert!(model.contains("pub status: Status"), "got:\n{model}");

    // The manifest is what makes the *later* layered commands field-aware.
    let config = puerto_toml::read(&output).unwrap();
    let invoice = config
        .entity
        .iter()
        .find(|e| e.name == "Invoice")
        .expect("Invoice in puerto.toml");
    let names: Vec<&str> = invoice.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["amount", "status"]);

    cleanup(&dir);
}

#[test]
fn generate_domain_fields_reach_the_later_layers() {
    // The whole point of persisting fields in `generate domain`: application, repository and
    // presentation all read them back from puerto.toml. They used to be written as `vec![]`, so
    // the entire domain-first workflow could only ever produce the default `name: String`.
    let dir = temp_dir("gen_domain_fields_layers");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    let fields = vec![puerto_toml::parse_field_arg("amount:i64").unwrap()];
    scaffold::run_generate_domain("Invoice", &output, &fields).unwrap();
    scaffold::run_generate_application("Invoice", &output).unwrap();
    scaffold::run_generate_repository("Invoice", &output).unwrap();
    scaffold::run_generate_presentation("Invoice", &output).unwrap();

    let impl_rs =
        fs::read_to_string(output.join("business/src/application/invoice/create_invoice.rs"))
            .unwrap();
    assert!(impl_rs.contains("amount"), "got:\n{impl_rs}");

    let dto = fs::read_to_string(output.join("presentation/src/api/invoice/dto.rs")).unwrap();
    assert!(dto.contains("pub amount: i64"), "got:\n{dto}");

    cleanup(&dir);
}

#[test]
fn generate_domain_rejects_conflicting_value_object_types() {
    let dir = temp_dir("gen_domain_vo_conflict");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    let fields = vec![
        puerto_toml::parse_field_arg("a:Code:String").unwrap(),
        puerto_toml::parse_field_arg("b:Code:i64").unwrap(),
    ];
    let result = scaffold::run_generate_domain("Invoice", &output, &fields);

    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("Code"),
        "error should name the conflicting value object"
    );

    cleanup(&dir);
}

#[test]
fn generate_application_creates_use_case_impls() {
    let dir = temp_dir("gen_application");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    scaffold::run_generate_domain("Widget", &output, &[]).unwrap();
    scaffold::run_generate_application("Widget", &output).unwrap();

    assert!(
        output
            .join("business/src/application/widget/create_widget.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/application/widget/get_widget.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/application/widget/list_widgets.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/application/widget/update_widget.rs")
            .exists()
    );
    assert!(
        output
            .join("business/src/application/widget/delete_widget.rs")
            .exists()
    );

    let lib = fs::read_to_string(output.join("business/src/lib.rs")).unwrap();
    assert!(lib.contains("pub mod create_widget;"));

    cleanup(&dir);
}

#[test]
fn generate_application_errors_without_prior_domain() {
    let dir = temp_dir("gen_app_no_domain");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    let result = scaffold::run_generate_application("Ghost", &output);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not found in puerto.toml")
    );

    cleanup(&dir);
}

#[test]
fn generate_repository_creates_infra_files() {
    let dir = temp_dir("gen_repository");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    scaffold::run_generate_domain("Widget", &output, &[]).unwrap();
    scaffold::run_generate_repository("Widget", &output).unwrap();

    assert!(
        output
            .join("infrastructure/src/widget/repository.rs")
            .exists()
    );
    let repo = fs::read_to_string(output.join("infrastructure/src/widget/repository.rs")).unwrap();
    assert!(repo.contains("InMemoryWidgetRepository"));

    cleanup(&dir);
}

#[test]
fn generate_presentation_creates_all_files_and_regenerates_bootstrap() {
    let dir = temp_dir("gen_presentation");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    scaffold::run_generate_domain("Widget", &output, &[]).unwrap();
    scaffold::run_generate_repository("Widget", &output).unwrap();
    scaffold::run_generate_presentation("Widget", &output).unwrap();

    assert!(output.join("presentation/src/api/widget.rs").exists());
    assert!(
        output
            .join("presentation/src/api/widget/routes.rs")
            .exists()
    );
    assert!(output.join("presentation/src/api/widget/dto.rs").exists());
    assert!(
        output
            .join("presentation/src/api/widget/responses.rs")
            .exists()
    );
    assert!(
        output
            .join("presentation/src/api/widget/error_mapper.rs")
            .exists()
    );

    let bootstrap =
        fs::read_to_string(output.join("presentation/src/generated/bootstrap.rs")).unwrap();
    assert!(bootstrap.contains("WidgetApi"));

    cleanup(&dir);
}

#[test]
fn generate_presentation_errors_without_prior_domain() {
    let dir = temp_dir("gen_pres_no_domain");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    let result = scaffold::run_generate_presentation("Ghost", &output);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("not found in puerto.toml")
    );

    cleanup(&dir);
}

#[test]
fn generate_scaffold_includes_object_mother() {
    let dir = temp_dir("scaffold_with_mother");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("myapp", &dir).unwrap();

    scaffold::run("Widget", &output, false, true, &[], &[]).unwrap();

    assert!(
        output
            .join("business/src/tests/mothers/widget_mother.rs")
            .exists()
    );
    let mother =
        fs::read_to_string(output.join("business/src/tests/mothers/widget_mother.rs")).unwrap();
    assert!(mother.contains("WidgetMother"));
    assert!(mother.contains("pub fn random()"));

    cleanup(&dir);
}
