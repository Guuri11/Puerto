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
