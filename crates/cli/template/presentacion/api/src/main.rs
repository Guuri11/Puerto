/// The configuration (environment and `.env`) and the server — tracing, health probes, CORS,
/// graceful shutdown — come from the Puerto runtime, `puerto-web`. The routes and the wiring are
/// in `generado/bootstrap.rs`; your own middleware and routes go in `app.rs`.
#[tokio::main]
async fn main() -> Result<(), puerto_web::Error> {
    let config = puerto_web::Config::from_env()?;
    puerto_web::App::new(config)
        .intentar_servir(api::generado::bootstrap::build_app)
        .await
}
