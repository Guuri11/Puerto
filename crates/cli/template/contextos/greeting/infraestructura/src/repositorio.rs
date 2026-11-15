use async_trait::async_trait;
use dominio_greeting::{modelo::Greeting, repositorio::GreetingRepositoryTrait};
use puerto_nucleo::ErrorRepositorio;

#[derive(Default)]
pub struct InMemoryGreetingRepository;

impl InMemoryGreetingRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GreetingRepositoryTrait for InMemoryGreetingRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, ErrorRepositorio> {
        tracing::debug!(name, "find_by_name");
        Ok(None)
    }
}
