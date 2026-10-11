use async_trait::async_trait;
use business::domain::greeting::{model::Greeting, repository::GreetingRepositoryTrait};
use business::domain::shared::repository_error::RepositoryError;

#[derive(Default)]
pub struct InMemoryGreetingRepository;

impl InMemoryGreetingRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GreetingRepositoryTrait for InMemoryGreetingRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, RepositoryError> {
        tracing::debug!(name, "find_by_name");
        Ok(None)
    }
}
