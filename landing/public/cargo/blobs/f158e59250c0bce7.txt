//! Yours: `puerto new` writes this file once and no generator touches it again.
//!
//! `generated/bootstrap.rs` hands every route — the API, the Swagger UI, the health probes — to
//! `configure` last. Add middleware (`routes.with(…)`) or routes of your own (`routes.at(…)`) here.

use poem::{Endpoint, Route};

pub fn configure(routes: Route) -> impl Endpoint {
    routes
}
