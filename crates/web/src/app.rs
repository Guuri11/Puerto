//! The server: what `main.rs` starts and `bootstrap.rs` builds the routes for.

use std::convert::Infallible;
use std::fmt::Display;
use std::time::Duration;

use poem::{
    Endpoint, EndpointExt, IntoEndpoint, Route, Server, listener::TcpListener, middleware::Tracing,
};
use poem_openapi::{OpenApi, OpenApiService};

use crate::{Config, Error, health, health::SondaDisponibilidad};

/// How long in-flight requests get to finish once a shutdown signal arrives. Keep it below the
/// orchestrator's kill timeout (`docker stop` waits 10s by default) so requests drain instead of
/// being cut.
pub const PLAZO_APAGADO: Duration = Duration::from_secs(8);

const BANNER: &str = r#"
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    ***********   ++++++++++++   =============
    %#*******##%%%#**********#%%%%++++++++++*%
    %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
   %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
   %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
    %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
    %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
     %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
     %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
      %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
      %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
      ****###########%%%%%%%%%%%%%%%%%%%%%%%
  *****************************        ****
  ***********     ***********************
   *                     ***************
                                *****

    P  U  E  R  T  O"#;

/// The running application: tracing, the banner, the listener, CORS and graceful shutdown.
pub struct App {
    config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Sets up tracing (`RUST_LOG`), prints the banner, builds the routes with `build` and serves
    /// them on `SERVICE_PORT` until Ctrl-C or SIGTERM, then drains in-flight requests for
    /// [`PLAZO_APAGADO`]. For a `build` that cannot fail; see [`App::intentar_servir`].
    pub async fn servir<F, Fut, E>(self, build: F) -> Result<(), Error>
    where
        F: FnOnce(Config) -> Fut,
        Fut: Future<Output = E>,
        E: IntoEndpoint + 'static,
        E::Endpoint: 'static,
    {
        self.intentar_servir(|config| async move { Ok::<_, Infallible>(build(config).await) })
            .await
    }

    /// [`App::servir`] with a `build` that can fail — the project's
    /// `api::generado::bootstrap::construir_app`, which connects to the database. Its error is logged and
    /// returned as [`Error::Arranque`]: `main` prints it and exits 1, nothing is bound.
    pub async fn intentar_servir<F, Fut, E, X>(self, build: F) -> Result<(), Error>
    where
        F: FnOnce(Config) -> Fut,
        Fut: Future<Output = Result<E, X>>,
        X: Display,
        E: IntoEndpoint + 'static,
        E::Endpoint: 'static,
    {
        iniciar_tracing();

        if self.config.banner() {
            println!("{BANNER}");
            println!("    puerto v{}\n", env!("CARGO_PKG_VERSION"));
        }

        let puerto = self.config.puerto();
        let built = build(self.config.clone()).await.map_err(|e| {
            let mensaje = e.to_string();
            tracing::error!(error = %mensaje, "el arranque falló");
            Error::Arranque(mensaje)
        })?;
        let endpoint = wrap(&self.config, built);
        Server::new(TcpListener::bind(format!("0.0.0.0:{puerto}")))
            .run_with_graceful_shutdown(endpoint, senal_apagado(), Some(PLAZO_APAGADO))
            .await?;
        Ok(())
    }
}

/// Logs to stdout, filtered by `RUST_LOG`. A subscriber the project installed itself wins.
pub fn iniciar_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

/// What every request goes through, outermost last: request tracing, then CORS.
fn wrap<E: IntoEndpoint>(config: &Config, endpoint: E) -> impl Endpoint + use<E>
where
    E::Endpoint: 'static,
{
    endpoint.into_endpoint().with(Tracing).with(config.cors())
}

/// The routes every project serves: its API under `/api`, the Swagger UI at `/`, and the health
/// probes at `/health` and `/health/ready`. `api` is built by the project, which knows its own name
/// and version.
pub fn rutas<T: OpenApi + 'static>(
    api: OpenApiService<T, ()>,
    config: &Config,
    ready: SondaDisponibilidad,
) -> Route {
    let api = api.server(format!("http://localhost:{}/api", config.puerto()));
    let ui = api.swagger_ui();
    health::montar(Route::new(), ready)
        .nest("/api", api)
        .nest("/", ui)
}

/// Resolves on Ctrl-C or SIGTERM — what `docker stop` and Kubernetes send. The listener then stops
/// accepting and in-flight requests get `PLAZO_APAGADO` to finish. The worker
/// (`puerto-events`) drains its loops on the same signal.
pub async fn senal_apagado() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("señal de apagado recibida, vaciando");
}

#[cfg(test)]
mod tests {
    use poem::{handler, http::StatusCode, test::TestClient};

    use super::*;
    use crate::health::{siempre_disponible, sondear};

    #[handler]
    fn hello() -> &'static str {
        "hello"
    }

    fn allowing(origin: &str) -> Config {
        Config::default().con_origenes_cors_permitidos(vec![origin.to_string()])
    }

    #[tokio::test]
    async fn should_stop_with_the_build_error_when_the_app_cannot_be_built() {
        // Arrange
        let app = App::new(Config::default());

        // Act
        let result = app
            .intentar_servir(|_| async { Err::<Route, _>("the database is down") })
            .await;

        // Assert: `main` prints the error with `Debug` and exits 1.
        let error = result.unwrap_err();
        assert!(matches!(error, Error::Arranque(_)), "{error:?}");
        assert_eq!(format!("{error:?}"), "the database is down");
    }

    #[tokio::test]
    async fn should_allow_a_preflight_when_the_origin_is_configured() {
        // Arrange
        let client = TestClient::new(wrap(
            &allowing("http://localhost:5173"),
            Route::new().at("/api/x", poem::get(hello)),
        ));

        // Act
        let response = client
            .options("/api/x")
            .header("Origin", "http://localhost:5173")
            .header("Access-Control-Request-Method", "GET")
            .send()
            .await;

        // Assert
        response.assert_status_is_ok();
        response.assert_header("access-control-allow-origin", "http://localhost:5173");
    }

    #[tokio::test]
    async fn should_refuse_a_preflight_when_the_origin_is_not_configured() {
        // Arrange
        let client = TestClient::new(wrap(
            &allowing("http://localhost:5173"),
            Route::new().at("/api/x", poem::get(hello)),
        ));

        // Act
        let response = client
            .options("/api/x")
            .header("Origin", "https://evil.example.com")
            .header("Access-Control-Request-Method", "GET")
            .send()
            .await;

        // Assert
        response.assert_status(StatusCode::FORBIDDEN);
        response.assert_header_is_not_exist("access-control-allow-origin");
    }

    #[tokio::test]
    async fn should_serve_the_health_probes_next_to_the_api() {
        // Arrange
        let rutas = rutas(
            OpenApiService::new((), "app", "1.0"),
            &Config::default(),
            sondear(|| async { false }),
        );
        let client = TestClient::new(rutas);

        // Act
        let live = client.get("/health").send().await;
        let ready = client.get("/health/ready").send().await;

        // Assert
        live.assert_status_is_ok();
        ready.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn should_serve_the_swagger_ui_at_the_root() {
        // Arrange
        let client = TestClient::new(rutas(
            OpenApiService::new((), "app", "1.0"),
            &Config::default(),
            siempre_disponible(),
        ));

        // Act
        let response = client.get("/").send().await;

        // Assert
        response.assert_status_is_ok();
        response.assert_content_type("text/html; charset=utf-8");
    }
}
