use std::path::Path;

use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::patchers::lib_rs::patch_business_lib_application_crud;

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
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/create_{snake}.rs"
        )),
        &apply(USE_CASE_IMPL, pascal, snake),
    )?;
    write_file(
        &base.join(format!("business/src/application/{snake}/get_{snake}.rs")),
        &apply(GET_USE_CASE_IMPL, pascal, snake),
    )?;
    write_file(
        &base.join(format!("business/src/application/{snake}/list_{snake}.rs")),
        &apply(LIST_USE_CASE_IMPL, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/update_{snake}.rs"
        )),
        &apply(UPDATE_USE_CASE_IMPL, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/application/{snake}/delete_{snake}.rs"
        )),
        &apply(DELETE_USE_CASE_IMPL, pascal, snake),
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

    write_application_files(&pascal, &snake, base)?;
    patch_business_lib_application_crud(base, &snake)?;

    println!("✓ business/application/ — 5 use case impls (create, get, list, update, delete)");
    println!();
    println!("  Next: puerto generate repository {pascal}");
    Ok(())
}
