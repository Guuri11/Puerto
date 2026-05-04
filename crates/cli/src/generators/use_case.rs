use std::path::Path;

use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::naming::{apply_uc, pascal_to_snake, to_pascal_case, write_file};
use crate::patchers::lib_rs::patch_business_lib_use_case;

const UC_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct {uc_pascal}Params {
    pub name: String,
}

#[async_trait]
pub trait {uc_pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const UC_IMPL: &str = r#"use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
    use_cases::{uc}::{{uc_pascal}Params, {uc_pascal}UseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct {uc_pascal}UseCaseImpl {
    pub repository: Arc<dyn {Pascal}RepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl {uc_pascal}UseCaseTrait for {uc_pascal}UseCaseImpl {
    async fn execute(&self, params: {uc_pascal}Params) -> Result<{Pascal}, {Pascal}Error> {
        self.logger.info(&format!("Executing {uc}: {:?}", params));
        todo!()
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
    async fn should_{uc}_when_valid() {
        // Arrange
        let mock_repo = Mock{Pascal}Repository::new();
        let use_case = {uc_pascal}UseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(silent_logger()),
        };

        // Act
        // TODO: implement test body
        let _ = &use_case;
    }
}
"#;

pub fn run_use_case(
    entity: &str,
    action: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(entity);
    let snake = pascal_to_snake(&pascal);
    let uc = action.to_string();
    let uc_pascal = to_pascal_case(&uc);

    // Errors if entity not in puerto.toml
    crate::puerto_toml::add_use_case(base, &pascal, &uc)?;

    write_file(
        &base.join(format!("business/src/domain/{snake}/use_cases/{uc}.rs")),
        &apply_uc(UC_TRAIT, &pascal, &snake, &uc_pascal, &uc),
    )?;
    write_file(
        &base.join(format!("business/src/application/{snake}/{uc}.rs")),
        &apply_uc(UC_IMPL, &pascal, &snake, &uc_pascal, &uc),
    )?;

    patch_business_lib_use_case(base, &snake, &uc)?;
    regenerate_bootstrap(base)?;

    println!("✓ Use case {uc_pascal} added to {pascal} (2 files).");
    println!("  puerto.toml updated + bootstrap.rs regenerated.");

    Ok(())
}
