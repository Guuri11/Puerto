//! The API: `main.rs` starts it. The wiring is generated (`generado/bootstrap.rs`); your own
//! middleware and routes go in `app.rs`.

mod app;
pub mod generado {
    pub mod bootstrap;
}
