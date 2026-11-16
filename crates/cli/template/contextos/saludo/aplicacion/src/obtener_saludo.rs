use std::sync::Arc;

use async_trait::async_trait;
use dominio_saludo::{
    errores::ErrorSaludo, modelo::Saludo, repositorio::RepositorioSaludo,
};

#[derive(Debug, Clone)]
pub struct ParamsObtenerSaludo {
    pub nombre: String,
}

#[async_trait]
pub trait ObtenerSaludo: Send + Sync {
    async fn ejecutar(&self, params: ParamsObtenerSaludo) -> Result<Saludo, ErrorSaludo>;
}

#[cfg(any(test, feature = "pruebas"))]
pub mod dobles {
    use mockall::mock;

    use super::*;

    mock! {
        pub ObtenerSaludo {}

        #[async_trait]
        impl ObtenerSaludo for ObtenerSaludo {
            async fn ejecutar(&self, params: ParamsObtenerSaludo) -> Result<Saludo, ErrorSaludo>;
        }
    }
}

pub struct ObtenerSaludoImpl {
    pub repositorio: Arc<dyn RepositorioSaludo>,
}

#[async_trait]
impl ObtenerSaludo for ObtenerSaludoImpl {
    async fn ejecutar(&self, params: ParamsObtenerSaludo) -> Result<Saludo, ErrorSaludo> {
        tracing::info!(nombre = %params.nombre, "obtener_saludo: inicio");

        if params.nombre.trim().is_empty() {
            let err = ErrorSaludo::Validacion("nombre_vacio".into());
            tracing::warn!(error = %err, "obtener_saludo: rechazado");
            return Err(err);
        }

        let resultado = match self
            .repositorio
            .buscar_por_nombre(&params.nombre)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "obtener_saludo: fallo del repositorio"))?
        {
            Some(saludo) => saludo,
            None => Saludo::new(&params.nombre)?,
        };

        tracing::info!("obtener_saludo: fin");
        Ok(resultado)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dominio_saludo::repositorio::dobles::MockRepositorioSaludo;

    #[tokio::test]
    async fn deberia_devolver_el_saludo_cuando_el_nombre_es_valido() {
        // Preparar
        let mut repositorio_doble = MockRepositorioSaludo::new();
        repositorio_doble.expect_buscar_por_nombre().returning(|_| Ok(None));

        let caso_de_uso = ObtenerSaludoImpl {
            repositorio: Arc::new(repositorio_doble),
        };

        // Actuar
        let resultado = caso_de_uso
            .ejecutar(ParamsObtenerSaludo {
                nombre: "Mundo".into(),
            })
            .await;

        // Comprobar
        assert!(resultado.is_ok());
        let saludo = resultado.unwrap();
        assert_eq!(saludo.nombre, "Mundo");
        assert!(saludo.mensaje.contains("Mundo"));
    }

    #[tokio::test]
    async fn deberia_devolver_error_cuando_el_nombre_esta_vacio() {
        // Preparar
        let repositorio_doble = MockRepositorioSaludo::new();
        let caso_de_uso = ObtenerSaludoImpl {
            repositorio: Arc::new(repositorio_doble),
        };

        // Actuar
        let resultado = caso_de_uso
            .ejecutar(ParamsObtenerSaludo { nombre: "".into() })
            .await;

        // Comprobar
        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err().to_string(),
            "saludo.validacion.nombre_vacio"
        );
    }

    #[tokio::test]
    async fn deberia_devolver_el_saludo_guardado_cuando_esta_en_el_repositorio() {
        // Preparar
        let en_cache = Saludo {
            nombre: "Puerto".into(),
            mensaje: "¡Hola desde la caché!".into(),
        };
        let mut repositorio_doble = MockRepositorioSaludo::new();
        repositorio_doble
            .expect_buscar_por_nombre()
            .returning(move |_| Ok(Some(en_cache.clone())));

        let caso_de_uso = ObtenerSaludoImpl {
            repositorio: Arc::new(repositorio_doble),
        };

        // Actuar
        let resultado = caso_de_uso
            .ejecutar(ParamsObtenerSaludo {
                nombre: "Puerto".into(),
            })
            .await;

        // Comprobar
        assert!(resultado.is_ok());
        assert_eq!(resultado.unwrap().mensaje, "¡Hola desde la caché!");
    }
}
