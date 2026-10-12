use puerto_core::RepositoryError;
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

impl From<RepositoryError> for GreetingError {
    /// Greetings are never written, so every repository failure is the store's.
    fn from(_: RepositoryError) -> Self {
        Self::RepositoryError
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_an_unavailable_store_into_a_repository_error() {
        let err = GreetingError::from(RepositoryError::Unavailable);
        assert_eq!(err.to_string(), "greeting.repository_error");
    }
}
