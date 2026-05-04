use std::{fs, path::Path};

use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::generators::types::resolve_type;
use crate::patchers::lib_rs::{patch_business_lib_domain_crud, patch_lib_block};
use crate::puerto_toml::Field;

fn effective_fields(fields: &[Field]) -> Vec<Field> {
    if fields.is_empty() {
        vec![Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
        }]
    } else {
        fields.to_vec()
    }
}

pub fn generate_model(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }

    let props_lines: Vec<String> = eff
        .iter()
        .map(|f| format!("    pub {}: {},", f.name, f.field_type))
        .collect();

    let mut entity_lines = vec![
        "    pub id: Uuid,".to_string(),
        "    pub created_at: DateTime<Utc>,".to_string(),
        "    pub updated_at: DateTime<Utc>,".to_string(),
        "    pub deleted: bool,".to_string(),
        "    pub deleted_at: Option<DateTime<Utc>>,".to_string(),
    ];
    for f in &eff {
        entity_lines.push(format!("    pub {}: {},", f.name, f.field_type));
    }

    let validations: Vec<String> = eff
        .iter()
        .filter(|f| f.field_type == "String")
        .map(|f| {
            format!(
                "        if props.{}.trim().is_empty() {{\n            return Err({}Error::ValidationError(\"{}_empty\".into()));\n        }}",
                f.name, pascal, f.name
            )
        })
        .collect();

    let new_assignments: Vec<String> = eff
        .iter()
        .map(|f| format!("            {}: props.{},", f.name, f.name))
        .collect();

    let required_string_fields: Vec<&Field> =
        eff.iter().filter(|f| f.field_type == "String").collect();

    let valid_props_lines: Vec<String> = eff
        .iter()
        .map(|f| {
            let mapping = resolve_type(&f.field_type).unwrap();
            format!("            {}: {},", f.name, mapping.default_expr)
        })
        .collect();

    let valid_test_name =
        if eff.len() == 1 && eff[0].name == "name" && eff[0].field_type == "String" {
            format!("should_create_{}_when_name_is_valid", snake)
        } else {
            format!("should_create_{}_when_fields_are_valid", snake)
        };

    let valid_assertion = if !eff.is_empty() && eff[0].field_type == "String" {
        format!(
            "\n        assert_eq!(result.unwrap().{}, \"example\");",
            eff[0].name
        )
    } else if !eff.is_empty() {
        "\n        assert!(result.is_ok());".to_string()
    } else {
        String::new()
    };

    let mut validation_tests: Vec<String> = vec![];
    for sf in &required_string_fields {
        let field_name = sf.name.clone();

        let empty_props: Vec<String> = eff
            .iter()
            .map(|f| {
                if f.name == field_name {
                    format!("            {}: \"\".into(),", f.name)
                } else {
                    let mapping = resolve_type(&f.field_type).unwrap();
                    format!("            {}: {},", f.name, mapping.default_expr)
                }
            })
            .collect();

        validation_tests.push(format!(
            "    #[test]
    fn should_reject_{snake}_when_{field}_is_empty() {{
        let result = {pascal}::new({pascal}Props {{
{props}
        }});
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            \"{snake}.validation_error.{field}_empty\"
        );
    }}",
            snake = snake,
            pascal = pascal,
            field = field_name,
            props = empty_props.join("\n"),
        ));

        let ws_props: Vec<String> = eff
            .iter()
            .map(|f| {
                if f.name == field_name {
                    format!("            {}: \"   \".into(),", f.name)
                } else {
                    let mapping = resolve_type(&f.field_type).unwrap();
                    format!("            {}: {},", f.name, mapping.default_expr)
                }
            })
            .collect();

        validation_tests.push(format!(
            "    #[test]
    fn should_reject_{snake}_when_{field}_is_only_whitespace() {{
        let result = {pascal}::new({pascal}Props {{
{props}
        }});
        assert!(result.is_err());
    }}",
            snake = snake,
            pascal = pascal,
            field = field_name,
            props = ws_props.join("\n"),
        ));
    }

    let extra_imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let props_str = props_lines.join("\n");
    let entity_str = entity_lines.join("\n");
    let validations_str = if validations.is_empty() {
        String::new()
    } else {
        validations.join("\n") + "\n"
    };
    let new_assignments_str = new_assignments.join("\n");
    let valid_props_str = valid_props_lines.join("\n");
    let validation_tests_str = if validation_tests.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", validation_tests.join("\n\n"))
    };

    format!(
        "use chrono::{{DateTime, Utc}};
use uuid::Uuid;{extra_imports}

use super::errors::{pascal}Error;

#[derive(Debug, Clone)]
pub struct {pascal}Props {{
{props}
}}

#[derive(Debug, Clone)]
pub struct {pascal} {{
{entity}
}}

impl {pascal} {{
    pub fn new(props: {pascal}Props) -> Result<Self, {pascal}Error> {{
{validations}        let now = chrono::Utc::now();
        Ok(Self {{
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            deleted: false,
            deleted_at: None,
{new_assignments}
        }})
    }}

    pub fn from_repository(data: {pascal}) -> Self {{
        data
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn {valid_test_name}() {{
        let result = {pascal}::new({pascal}Props {{
{valid_props}
        }});{assertion}
    }}
{validation_tests}
}}
",
        extra_imports = extra_imports_str,
        pascal = pascal,
        props = props_str,
        entity = entity_str,
        validations = validations_str,
        new_assignments = new_assignments_str,
        valid_test_name = valid_test_name,
        valid_props = valid_props_str,
        assertion = valid_assertion,
        validation_tests = validation_tests_str,
    )
}

pub fn generate_mother(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let mut mother_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if !mother_imports.contains(&stmt) {
                    mother_imports.push(stmt);
                }
            }
        }
    }

    let required_string_fields: Vec<&Field> =
        eff.iter().filter(|f| f.field_type == "String").collect();

    let mother_fields: Vec<String> = eff
        .iter()
        .map(|f| {
            let storage_type = mother_storage_type(&f.field_type);
            format!("    {}: Option<{}>,", f.name, storage_type)
        })
        .collect();

    let with_methods: Vec<String> = eff
        .iter()
        .map(|f| {
            let (param_type, conversion) = mother_with_param(&f.field_type, &f.name);
            format!(
                "    pub fn with_{field}(mut self, {field}: {param_type}) -> Self {{\n        self.{field} = Some({conversion});\n        self\n    }}",
                field = f.name,
                param_type = param_type,
                conversion = conversion,
            )
        })
        .collect();

    let empty_methods: Vec<String> = required_string_fields
        .iter()
        .map(|f| {
            format!(
                "    pub fn with_empty_{field}(mut self) -> Self {{\n        self.{field} = Some(String::new());\n        self\n    }}",
                field = f.name,
            )
        })
        .collect();

    let build_assignments: Vec<String> = eff
        .iter()
        .map(|f| {
            let mapping = resolve_type(&f.field_type).unwrap();
            if is_option_type(&f.field_type) {
                format!("            {}: self.{},", f.name, f.name)
            } else {
                match mapping.rust_type {
                    "String" => format!(
                        "            {}: self.{}.unwrap_or_else(|| \"example\".to_string()),",
                        f.name, f.name
                    ),
                    "i64" => format!("            {}: self.{}.unwrap_or(42),", f.name, f.name),
                    "bool" => format!("            {}: self.{}.unwrap_or(true),", f.name, f.name),
                    "f64" => format!("            {}: self.{}.unwrap_or(1.5),", f.name, f.name),
                    "Uuid" => format!(
                        "            {}: self.{}.unwrap_or_else(Uuid::new_v4),",
                        f.name, f.name
                    ),
                    "DateTime<Utc>" => format!(
                        "            {}: self.{}.unwrap_or_else(Utc::now),",
                        f.name, f.name
                    ),
                    "Vec<String>" | "Vec<i64>" => format!(
                        "            {}: self.{}.unwrap_or_default(),",
                        f.name, f.name
                    ),
                    "HashMap<String, String>" => format!(
                        "            {}: self.{}.unwrap_or_default(),",
                        f.name, f.name
                    ),
                    _ => format!(
                        "            {}: self.{}.unwrap_or_default(),",
                        f.name, f.name
                    ),
                }
            }
        })
        .collect();

    let props_assignments: Vec<String> = build_assignments.clone();

    let imports_str = if mother_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", mother_imports.join("\n"))
    };

    let fields_str = mother_fields.join("\n");
    let with_methods_str = with_methods.join("\n\n");
    let empty_methods_str = if empty_methods.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", empty_methods.join("\n\n"))
    };
    let build_assignments_str = build_assignments.join("\n");
    let props_assignments_str = props_assignments.join("\n");

    format!(
        "use crate::domain::{snake}::model::{pascal};
use crate::domain::{snake}::model::{pascal}Props;{imports}

pub struct {pascal}Mother {{
{fields}
}}

impl {pascal}Mother {{
    pub fn new() -> Self {{
        Self {{ {defaults} }}
    }}

{with_methods}{empty_methods}

    pub fn build(self) -> {pascal} {{
        {pascal}::new({pascal}Props {{
{build_assignments}
        }})
        .expect(\"{pascal}Mother: failed to build valid {pascal}\")
    }}

    pub fn build_props(self) -> {pascal}Props {{
        {pascal}Props {{
{props_assignments}
        }}
    }}

    pub fn random() -> {pascal} {{
        Self::new().build()
    }}

    pub fn random_vec(n: usize) -> Vec<{pascal}> {{
        (0..n).map(|_| Self::random()).collect()
    }}
}}
",
        imports = imports_str,
        pascal = pascal,
        snake = snake,
        fields = fields_str,
        defaults = eff
            .iter()
            .map(|f| format!("{}: None", f.name))
            .collect::<Vec<_>>()
            .join(", "),
        with_methods = with_methods_str,
        empty_methods = empty_methods_str,
        build_assignments = build_assignments_str,
        props_assignments = props_assignments_str,
    )
}

fn is_option_type(field_type: &str) -> bool {
    field_type.starts_with("Option<")
}

fn mother_storage_type(field_type: &str) -> String {
    if is_option_type(field_type) {
        let inner = field_type
            .strip_prefix("Option<")
            .unwrap()
            .strip_suffix('>')
            .unwrap();
        inner.to_string()
    } else {
        field_type.to_string()
    }
}

fn mother_with_param(field_type: &str, field_name: &str) -> (String, String) {
    if is_option_type(field_type) {
        let inner = field_type
            .strip_prefix("Option<")
            .unwrap()
            .strip_suffix('>')
            .unwrap();
        match inner {
            "String" => ("&str".to_string(), format!("{}.to_string()", field_name)),
            "DateTime<Utc>" => ("DateTime<Utc>".to_string(), field_name.to_string()),
            _ => (inner.to_string(), field_name.to_string()),
        }
    } else {
        match field_type {
            "String" => ("&str".to_string(), format!("{}.to_string()", field_name)),
            "DateTime<Utc>" => ("DateTime<Utc>".to_string(), field_name.to_string()),
            _ => (field_type.to_string(), field_name.to_string()),
        }
    }
}

pub(crate) const ERRORS: &str = r#"use thiserror::Error;

#[derive(Debug, Error)]
pub enum {Pascal}Error {
    #[error("{snake}.validation_error.{0}")]
    ValidationError(String),
    #[error("{snake}.not_found")]
    NotFound,
    #[error("{snake}.duplicate")]
    Duplicate,
    #[error("{snake}.repository_error")]
    RepositoryError,
    #[error("{snake}.unknown")]
    Unknown,
}
"#;

const CRUD_REPOSITORY: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use super::{errors::{Pascal}Error, model::{Pascal}};

#[async_trait]
pub trait {Pascal}RepositoryTrait: Send + Sync {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use uuid::Uuid;

    use super::*;

    mock! {
        pub {Pascal}Repository {}

        #[async_trait]
        impl {Pascal}RepositoryTrait for {Pascal}Repository {
            async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
            async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
            async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
        }
    }
}
"#;

pub fn generate_create_use_case_trait(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }

    let params_fields: Vec<String> = eff
        .iter()
        .filter(|f| f.field_type != "Uuid")
        .map(|f| format!("    pub {}: {},", f.name, f.field_type))
        .collect();

    let imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let params_fields_str = params_fields.join("\n");

    format!(
        r#"use async_trait::async_trait;{imports}

use crate::domain::{snake}::{{errors::{pascal}Error, model::{pascal}}};

#[derive(Debug, Clone)]
pub struct Create{pascal}Params {{
{params_fields}
}}

#[async_trait]
pub trait Create{pascal}UseCaseTrait: Send + Sync {{
    async fn execute(&self, params: Create{pascal}Params) -> Result<{pascal}, {pascal}Error>;
}}
"#,
        imports = imports_str,
        pascal = pascal,
        snake = snake,
        params_fields = params_fields_str,
    )
}

pub(crate) const USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Create{Pascal}Params {
    pub name: String,
}

#[async_trait]
pub trait Create{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const GET_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Get{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Get{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Get{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const LIST_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug)]
pub struct List{Pascal}Params;

#[async_trait]
pub trait List{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: List{Pascal}Params) -> Result<Vec<{Pascal}>, {Pascal}Error>;
}
"#;

const UPDATE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Update{Pascal}Params {
    pub id: Uuid,
    pub name: String,
}

#[async_trait]
pub trait Update{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Update{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const DELETE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct Delete{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Delete{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Delete{Pascal}Params) -> Result<(), {Pascal}Error>;
}
"#;

pub fn generate_update_use_case_trait(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let mut extra_imports: Vec<String> = vec![];
    for f in &eff {
        if let Ok(mapping) = resolve_type(&f.field_type) {
            if let Some(imp) = mapping.needs_import {
                let stmt = format!("use {};", imp);
                if !extra_imports.contains(&stmt) {
                    extra_imports.push(stmt);
                }
            }
        }
    }

    let mut params_fields = vec!["    pub id: Uuid,".to_string()];
    for f in &eff {
        if f.field_type != "Uuid" {
            params_fields.push(format!("    pub {}: {},", f.name, f.field_type));
        }
    }

    let imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let params_fields_str = params_fields.join("\n");

    format!(
        r#"use async_trait::async_trait;
use uuid::Uuid;{imports}

use crate::domain::{snake}::{{errors::{pascal}Error, model::{pascal}}};

#[derive(Debug, Clone)]
pub struct Update{pascal}Params {{
{params_fields}
}}

#[async_trait]
pub trait Update{pascal}UseCaseTrait: Send + Sync {{
    async fn execute(&self, params: Update{pascal}Params) -> Result<{pascal}, {pascal}Error>;
}}
"#,
        imports = imports_str,
        pascal = pascal,
        snake = snake,
        params_fields = params_fields_str,
    )
}

pub fn write_domain_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &generate_model(pascal, snake, fields),
    )?;
    write_file(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &apply(ERRORS, pascal, snake),
    )?;
    write_file(
        &base.join(format!("business/src/domain/{snake}/repository.rs")),
        &apply(CRUD_REPOSITORY, pascal, snake),
    )?;
    let create_uc = if fields.is_empty() {
        apply(USE_CASE_TRAIT, pascal, snake)
    } else {
        generate_create_use_case_trait(pascal, snake, fields)
    };
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &create_uc,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/get_{snake}.rs"
        )),
        &apply(GET_USE_CASE_TRAIT, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/list_{snake}.rs"
        )),
        &apply(LIST_USE_CASE_TRAIT, pascal, snake),
    )?;
    let update_uc = if fields.is_empty() {
        apply(UPDATE_USE_CASE_TRAIT, pascal, snake)
    } else {
        generate_update_use_case_trait(pascal, snake, fields)
    };
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/update_{snake}.rs"
        )),
        &update_uc,
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/delete_{snake}.rs"
        )),
        &apply(DELETE_USE_CASE_TRAIT, pascal, snake),
    )?;
    Ok(())
}

pub fn write_mother(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/tests/mothers/{snake}_mother.rs")),
        &generate_mother(pascal, snake, fields),
    )?;
    Ok(())
}

pub fn patch_mothers_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake}_mother;")) {
        return Ok(());
    }

    let new_mod = format!("\n        pub mod {snake}_mother;\n");

    if let Ok(patched) = patch_lib_block(&src, &["tests", "mothers"], &new_mod) {
        fs::write(&path, patched)?;
        return Ok(());
    }

    let mut content = src;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n#[cfg(test)]\npub mod tests {{\n    pub mod mothers {{\n        pub mod {snake}_mother;\n    }}\n}}\n"
    ));
    fs::write(&path, content)?;
    Ok(())
}

pub fn run_generate_domain(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} is already in puerto.toml. Use `puerto generate use-case` to add a use case."
        )
        .into());
    }

    let fields: Vec<Field> = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_domain_files(&pascal, &snake, base, &fields)?;
    write_mother(&pascal, &snake, base, &fields)?;
    patch_business_lib_domain_crud(base, &snake)?;
    patch_mothers_lib(base, &snake)?;

    let use_cases = vec![
        format!("create_{snake}"),
        format!("get_{snake}"),
        format!("list_{snake}"),
        format!("update_{snake}"),
        format!("delete_{snake}"),
    ];
    crate::puerto_toml::add_entity(base, &pascal, use_cases, config.project.db, vec![])?;

    println!("✓ business/domain/    — model, errors, repository trait, 5 use case traits");
    println!("✓ business/tests/     — {pascal}Mother (Object Mother)");
    println!("✓ puerto.toml         — {pascal} registered");
    println!();
    println!("  Next: puerto generate application {pascal}");
    Ok(())
}
