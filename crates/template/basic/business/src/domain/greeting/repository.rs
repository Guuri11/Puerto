use async_trait::async_trait;

use super::{errors::GreetingError, model::Greeting};

/// Port: contract for greeting persistence (implemented by infrastructure)
#[async_trait]
pub trait GreetingRepositoryTrait: Send + Sync {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, GreetingError>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
        pub GreetingRepository {}

        #[async_trait]
        impl GreetingRepositoryTrait for GreetingRepository {
            async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, GreetingError>;
        }
    }
}
