use async_trait::async_trait;

use crate::domain::greeting::{errors::GreetingError, model::Greeting};

#[derive(Debug, Clone)]
pub struct GetGreetingParams {
    pub name: String,
}

#[async_trait]
pub trait GetGreetingUseCaseTrait: Send + Sync {
    async fn execute(&self, params: GetGreetingParams) -> Result<Greeting, GreetingError>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
        pub GetGreetingUseCase {}

        #[async_trait]
        impl GetGreetingUseCaseTrait for GetGreetingUseCase {
            async fn execute(&self, params: GetGreetingParams) -> Result<Greeting, GreetingError>;
        }
    }
}
