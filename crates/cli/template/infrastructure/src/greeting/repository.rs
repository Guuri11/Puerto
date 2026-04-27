use std::sync::Arc;

use async_trait::async_trait;
use business::domain::{
    greeting::{errors::GreetingError, model::Greeting, repository::GreetingRepositoryTrait},
    logger::LoggerTrait,
};

pub struct InMemoryGreetingRepository {
    logger: Arc<dyn LoggerTrait>,
}

impl InMemoryGreetingRepository {
    pub fn new(logger: Arc<dyn LoggerTrait>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl GreetingRepositoryTrait for InMemoryGreetingRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Greeting>, GreetingError> {
        self.logger.debug(&format!("find_by_name: {name}"));
        Ok(None)
    }
}
