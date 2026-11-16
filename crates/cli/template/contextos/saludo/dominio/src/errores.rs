use puerto_nucleo::ErrorRepositorio;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorSaludo {
    #[error("saludo.validacion.{0}")]
    Validacion(String),
    #[error("saludo.no_encontrado")]
    NoEncontrado,
    #[error("saludo.repositorio")]
    Repositorio,
}

impl From<ErrorRepositorio> for ErrorSaludo {
    /// Los saludos nunca se escriben, así que cualquier fallo del repositorio es del almacén.
    fn from(_: ErrorRepositorio) -> Self {
        Self::Repositorio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deberia_convertir_un_almacen_no_disponible_en_un_error_de_repositorio() {
        let err = ErrorSaludo::from(ErrorRepositorio::NoDisponible);
        assert_eq!(err.to_string(), "saludo.repositorio");
    }
}
