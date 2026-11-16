//! Lo escribe una vez `puerto nuevo`; desde entonces es tuyo y ningún generador vuelve a tocarlo.
//!
//! `generado/bootstrap.rs` entrega todas las rutas —la API, la Swagger UI, las sondas de salud— a
//! `configurar`, lo último. Añade aquí middleware (`rutas.with(…)`) o rutas propias
//! (`rutas.at(…)`).

use poem::{Endpoint, Route};

pub fn configurar(rutas: Route) -> impl Endpoint {
    rutas
}
