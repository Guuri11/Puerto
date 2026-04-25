use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::greeting::{
    errors::GreetingError,
    model::Greeting,
    repository::GreetingRepositoryTrait,
    use_cases::get_greeting::{GetGreetingParams, GetGreetingUseCaseTrait},
};
use crate::domain::logger::LoggerTrait;

pub struct GetGreetingUseCaseImpl {
    pub repository: Arc<dyn GreetingRepositoryTrait>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[async_trait]
impl GetGreetingUseCaseTrait for GetGreetingUseCaseImpl {
    async fn execute(&self, params: GetGreetingParams) -> Result<Greeting, GreetingError> {
        self.logger.info(&format!("Getting greeting for: {}", params.name));

        if params.name.trim().is_empty() {
            return Err(GreetingError::ValidationError("name_empty".into()));
        }

        let result = match self.repository.find_by_name(&params.name).await? {
            Some(greeting) => greeting,
            None => Greeting::new(&params.name)?,
        };

        self.logger.info(&format!("Greeting created: {}", result.message));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::greeting::repository::mocks::MockGreetingRepository;
    use crate::domain::logger::mocks::MockLogger;

    fn logger_expecting_info(times: usize) -> MockLogger {
        let mut mock = MockLogger::new();
        mock.expect_info().times(times).returning(|_| ());
        mock
    }

    #[tokio::test]
    async fn should_return_greeting_for_valid_name() {
        // Arrange
        let mut mock_repo = MockGreetingRepository::new();
        mock_repo
            .expect_find_by_name()
            .returning(|_| Ok(None));

        let use_case = GetGreetingUseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(logger_expecting_info(2)),
        };

        // Act
        let result = use_case
            .execute(GetGreetingParams { name: "World".into() })
            .await;

        // Assert
        assert!(result.is_ok());
        let greeting = result.unwrap();
        assert_eq!(greeting.name, "World");
        assert!(greeting.message.contains("World"));
    }

    #[tokio::test]
    async fn should_return_error_when_name_is_empty() {
        // Arrange
        let mock_repo = MockGreetingRepository::new();
        let use_case = GetGreetingUseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(logger_expecting_info(1)),
        };

        // Act
        let result = use_case
            .execute(GetGreetingParams { name: "".into() })
            .await;

        // Assert
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "greeting.validation_error.name_empty"
        );
    }

    #[tokio::test]
    async fn should_return_cached_greeting_when_found_in_repository() {
        // Arrange
        let cached = Greeting {
            name: "Harbor".into(),
            message: "Hello from cache!".into(),
        };
        let mut mock_repo = MockGreetingRepository::new();
        mock_repo
            .expect_find_by_name()
            .returning(move |_| Ok(Some(cached.clone())));

        let use_case = GetGreetingUseCaseImpl {
            repository: Arc::new(mock_repo),
            logger: Arc::new(logger_expecting_info(2)),
        };

        // Act
        let result = use_case
            .execute(GetGreetingParams { name: "Harbor".into() })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message, "Hello from cache!");
    }
}
