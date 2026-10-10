//! Liveness and readiness probes, served outside the OpenAPI service.
//!
//! `GET /health` answers *is the process up?* — nothing more. `GET /health/ready` answers *can it
//! serve traffic?* by asking the dependencies (the database, in a `--db` project). The two are
//! separate on purpose: a liveness-only probe reports a stack whose database is down as healthy,
//! and an orchestrator that restarts on a failed *readiness* check restarts a process that was fine.

use poem::{handler, http::StatusCode};

/// Liveness: the process answers.
#[handler]
pub fn liveness() -> &'static str {
    "ok"
}

/// The readiness response: 200 when every dependency answered, 503 otherwise. `bootstrap.rs` calls
/// it with the result of its dependency checks — or `true` when the project has none.
pub fn readiness(ready: bool) -> (StatusCode, &'static str) {
    if ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_answer_200_when_dependencies_are_ready() {
        // Act
        let (status, body) = readiness(true);

        // Assert
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ready");
    }

    #[test]
    fn should_answer_503_when_a_dependency_is_down() {
        // Act
        let (status, _) = readiness(false);

        // Assert
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }
}
