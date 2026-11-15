use async_trait::async_trait;
use puerto_nucleo::ErrorRepositorio;

use crate::modelo::Greeting;

/// Port: contract for greeting persistence (implemented by infrastructure)
#[async_trait]
pub trait GreetingRepositoryTrait: Send + Sync {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, ErrorRepositorio>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod mocks {
    use mockall::mock;

    use super::*;

    mock! {
        pub GreetingRepository {}

        #[async_trait]
        impl GreetingRepositoryTrait for GreetingRepository {
            async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, ErrorRepositorio>;
        }
    }
}
