use thiserror::Error;

/// What a repository port fails with, whatever the entity and the adapter. Each entity's error
/// converts it (`From<RepositoryError>`), so a use case propagates it with `?`. Its codes reach the
/// logs only: the API answers with the entity's own (`product.duplicate`).
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
}
