use async_trait::async_trait;
use business::domain::greeting::{errors::GreetingError, model::Greeting, repository::GreetingRepositoryTrait};

/// In-memory greeting repository (replace with a real DB adapter when needed)
pub struct InMemoryGreetingRepository;

#[async_trait]
impl GreetingRepositoryTrait for InMemoryGreetingRepository {
    async fn find_by_name(&self, _name: &str) -> Result<Option<Greeting>, GreetingError> {
        // No cache — always let the domain generate the greeting
        Ok(None)
    }
}
