use thiserror::Error;

/// What a repository port fails with, whatever the entity and the adapter. Each entity's error
/// converts it (`From<ErrorRepositorio>`), so a use case propagates it with `?`. Its codes reach the
/// logs only: the API answers with the entity's own (`product.duplicate`).
///
/// `#[non_exhaustive]`: a later version may add a failure without breaking the projects that
/// match on it — their `_` arm treats it as the store's problem until they give it a meaning.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ErrorRepositorio {
    /// A unique constraint rejected the write.
    #[error("repositorio.duplicado")]
    Duplicado,
    /// A stored row no longer passes the domain's rules.
    #[error("repositorio.datos_invalidos")]
    DatosInvalidos,
    /// The store failed or could not be reached.
    #[error("repositorio.no_disponible")]
    NoDisponible,
    /// A reference names a row that does not exist (a foreign key rejected the write). Holds the
    /// field: the entity's error turns it into a validation error on it (ADR-0022).
    #[error("repositorio.referencia_inexistente.{0}")]
    ReferenciaInexistente(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_speak_machine_readable_codes() {
        assert_eq!(
            ErrorRepositorio::Duplicado.to_string(),
            "repositorio.duplicado"
        );
        assert_eq!(
            ErrorRepositorio::DatosInvalidos.to_string(),
            "repositorio.datos_invalidos"
        );
        assert_eq!(
            ErrorRepositorio::NoDisponible.to_string(),
            "repositorio.no_disponible"
        );
    }

    #[test]
    fn should_name_the_field_whose_reference_is_missing() {
        assert_eq!(
            ErrorRepositorio::ReferenciaInexistente("customer_id").to_string(),
            "repositorio.referencia_inexistente.customer_id"
        );
    }
}
