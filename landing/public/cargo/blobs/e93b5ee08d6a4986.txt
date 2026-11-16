//! La API: la arranca `main.rs`. El cableado se genera (`generado/bootstrap.rs`); tu middleware y
//! tus rutas van en `app.rs`.

mod app;
pub mod generado {
    pub mod bootstrap;
}
