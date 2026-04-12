use thiserror::Error;

#[derive(Debug, Error)]
pub enum GreetingError {
    #[error("greeting.validation_error.{0}")]
    ValidationError(String),
    #[error("greeting.not_found")]
    NotFound,
    #[error("greeting.repository_error")]
    RepositoryError,
}
