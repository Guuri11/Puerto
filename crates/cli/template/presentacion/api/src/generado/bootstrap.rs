// GENERADO por puerto. Edita puerto.toml y después ejecuta `puerto generar bootstrap`.
use poem::Endpoint;
use poem_openapi::OpenApiService;
use rest_saludo::rutas::ApiSaludo;

/// Las rutas, cableadas. Un error es lo que paró el arranque: `main` lo muestra y sale con 1.
pub async fn construir_app(config: puerto_web::Config) -> Result<impl Endpoint, String> {
    let casos_saludo = cableado::saludo::construir();
    let api_saludo = ApiSaludo { obtener_saludo: casos_saludo.obtener_saludo };

    let disponible = puerto_web::health::siempre_disponible();

    // `app.rs` es del proyecto: el middleware y las rutas que se añadan allí envuelven todo esto.
    Ok(crate::app::configurar(puerto_web::rutas(
        servicio_openapi(api_saludo),
        &config,
        disponible,
    )))
}

/// El servicio de la API, construido en un solo sitio: cualquier otro que lea su spec (el test de
/// exportación de un proyecto con `web/`) ve la API que responde el servidor.
fn servicio_openapi<T: poem_openapi::OpenApi>(api: T) -> OpenApiService<T, ()> {
    OpenApiService::new(api, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use aplicacion_saludo::obtener_saludo::dobles::MockObtenerSaludo;

    /// El contrato de la API del que se generan los tipos del frontend (`make web/sync`).
    const OPENAPI_JSON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/api/openapi.json");

    /// Se construye con casos de uso dobles: sin servidor ni base de datos. Si falta el fichero,
    /// se escribe; si es distinto, falla: ejecuta `make web/sync` (pone `UPDATE_OPENAPI=1`) y haz
    /// commit del diff.
    #[test]
    fn la_spec_openapi_coincide_con_docs_api_openapi_json() {
        let api_saludo = ApiSaludo { obtener_saludo: Arc::new(MockObtenerSaludo::new()) };
        let spec = servicio_openapi(api_saludo).spec();

        let path = std::path::Path::new(OPENAPI_JSON);
        if std::env::var_os("UPDATE_OPENAPI").is_some() || !path.exists() {
            let dir = path.parent().expect("docs/api/openapi.json tiene directorio padre");
            std::fs::create_dir_all(dir).expect("crear docs/api");
            std::fs::write(path, &spec).expect("escribir docs/api/openapi.json");
            return;
        }
        let en_disco = std::fs::read_to_string(path).expect("leer docs/api/openapi.json");
        assert!(
            en_disco == spec,
            "docs/api/openapi.json no es la API que sirve este código: ejecuta `make web/sync`"
        );
    }
}
