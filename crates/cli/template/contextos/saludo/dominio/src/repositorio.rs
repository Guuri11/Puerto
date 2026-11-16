use async_trait::async_trait;
use puerto_nucleo::ErrorRepositorio;

use crate::modelo::Saludo;

/// Puerto: contrato de persistencia de los saludos (lo implementa la infraestructura)
#[async_trait]
pub trait RepositorioSaludo: Send + Sync {
    async fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Saludo>, ErrorRepositorio>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use mockall::mock;

    use super::*;

    mock! {
        pub RepositorioSaludo {}

        #[async_trait]
        impl RepositorioSaludo for RepositorioSaludo {
            async fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Saludo>, ErrorRepositorio>;
        }
    }
}
