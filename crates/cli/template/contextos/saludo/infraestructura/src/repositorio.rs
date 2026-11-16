use async_trait::async_trait;
use dominio_saludo::{modelo::Saludo, repositorio::RepositorioSaludo};
use puerto_nucleo::ErrorRepositorio;

#[derive(Default)]
pub struct RepositorioSaludoEnMemoria;

impl RepositorioSaludoEnMemoria {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RepositorioSaludo for RepositorioSaludoEnMemoria {
    async fn buscar_por_nombre(&self, nombre: &str) -> Result<Option<Saludo>, ErrorRepositorio> {
        tracing::debug!(nombre = nombre, "buscar_por_nombre");
        Ok(None)
    }
}
