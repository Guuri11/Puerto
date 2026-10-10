use async_trait::async_trait;
use business::domain::greeting::{
    errors::GreetingError, model::Greeting, repository::GreetingRepositoryTrait,
};

#[derive(Default)]
pub struct InMemoryGreetingRepository;

impl InMemoryGreetingRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GreetingRepositoryTrait for InMemoryGreetingRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, GreetingError> {
        tracing::debug!(name, "find_by_name");
        Ok(None)
    }
}
