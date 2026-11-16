// GENERADO por puerto. Edita puerto.toml y después ejecuta `puerto generar bootstrap`.
//! Los repositorios, unidades de trabajo y casos de uso de cada contexto, construidos para los
//! binarios (ADR-0023 § 7 de Puerto). Aquí no hay HTTP: `api` monta sus rutas sobre estos casos de
//! uso.

pub mod saludo {
    use std::sync::Arc;

    use aplicacion_saludo::obtener_saludo::{ObtenerSaludoImpl, ObtenerSaludo};
    use dominio_saludo::repositorio::RepositorioSaludo;
    use infra_saludo::repositorio::RepositorioSaludoEnMemoria;

    /// Los casos de uso de `saludo`, sobre sus adaptadores.
    pub struct CasosDeUso {
        pub obtener_saludo: Arc<dyn ObtenerSaludo>,
    }

    pub fn construir() -> CasosDeUso {
        let repositorio: Arc<dyn RepositorioSaludo> = Arc::new(RepositorioSaludoEnMemoria::new());
        CasosDeUso {
            obtener_saludo: Arc::new(ObtenerSaludoImpl { repositorio: Arc::clone(&repositorio) }),
        }
    }
}
