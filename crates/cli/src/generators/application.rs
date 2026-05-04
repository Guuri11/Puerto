use std::path::Path;

use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::generators::types::resolve_type;
use crate::patchers::lib_rs::patch_business_lib_application_crud;
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

fn test_props_lines(eff: &[Field], string_override: &str) -> String {
    eff.iter()
        .map(|f| {
            let mapping = resolve_type(&f.field_type).unwrap();
            let value = if f.field_type == "String" {
                format!("\"{}\".to_string()", string_override)
            } else {
                mapping.default_expr.to_string()
            };
            format!("            {}: {},", f.name, value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_create_params_lines(eff: &[Field]) -> String {
    eff.iter()
        .filter(|f| f.field_type != "Uuid")
        .map(|f| {
            let mapping = resolve_type(&f.field_type).unwrap();
            format!("            {}: {},", f.name, mapping.default_expr)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_create_params_lines_with_empty(eff: &[Field], empty_field: &str) -> String {
    eff.iter()
        .filter(|f| f.field_type != "Uuid")
        .map(|f| {
            if f.name == empty_field {
                format!("            {}: \"\".to_string(),", f.name)
            } else {
                let mapping = resolve_type(&f.field_type).unwrap();
                format!("            {}: {},", f.name, mapping.default_expr)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_update_params_lines(eff: &[Field]) -> String {
    let mut lines = vec!["            id: entity_id,".to_string()];
    for f in eff.iter().filter(|f| f.field_type != "Uuid") {
        let value = if f.field_type == "String" {
            "\"updated\".to_string()".to_string()
        } else {
            resolve_type(&f.field_type)
                .unwrap()
                .default_expr
                .to_string()
        };
        lines.push(format!("            {}: {},", f.name, value));
    }
    lines.join("\n")
}

fn test_update_params_lines_with_empty(eff: &[Field], empty_field: &str) -> String {
    let mut lines = vec!["            id: entity_id,".to_string()];
    for f in eff.iter().filter(|f| f.field_type != "Uuid") {
        if f.name == empty_field {
            lines.push(format!("            {}: \"\".to_string(),", f.name));
        } else {
            let value = if f.field_type == "String" {
                "\"updated\".to_string()".to_string()
            } else {
                resolve_type(&f.field_type)
                    .unwrap()
                    .default_expr
                    .to_string()
            };
            lines.push(format!("            {}: {},", f.name, value));
        }
    }
    lines.join("\n")
}

pub fn generate_create_use_case_impl(pascal: &str, snake: &str, fields: &[Field]) -> String {
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

    let props_fields: Vec<String> = eff
        .iter()
        .filter(|f| f.field_type != "Uuid")
        .map(|f| format!("            {}: params.{},", f.name, f.name))
        .collect();

    let log_ident = format!("params.{}", eff[0].name);

    let imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let props_fields_str = props_fields.join("\n");

    let mut s = String::new();
    s.push_str("use std::sync::Arc;\n\nuse async_trait::async_trait;");
    s.push_str(&imports_str);
    s.push_str(&format!("\n\nuse crate::domain::{snake}::{{\n    errors::{pascal}Error,\n    model::{{{pascal}, {pascal}Props}},\n    repository::{pascal}RepositoryTrait,\n    use_cases::create_{snake}::{{Create{pascal}Params, Create{pascal}UseCaseTrait}},\n}};\nuse crate::domain::logger::LoggerTrait;\n\npub struct Create{pascal}UseCaseImpl {{\n    pub repository: Arc<dyn {pascal}RepositoryTrait>,\n    pub logger: Arc<dyn LoggerTrait>,\n}}\n\n#[async_trait]\nimpl Create{pascal}UseCaseTrait for Create{pascal}UseCaseImpl {{\n    async fn execute(&self, params: Create{pascal}Params) -> Result<{pascal}, {pascal}Error> {{\n"));
    s.push_str(&format!(
        "        self.logger.info(&format!(\"Creating {snake}: {}\"));\n",
        log_ident
    ));
    s.push_str(&format!("        let entity = {pascal}::new({pascal}Props {{\n{props_fields}\n        }}).map_err(|e| {{\n            self.logger.warn(&e.to_string());\n            e\n        }})?;\n        self.repository.save(&entity).await.map_err(|e| {{\n            self.logger.error(&e.to_string());\n            e\n        }})?;\n        self.logger.info(&format!(\"{pascal} created\"));\n        Ok(entity)\n    }}\n}}\n", pascal=pascal, props_fields=props_fields_str));

    let string_fields: Vec<&Field> = eff.iter().filter(|f| f.field_type == "String").collect();
    let first_string_field: Option<&Field> = string_fields.first().copied();
    let valid_test_name =
        if eff.len() == 1 && eff[0].name == "name" && eff[0].field_type == "String" {
            format!("should_create_{snake}_when_name_is_valid")
        } else {
            format!("should_create_{snake}_when_fields_are_valid")
        };
    let valid_assertion = if let Some(f) = first_string_field {
        format!(
            "\n        assert_eq!(result.unwrap().{}, \"example\");",
            f.name
        )
    } else {
        "\n        assert!(result.is_ok());".to_string()
    };
    let valid_params = test_create_params_lines(&eff);
    let mut empty_tests: Vec<String> = vec![];
    for sf in &string_fields {
        let empty_params = test_create_params_lines_with_empty(&eff, &sf.name);
        let test_name = format!("should_return_error_when_{}_is_empty", sf.name);
        empty_tests.push(format!(
            "    #[tokio::test]
    async fn {test_name}() {{
        // Arrange
        let mock_repo = Mock{pascal}Repository::new();
        let use_case = Create{pascal}UseCaseImpl {{
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        }};

        // Act
        let result = use_case
            .execute(Create{pascal}Params {{
{empty_params}
            }})
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            \"{snake}.validation_error.{field}_empty\"
        );
    }}",
            test_name = test_name,
            pascal = pascal,
            empty_params = empty_params,
            snake = snake,
            field = sf.name,
        ));
    }
    let empty_tests_str = if empty_tests.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", empty_tests.join("\n\n"))
    };

    s.push_str(&format!(
        "
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{pascal}Repository;
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {{
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }}

    #[tokio::test]
    async fn {valid_test_name}() {{
        // Arrange
        let mut mock_repo = Mock{pascal}Repository::new();
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Create{pascal}UseCaseImpl {{
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        }};

        // Act
        let result = use_case
            .execute(Create{pascal}Params {{
{valid_params}
            }})
            .await;

        // Assert
        assert!(result.is_ok());{valid_assertion}
    }}{empty_tests}
}}
",
        snake = snake,
        pascal = pascal,
        valid_test_name = valid_test_name,
        valid_params = valid_params,
        valid_assertion = valid_assertion,
        empty_tests = empty_tests_str,
    ));
    s
}

pub fn generate_update_use_case_impl(pascal: &str, snake: &str, fields: &[Field]) -> String {
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

    let validations: Vec<String> = eff
        .iter()
        .filter(|f| f.field_type == "String")
        .map(|f| {
            format!(
                "        if params.{}.trim().is_empty() {{\n            let err = {}Error::ValidationError(\"{}_empty\".into());\n            self.logger.warn(&err.to_string());\n            return Err(err);\n        }}",
                f.name, pascal, f.name
            )
        })
        .collect();

    let assignments: Vec<String> = eff
        .iter()
        .filter(|f| f.field_type != "Uuid")
        .map(|f| format!("        entity.{} = params.{};", f.name, f.name))
        .collect();

    let imports_str = if extra_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", extra_imports.join("\n"))
    };

    let validations_str = if validations.is_empty() {
        String::new()
    } else {
        validations.join("\n") + "\n"
    };

    let assignments_str = if assignments.is_empty() {
        String::new()
    } else {
        format!("\n{}", assignments.join("\n"))
    };

    let mut s = String::new();
    s.push_str("use std::sync::Arc;\n\nuse async_trait::async_trait;");
    s.push_str(&imports_str);
    s.push_str(&format!("\n\nuse crate::domain::{snake}::{{\n    errors::{pascal}Error,\n    model::{pascal},\n    repository::{pascal}RepositoryTrait,\n    use_cases::update_{snake}::{{Update{pascal}Params, Update{pascal}UseCaseTrait}},\n}};\nuse crate::domain::logger::LoggerTrait;\n\npub struct Update{pascal}UseCaseImpl {{\n    pub repository: Arc<dyn {pascal}RepositoryTrait>,\n    pub logger: Arc<dyn LoggerTrait>,\n}}\n\n#[async_trait]\nimpl Update{pascal}UseCaseTrait for Update{pascal}UseCaseImpl {{\n    async fn execute(&self, params: Update{pascal}Params) -> Result<{pascal}, {pascal}Error> {{\n", snake=snake, pascal=pascal));
    s.push_str(&format!("        self.logger.info(&format!(\"Updating {snake}: {{}}\", params.id));\n        let mut entity = self\n            .repository\n            .find_by_id(params.id)\n            .await\n            .map_err(|e| {{\n                self.logger.error(&e.to_string());\n                e\n            }})?\n            .ok_or_else(|| {{\n                let err = {pascal}Error::NotFound;\n                self.logger.warn(&err.to_string());\n                err\n            }})?;\n", snake=snake, pascal=pascal));
    s.push_str(&validations_str);
    s.push_str(&assignments_str);
    s.push_str("\n        entity.updated_at = chrono::Utc::now();\n        self.repository.save(&entity).await.map_err(|e| {\n            self.logger.error(&e.to_string());\n            e\n        })?;\n        Ok(entity)\n    }\n}\n");

    let string_fields: Vec<&Field> = eff.iter().filter(|f| f.field_type == "String").collect();
    let first_string_field: Option<&Field> = string_fields.first().copied();
    let original_props = test_props_lines(&eff, "original");
    let update_params = test_update_params_lines(&eff);
    let not_found_update_params = {
        let mut lines = vec!["            id: Uuid::new_v4(),".to_string()];
        for f in eff.iter().filter(|f| f.field_type != "Uuid") {
            let mapping = resolve_type(&f.field_type).unwrap();
            let value = if f.field_type == "String" {
                "\"new\".to_string()".to_string()
            } else {
                mapping.default_expr.to_string()
            };
            lines.push(format!("            {}: {},", f.name, value));
        }
        lines.join("\n")
    };

    let mut empty_update_tests: Vec<String> = vec![];
    for sf in &string_fields {
        let empty_params = test_update_params_lines_with_empty(&eff, &sf.name);
        let test_name = format!("should_return_error_when_{}_is_empty", sf.name);
        empty_update_tests.push(format!(
            "    #[tokio::test]
    async fn {test_name}() {{
        // Arrange
        let entity = {pascal}::new({pascal}Props {{
{original_props}
        }}).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        let use_case = Update{pascal}UseCaseImpl {{
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        }};

        // Act
        let result = use_case
            .execute(Update{pascal}Params {{
{empty_params}
            }})
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            \"{snake}.validation_error.{field}_empty\"
        );
    }}",
            test_name = test_name,
            pascal = pascal,
            original_props = original_props,
            empty_params = empty_params,
            snake = snake,
            field = sf.name,
        ));
    }
    let empty_tests_str = if empty_update_tests.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", empty_update_tests.join("\n\n"))
    };

    let valid_assertion = if let Some(f) = first_string_field {
        format!("assert_eq!(result.unwrap().{}, \"updated\");", f.name)
    } else {
        "assert!(result.is_ok());".to_string()
    };

    s.push_str(&format!(
        "
#[cfg(test)]
mod tests {{
    use super::*;
    use crate::domain::{snake}::{{
        model::{{{pascal}, {pascal}Props}},
        repository::mocks::Mock{pascal}Repository,
    }};
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {{
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }}

    #[tokio::test]
    async fn should_update_{snake}_when_params_are_valid() {{
        // Arrange
        let entity = {pascal}::new({pascal}Props {{
{original_props}
        }}).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Update{pascal}UseCaseImpl {{
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        }};

        // Act
        let result = use_case
            .execute(Update{pascal}Params {{
{update_params}
            }})
            .await;

        // Assert
        {valid_assertion}
    }}

    #[tokio::test]
    async fn should_return_not_found_when_{snake}_does_not_exist() {{
        // Arrange
        let mut mock_repo = Mock{pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Update{pascal}UseCaseImpl {{
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        }};

        // Act
        let result = use_case
            .execute(Update{pascal}Params {{
{not_found_update_params}
            }})
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), \"{snake}.not_found\");
    }}{empty_tests}
}}
",
        snake = snake,
        pascal = pascal,
        original_props = original_props,
        update_params = update_params,
        not_found_update_params = not_found_update_params,
        valid_assertion = valid_assertion,
        empty_tests = empty_tests_str,
    ));
    s
}

pub fn generate_get_use_case_impl(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let props_fields = test_props_lines(&eff, "example");
    let first_string_field: Option<&Field> = eff.iter().find(|f| f.field_type == "String");
    let found_assertion = if let Some(f) = first_string_field {
        format!(
            "\n        assert_eq!(result.unwrap().{}, \"example\");",
            f.name
        )
    } else {
        "\n        assert!(result.is_ok());".to_string()
    };

    let mut s = String::new();
    s.push_str("use std::sync::Arc;\n\nuse async_trait::async_trait;\n");
    s.push_str(&format!("\nuse crate::domain::{snake}::{{\n    errors::{pascal}Error,\n    model::{pascal},\n    repository::{pascal}RepositoryTrait,\n    use_cases::get_{snake}::{{Get{pascal}Params, Get{pascal}UseCaseTrait}},\n}};\nuse crate::domain::logger::LoggerTrait;\n\npub struct Get{pascal}UseCaseImpl {{\n    pub repository: Arc<dyn {pascal}RepositoryTrait>,\n    pub logger: Arc<dyn LoggerTrait>,\n}}\n\n#[async_trait]\nimpl Get{pascal}UseCaseTrait for Get{pascal}UseCaseImpl {{\n    async fn execute(&self, params: Get{pascal}Params) -> Result<{pascal}, {pascal}Error> {{\n", snake=snake, pascal=pascal));
    s.push_str(&format!(
        "        self.logger.info(&format!(\"Getting {snake}: {{}}\", params.id));\n        let result = self.repository.find_by_id(params.id).await.map_err(|e| {{\n            self.logger.error(&e.to_string());\n            e\n        }})?;\n        result.ok_or_else(|| {{\n            let err = {pascal}Error::NotFound;\n            self.logger.warn(&err.to_string());\n            err\n        }})\n    }}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n    use crate::domain::{snake}::{{\n        model::{{{pascal}, {pascal}Props}},\n        repository::mocks::Mock{pascal}Repository,\n    }};\n    use crate::domain::logger::mocks::MockLogger;\n    use uuid::Uuid;\n\n    fn silent_logger() -> MockLogger {{\n        let mut mock = MockLogger::new();\n        mock.expect_info().returning(|_| ());\n        mock.expect_warn().returning(|_| ());\n        mock.expect_error().returning(|_| ());\n        mock.expect_debug().returning(|_| ());\n        mock\n    }}\n\n    #[tokio::test]\n    async fn should_return_{snake}_when_id_exists() {{\n        let entity = {pascal}::new({pascal}Props {{\n{props_fields}\n        }}).unwrap();\n        let entity_id = entity.id;\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo\n            .expect_find_by_id()\n            .returning(move |_| Ok(Some(entity.clone())));\n        let use_case = Get{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case.execute(Get{pascal}Params {{ id: entity_id }}).await;\n\n        assert!(result.is_ok());{found_assertion}\n    }}\n\n    #[tokio::test]\n    async fn should_return_not_found_when_id_does_not_exist() {{\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo.expect_find_by_id().returning(|_| Ok(None));\n        let use_case = Get{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case\n            .execute(Get{pascal}Params {{ id: Uuid::new_v4() }})\n            .await;\n\n        assert!(result.is_err());\n        assert_eq!(result.unwrap_err().to_string(), \"{snake}.not_found\");\n    }}\n}}\n",
        snake=snake, pascal=pascal, props_fields=props_fields, found_assertion=found_assertion));
    s
}

pub fn generate_list_use_case_impl(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let first_props = test_props_lines(&eff, "first");
    let second_props = test_props_lines(&eff, "second");

    let mut s = String::new();
    s.push_str("use std::sync::Arc;\n\nuse async_trait::async_trait;\n");
    s.push_str(&format!("\nuse crate::domain::{snake}::{{\n    errors::{pascal}Error,\n    model::{pascal},\n    repository::{pascal}RepositoryTrait,\n    use_cases::list_{snake}::{{List{pascal}Params, List{pascal}UseCaseTrait}},\n}};\nuse crate::domain::logger::LoggerTrait;\n\npub struct List{pascal}UseCaseImpl {{\n    pub repository: Arc<dyn {pascal}RepositoryTrait>,\n    pub logger: Arc<dyn LoggerTrait>,\n}}\n\n#[async_trait]\nimpl List{pascal}UseCaseTrait for List{pascal}UseCaseImpl {{\n    async fn execute(&self, _params: List{pascal}Params) -> Result<Vec<{pascal}>, {pascal}Error> {{\n        self.logger.info(\"Listing {snake}s\");\n        self.repository.find_all().await.map_err(|e| {{\n            self.logger.error(&e.to_string());\n            e\n        }})\n    }}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n    use crate::domain::{snake}::{{\n        model::{{{pascal}, {pascal}Props}},\n        repository::mocks::Mock{pascal}Repository,\n    }};\n    use crate::domain::logger::mocks::MockLogger;\n\n    fn silent_logger() -> MockLogger {{\n        let mut mock = MockLogger::new();\n        mock.expect_info().returning(|_| ());\n        mock.expect_warn().returning(|_| ());\n        mock.expect_error().returning(|_| ());\n        mock.expect_debug().returning(|_| ());\n        mock\n    }}\n\n    #[tokio::test]\n    async fn should_return_all_{snake}s() {{\n        let entities = vec![\n            {pascal}::new({pascal}Props {{\n{first_props}\n        }}).unwrap(),\n            {pascal}::new({pascal}Props {{\n{second_props}\n        }}).unwrap(),\n        ];\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo\n            .expect_find_all()\n            .returning(move || Ok(entities.clone()));\n        let use_case = List{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case.execute(List{pascal}Params).await;\n\n        assert!(result.is_ok());\n        assert_eq!(result.unwrap().len(), 2);\n    }}\n\n    #[tokio::test]\n    async fn should_return_empty_list_when_no_{snake}s_exist() {{\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo.expect_find_all().returning(|| Ok(vec![]));\n        let use_case = List{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case.execute(List{pascal}Params).await;\n\n        assert!(result.is_ok());\n        assert!(result.unwrap().is_empty());\n    }}\n}}\n",
        snake=snake, pascal=pascal, first_props=first_props, second_props=second_props));
    s
}

pub fn generate_delete_use_case_impl(pascal: &str, snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);

    let props_fields = test_props_lines(&eff, "example");

    let mut s = String::new();
    s.push_str("use std::sync::Arc;\n\nuse async_trait::async_trait;\n");
    s.push_str(&format!("\nuse crate::domain::{snake}::{{\n    errors::{pascal}Error,\n    repository::{pascal}RepositoryTrait,\n    use_cases::delete_{snake}::{{Delete{pascal}Params, Delete{pascal}UseCaseTrait}},\n}};\nuse crate::domain::logger::LoggerTrait;\n\npub struct Delete{pascal}UseCaseImpl {{\n    pub repository: Arc<dyn {pascal}RepositoryTrait>,\n    pub logger: Arc<dyn LoggerTrait>,\n}}\n\n#[async_trait]\nimpl Delete{pascal}UseCaseTrait for Delete{pascal}UseCaseImpl {{\n    async fn execute(&self, params: Delete{pascal}Params) -> Result<(), {pascal}Error> {{\n", snake=snake, pascal=pascal));
    s.push_str(&format!(
        "        self.logger.info(&format!(\"Deleting {snake}: {{}}\", params.id));\n        let mut entity = self\n            .repository\n            .find_by_id(params.id)\n            .await\n            .map_err(|e| {{\n                self.logger.error(&e.to_string());\n                e\n            }})?\n            .ok_or_else(|| {{\n                let err = {pascal}Error::NotFound;\n                self.logger.warn(&err.to_string());\n                err\n            }})?;\n        let now = chrono::Utc::now();\n        entity.deleted = true;\n        entity.deleted_at = Some(now);\n        entity.updated_at = now;\n        self.repository.save(&entity).await.map_err(|e| {{\n            self.logger.error(&e.to_string());\n            e\n        }})?;\n        Ok(())\n    }}\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n    use crate::domain::{snake}::{{\n        model::{{{pascal}, {pascal}Props}},\n        repository::mocks::Mock{pascal}Repository,\n    }};\n    use crate::domain::logger::mocks::MockLogger;\n    use uuid::Uuid;\n\n    fn silent_logger() -> MockLogger {{\n        let mut mock = MockLogger::new();\n        mock.expect_info().returning(|_| ());\n        mock.expect_warn().returning(|_| ());\n        mock.expect_error().returning(|_| ());\n        mock.expect_debug().returning(|_| ());\n        mock\n    }}\n\n    #[tokio::test]\n    async fn should_soft_delete_{snake}_when_id_exists() {{\n        let entity = {pascal}::new({pascal}Props {{\n{props_fields}\n        }}).unwrap();\n        let entity_id = entity.id;\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo\n            .expect_find_by_id()\n            .returning(move |_| Ok(Some(entity.clone())));\n        mock_repo.expect_save().returning(|_| Ok(()));\n        let use_case = Delete{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case\n            .execute(Delete{pascal}Params {{ id: entity_id }})\n            .await;\n\n        assert!(result.is_ok());\n    }}\n\n    #[tokio::test]\n    async fn should_return_not_found_when_{snake}_does_not_exist() {{\n        let mut mock_repo = Mock{pascal}Repository::new();\n        mock_repo.expect_find_by_id().returning(|_| Ok(None));\n        let use_case = Delete{pascal}UseCaseImpl {{\n            repository: Arc::new(mock_repo),\n            logger: Arc::new(silent_logger()),\n        }};\n\n        let result = use_case\n            .execute(Delete{pascal}Params {{ id: Uuid::new_v4() }})\n            .await;\n\n        assert!(result.is_err());\n        assert_eq!(result.unwrap_err().to_string(), \"{snake}.not_found\");\n    }}\n}}\n",
        snake=snake, pascal=pascal, props_fields=props_fields));
    s
}

pub(crate) const USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{{Pascal}, {Pascal}Props},
    repository::{Pascal}RepositoryTrait,
    use_cases::create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Create{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Create{Pascal}UseCaseTrait for Create{Pascal}UseCaseImpl {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Creating {snake}: {}", params.name));
        let entity = {Pascal}::new({Pascal}Props { name: params.name }).map_err(|e| {
            self.logger.warn(&e.to_string());
            e
        })?;
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        self.logger.info(&format!("{Pascal} created: {}", entity.name));
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::repository::mocks::Mock{Pascal}Repository;
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_create_{snake}_when_name_is_valid() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Create{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Create{Pascal}Params { name: "example".into() })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[tokio::test]
    async fn should_return_error_when_name_is_empty() {
        // Arrange
        let mock_repo = Mock{Pascal}Repository::new();
        let use_case = Create{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Create{Pascal}Params { name: "".into() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }
}
"#;

const GET_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::get_{snake}::{Get{Pascal}Params, Get{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Get{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Get{Pascal}UseCaseTrait for Get{Pascal}UseCaseImpl {
    async fn execute(&self, params: Get{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Getting {snake}: {}", params.id));
        let result = self.repository.find_by_id(params.id).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        result.ok_or_else(|| {
            let err = {Pascal}Error::NotFound;
            self.logger.warn(&err.to_string());
            err
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{{Pascal}, {Pascal}Props},
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_return_{snake}_when_id_exists() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "example".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        let use_case = Get{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(Get{Pascal}Params { id: entity_id }).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[tokio::test]
    async fn should_return_not_found_when_id_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Get{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Get{Pascal}Params { id: Uuid::new_v4() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }
}
"#;

const LIST_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::list_{snake}::{List{Pascal}Params, List{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct List{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl List{Pascal}UseCaseTrait for List{Pascal}UseCaseImpl {
    async fn execute(&self, _params: List{Pascal}Params) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.info("Listing {snake}s");
        self.repository.find_all().await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{{Pascal}, {Pascal}Props},
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_return_all_{snake}s() {
        // Arrange
        let entities = vec![
            {Pascal}::new({Pascal}Props { name: "first".into() }).unwrap(),
            {Pascal}::new({Pascal}Props { name: "second".into() }).unwrap(),
        ];
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_all()
            .returning(move || Ok(entities.clone()));
        let use_case = List{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(List{Pascal}Params).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn should_return_empty_list_when_no_{snake}s_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_all().returning(|| Ok(vec![]));
        let use_case = List{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case.execute(List{Pascal}Params).await;

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
"#;

const UPDATE_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::update_{snake}::{Update{Pascal}Params, Update{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Update{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Update{Pascal}UseCaseTrait for Update{Pascal}UseCaseImpl {
    async fn execute(&self, params: Update{Pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Updating {snake}: {}", params.id));
        let mut entity = self
            .repository
            .find_by_id(params.id)
            .await
            .map_err(|e| {
                self.logger.error(&e.to_string());
                e
            })?
            .ok_or_else(|| {
                let err = {Pascal}Error::NotFound;
                self.logger.warn(&err.to_string());
                err
            })?;
        if params.name.trim().is_empty() {
            let err = {Pascal}Error::ValidationError("name_empty".into());
            self.logger.warn(&err.to_string());
            return Err(err);
        }
        entity.name = params.name;
        entity.updated_at = chrono::Utc::now();
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{{Pascal}, {Pascal}Props},
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_update_{snake}_when_params_are_valid() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "original".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: entity_id,
                name: "updated".into(),
            })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "updated");
    }

    #[tokio::test]
    async fn should_return_not_found_when_{snake}_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: Uuid::new_v4(),
                name: "new".into(),
            })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }

    #[tokio::test]
    async fn should_return_error_when_name_is_empty() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "original".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        let use_case = Update{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Update{Pascal}Params {
                id: entity_id,
                name: "".into(),
            })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }
}
"#;

const DELETE_USE_CASE_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    repository::{Pascal}RepositoryTrait,
    use_cases::delete_{snake}::{Delete{Pascal}Params, Delete{Pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct Delete{Pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl Delete{Pascal}UseCaseTrait for Delete{Pascal}UseCaseImpl {
    async fn execute(&self, params: Delete{Pascal}Params) -> Result<(), {Pascal}Error> {
        self.logger.info(&format!("Deleting {snake}: {}", params.id));
        let mut entity = self
            .repository
            .find_by_id(params.id)
            .await
            .map_err(|e| {
                self.logger.error(&e.to_string());
                e
            })?
            .ok_or_else(|| {
                let err = {Pascal}Error::NotFound;
                self.logger.warn(&err.to_string());
                err
            })?;
        let now = chrono::Utc::now();
        entity.deleted = true;
        entity.deleted_at = Some(now);
        entity.updated_at = now;
        self.repository.save(&entity).await.map_err(|e| {
            self.logger.error(&e.to_string());
            e
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{snake}::{
        model::{{Pascal}, {Pascal}Props},
        repository::mocks::Mock{Pascal}Repository,
    };
    use crate::domain::logger::mocks::MockLogger;
    use uuid::Uuid;

    fn silent_logger() -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().returning(|_| ());
        mock.expect_warn().returning(|_| ());
        mock.expect_error().returning(|_| ());
        mock.expect_debug().returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_soft_delete_{snake}_when_id_exists() {
        // Arrange
        let entity = {Pascal}::new({Pascal}Props { name: "example".into() }).unwrap();
        let entity_id = entity.id;
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(entity.clone())));
        mock_repo.expect_save().returning(|_| Ok(()));
        let use_case = Delete{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Delete{Pascal}Params { id: entity_id })
            .await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_return_not_found_when_{snake}_does_not_exist() {
        // Arrange
        let mut mock_repo = Mock{Pascal}Repository::new();
        mock_repo.expect_find_by_id().returning(|_| Ok(None));
        let use_case = Delete{Pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        let result = use_case
            .execute(Delete{Pascal}Params { id: Uuid::new_v4() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "{snake}.not_found");
    }
}
"#;

pub fn write_application_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    fields: &[Field],
) -> Result<(), Box<dyn std::error::Error>> {
    let create_impl = if fields.is_empty() {
        apply(USE_CASE_IMPL, pascal, snake)
    } else {
        generate_create_use_case_impl(pascal, snake, fields)
    };
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/create_{snake}.rs"
        )),
        &create_impl,
    )?;
    let get_impl = if fields.is_empty() {
        apply(GET_USE_CASE_IMPL, pascal, snake)
    } else {
        generate_get_use_case_impl(pascal, snake, fields)
    };
    write_file(
        &base.join(format!("business/src/application/{snake}/get_{snake}.rs")),
        &get_impl,
    )?;
    let list_impl = if fields.is_empty() {
        apply(LIST_USE_CASE_IMPL, pascal, snake)
    } else {
        generate_list_use_case_impl(pascal, snake, fields)
    };
    write_file(
        &base.join(format!("business/src/application/{snake}/list_{snake}.rs")),
        &list_impl,
    )?;
    let update_impl = if fields.is_empty() {
        apply(UPDATE_USE_CASE_IMPL, pascal, snake)
    } else {
        generate_update_use_case_impl(pascal, snake, fields)
    };
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/update_{snake}.rs"
        )),
        &update_impl,
    )?;
    let delete_impl = if fields.is_empty() {
        apply(DELETE_USE_CASE_IMPL, pascal, snake)
    } else {
        generate_delete_use_case_impl(pascal, snake, fields)
    };
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/delete_{snake}.rs"
        )),
        &delete_impl,
    )?;
    Ok(())
}

pub fn run_generate_application(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate domain {pascal}` first."
        )
        .into());
    }

    let fields: Vec<Field> = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_application_files(&pascal, &snake, base, &fields)?;
    patch_business_lib_application_crud(base, &snake)?;

    println!("✓ business/application/ — 5 use case impls (create, get, list, update, delete)");
    println!();
    println!("  Next: puerto generate repository {pascal}");
    Ok(())
}
