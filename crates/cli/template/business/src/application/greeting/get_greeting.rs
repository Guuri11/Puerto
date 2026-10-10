use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::greeting::{
    errors::GreetingError,
    model::Greeting,
    repository::GreetingRepositoryTrait,
    use_cases::get_greeting::{GetGreetingParams, GetGreetingUseCaseTrait},
};

pub struct GetGreetingUseCaseImpl {
    pub repository: Arc<dyn GreetingRepositoryTrait>,
}

#[async_trait]
impl GetGreetingUseCaseTrait for GetGreetingUseCaseImpl {
    async fn execute(&self, params: GetGreetingParams) -> Result<Greeting, GreetingError> {
        tracing::info!(name = %params.name, "get_greeting: start");

        if params.name.trim().is_empty() {
            let err = GreetingError::ValidationError("name_empty".into());
            tracing::warn!(error = %err, "get_greeting: rejected");
            return Err(err);
        }

        let result = match self
            .repository
            .find_by_name(&params.name)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "get_greeting: repository failed"))?
        {
            Some(greeting) => greeting,
            None => Greeting::new(&params.name)?,
        };

        tracing::info!("get_greeting: done");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::greeting::repository::mocks::MockGreetingRepository;

    #[tokio::test]
    async fn should_return_greeting_for_valid_name() {
        // Arrange
        let mut mock_repo = MockGreetingRepository::new();
        mock_repo.expect_find_by_name().returning(|_| Ok(None));

        let use_case = GetGreetingUseCaseImpl {
            repository: Arc::new(mock_repo),
        };

        // Act
        let result = use_case
            .execute(GetGreetingParams {
                name: "World".into(),
            })
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
            name: "Puerto".into(),
            message: "Hello from cache!".into(),
        };
        let mut mock_repo = MockGreetingRepository::new();
        mock_repo
            .expect_find_by_name()
            .returning(move |_| Ok(Some(cached.clone())));

        let use_case = GetGreetingUseCaseImpl {
            repository: Arc::new(mock_repo),
        };

        // Act
        let result = use_case
            .execute(GetGreetingParams {
                name: "Puerto".into(),
            })
            .await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().message, "Hello from cache!");
    }
}
