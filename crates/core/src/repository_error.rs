use thiserror::Error;

/// What a repository port fails with, whatever the entity and the adapter. Each entity's error
/// converts it (`From<RepositoryError>`), so a use case propagates it with `?`. Its codes reach the
/// logs only: the API answers with the entity's own (`product.duplicate`).
///
/// `#[non_exhaustive]`: a later version may add a failure without breaking the projects that
/// match on it — their `_` arm treats it as the store's problem until they give it a meaning.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RepositoryError {
    /// A unique constraint rejected the write.
    #[error("repository.duplicate")]
    Duplicate,
    /// A stored row no longer passes the domain's rules.
    #[error("repository.invalid_data")]
    InvalidData,
    /// The store failed or could not be reached.
    #[error("repository.unavailable")]
    Unavailable,
    /// A reference names a row that does not exist (a foreign key rejected the write). Holds the
    /// field: the entity's error turns it into a validation error on it (ADR-0022).
    #[error("repository.missing_reference.{0}")]
    MissingReference(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_speak_machine_readable_codes() {
        assert_eq!(
            RepositoryError::Duplicate.to_string(),
            "repository.duplicate"
        );
        assert_eq!(
            RepositoryError::InvalidData.to_string(),
            "repository.invalid_data"
        );
        assert_eq!(
            RepositoryError::Unavailable.to_string(),
            "repository.unavailable"
        );
    }

    #[test]
    fn should_name_the_field_whose_reference_is_missing() {
        assert_eq!(
            RepositoryError::MissingReference("customer_id").to_string(),
            "repository.missing_reference.customer_id"
        );
    }
}
