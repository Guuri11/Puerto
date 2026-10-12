//! Liveness and readiness probes, served outside the OpenAPI service (ADR-0008).
//!
//! `GET /health` answers *is the process up?* — nothing more. `GET /health/ready` answers *can it
//! serve traffic?* by asking the dependencies (the database, in a `--db` project). The two are
//! separate on purpose: a liveness-only probe reports a stack whose database is down as healthy,
//! and an orchestrator that restarts on a failed *readiness* check restarts a process that was fine.

use std::{future::Future, pin::Pin, sync::Arc};

use poem::{Route, get, handler, http::StatusCode};

/// Asks the dependencies whether the application can serve traffic: `true` when every one answered.
pub type ReadinessProbe = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = bool> + Send>> + Send + Sync>;

/// A probe from an async check — `probe(move || { let pool = pool.clone(); async move { ping(&pool).await } })`.
pub fn probe<F, Fut>(check: F) -> ReadinessProbe
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    Arc::new(move || Box::pin(check()))
}

/// The probe of an application with no dependency to ask.
pub fn always_ready() -> ReadinessProbe {
    probe(|| async { true })
}

/// Adds `GET /health` and `GET /health/ready` to `route`.
pub fn mount(route: Route, ready: ReadinessProbe) -> Route {
    route.at("/health", get(liveness)).at(
        "/health/ready",
        get(poem::endpoint::make(move |_| {
            let ready = Arc::clone(&ready);
            async move { readiness(ready().await) }
        })),
    )
}

/// Liveness: the process answers.
#[handler]
fn liveness() -> &'static str {
    "ok"
}

/// 200 when every dependency answered, 503 otherwise.
fn readiness(ready: bool) -> (StatusCode, &'static str) {
    if ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

#[cfg(test)]
mod tests {
    use poem::test::TestClient;

    use super::*;

    #[tokio::test]
    async fn should_answer_200_on_liveness_when_a_dependency_is_down() {
        // Arrange
        let client = TestClient::new(mount(Route::new(), probe(|| async { false })));

        // Act
        let response = client.get("/health").send().await;

        // Assert
        response.assert_status_is_ok();
        response.assert_text("ok").await;
    }

    #[tokio::test]
    async fn should_answer_200_on_readiness_when_dependencies_are_ready() {
        // Arrange
        let client = TestClient::new(mount(Route::new(), always_ready()));

        // Act
        let response = client.get("/health/ready").send().await;

        // Assert
        response.assert_status_is_ok();
        response.assert_text("ready").await;
    }

    #[tokio::test]
    async fn should_answer_503_on_readiness_when_a_dependency_is_down() {
        // Arrange
        let client = TestClient::new(mount(Route::new(), probe(|| async { false })));

        // Act
        let response = client.get("/health/ready").send().await;

        // Assert
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
        response.assert_text("not ready").await;
    }

    #[tokio::test]
    async fn should_ask_the_dependencies_on_every_readiness_request() {
        // Arrange
        let up = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let flag = Arc::clone(&up);
        let client = TestClient::new(mount(
            Route::new(),
            probe(move || {
                let flag = Arc::clone(&flag);
                async move { flag.load(std::sync::atomic::Ordering::SeqCst) }
            }),
        ));
        client
            .get("/health/ready")
            .send()
            .await
            .assert_status_is_ok();

        // Act
        up.store(false, std::sync::atomic::Ordering::SeqCst);
        let response = client.get("/health/ready").send().await;

        // Assert
        response.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }
}
