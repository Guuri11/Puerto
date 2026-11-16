/// La configuración (el entorno y `.env`) y el servidor —tracing, sondas de salud, CORS, apagado
/// ordenado— vienen del runtime de Puerto, `puerto-web`. Las rutas y el cableado están en
/// `generado/bootstrap.rs`; tu middleware y tus rutas van en `app.rs`.
#[tokio::main]
async fn main() -> Result<(), puerto_web::Error> {
    let config = puerto_web::Config::from_env()?;
    puerto_web::App::new(config)
        .intentar_servir(api::generado::bootstrap::construir_app)
        .await
}
