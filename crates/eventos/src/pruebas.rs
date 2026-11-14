//! Fakes of the transport ports, shared by the relay and consumer tests. Each one is a handle:
//! a clone sees what the loop under test did to the original.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use puerto_nucleo::eventos::entrega::Entrega;
use puerto_nucleo::eventos::transporte::{
    DestinoDescartados, ErrorTransporte, Outbox, PublicadorEventos, RegistroDescartado, Reservado,
    SuscripcionEventos,
};
use puerto_nucleo::eventos::{
    EventoDominio, MOTIVO_PAYLOAD_INVALIDO, ManejadorEvento, ResultadoManejo, SobreEvento,
};
use serde_json::{Value, json};
use uuid::Uuid;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Happened(&'static str, Uuid);

impl EventoDominio for Happened {
    fn tipo_evento(&self) -> &'static str {
        self.0
    }
    fn tipo_agregado(&self) -> &'static str {
        "product"
    }
    fn id_agregado(&self) -> Uuid {
        self.1
    }
    fn version_esquema(&self) -> u16 {
        1
    }
    fn payload(&self) -> Value {
        json!({ "id": self.1, "name": "Desk lamp" })
    }
}

pub fn envelope(event_type: &'static str) -> SobreEvento {
    SobreEvento::new(&Happened(event_type, Uuid::now_v7()))
}

#[derive(Default)]
struct OutboxState {
    rows: Vec<(SobreEvento, bool)>,
    marcados: Vec<Uuid>,
    llamadas_marcar: usize,
    sin_liderar: bool,
    unreadable: bool,
}

#[derive(Clone, Default)]
pub struct OutboxFalso(Arc<Mutex<OutboxState>>);

impl OutboxFalso {
    pub fn with(events: &[SobreEvento]) -> Self {
        let outbox = Self::default();
        lock(&outbox.0).rows = events.iter().cloned().map(|e| (e, false)).collect();
        outbox
    }

    pub fn sin_liderar(self) -> Self {
        lock(&self.0).sin_liderar = true;
        self
    }

    pub fn unreadable(self) -> Self {
        lock(&self.0).unreadable = true;
        self
    }

    pub fn marcados(&self) -> Vec<Uuid> {
        lock(&self.0).marcados.clone()
    }

    pub fn llamadas_marcar(&self) -> usize {
        lock(&self.0).llamadas_marcar
    }

    pub fn sin_publicar(&self) -> Vec<Uuid> {
        lock(&self.0)
            .rows
            .iter()
            .filter(|(_, publicados)| !publicados)
            .map(|(e, _)| e.id)
            .collect()
    }
}

#[async_trait]
impl Outbox for OutboxFalso {
    async fn intentar_liderar(&self) -> Result<bool, ErrorTransporte> {
        Ok(!lock(&self.0).sin_liderar)
    }

    async fn buscar_sin_publicar(&self, limit: u32) -> Result<Vec<SobreEvento>, ErrorTransporte> {
        let state = lock(&self.0);
        if state.unreadable {
            return Err(ErrorTransporte::NoDisponible);
        }
        Ok(state
            .rows
            .iter()
            .filter(|(_, publicados)| !publicados)
            .take(limit as usize)
            .map(|(e, _)| e.clone())
            .collect())
    }

    async fn marcar_publicados(&self, ids: &[Uuid]) -> Result<(), ErrorTransporte> {
        let mut state = lock(&self.0);
        state.llamadas_marcar += 1;
        state.marcados.extend_from_slice(ids);
        for (event, publicados) in state.rows.iter_mut() {
            *publicados |= ids.contains(&event.id);
        }
        Ok(())
    }
}

/// Accepts a number of events, then fails each one.
#[derive(Clone, Default)]
pub struct PublicadorFalso(Arc<Mutex<(usize, Vec<Uuid>)>>);

impl PublicadorFalso {
    pub fn aceptando(capacity: usize) -> Self {
        let publicador = Self::default();
        publicador.aceptar(capacity);
        publicador
    }

    /// Accepts `capacity` more events from now on.
    pub fn aceptar(&self, capacity: usize) {
        let mut state = lock(&self.0);
        state.0 = state.1.len().saturating_add(capacity);
    }

    pub fn publicados(&self) -> Vec<Uuid> {
        lock(&self.0).1.clone()
    }
}

#[async_trait]
impl PublicadorEventos for PublicadorFalso {
    async fn publicar(&self, event: &SobreEvento) -> Result<(), ErrorTransporte> {
        let mut state = lock(&self.0);
        if state.1.len() >= state.0 {
            return Err(ErrorTransporte::NoDisponible);
        }
        state.1.push(event.id);
        Ok(())
    }
}

#[derive(Default)]
struct SubscriptionState {
    due: Vec<Reservado>,
    resueltos: Vec<(Uuid, Entrega)>,
    llamadas_resolver: usize,
    fallando_al_resolver: bool,
    watched: Option<DestinoFalso>,
    dead_letters_at_settle: Vec<usize>,
    max_batch: Option<u32>,
}

#[derive(Clone, Default)]
pub struct SuscripcionFalsa(Arc<Mutex<SubscriptionState>>);

impl SuscripcionFalsa {
    pub fn with(due: Vec<Reservado>) -> Self {
        let suscripcion = Self::default();
        lock(&suscripcion.0).due = due;
        suscripcion
    }

    /// Hands out one claim per batch, as a transport that has to keep an order does.
    pub fn de_uno_en_uno(self) -> Self {
        lock(&self.0).max_batch = Some(1);
        self
    }

    pub fn fallando_al_resolver(self) -> Self {
        lock(&self.0).fallando_al_resolver = true;
        self
    }

    /// Notes how many dead letters `sink` held each time a claim was settled.
    pub fn observando(self, sink: &DestinoFalso) -> Self {
        lock(&self.0).watched = Some(sink.clone());
        self
    }

    pub fn resueltos(&self) -> Vec<(Uuid, Entrega)> {
        lock(&self.0).resueltos.clone()
    }

    pub fn llamadas_resolver(&self) -> usize {
        lock(&self.0).llamadas_resolver
    }

    pub fn descartados_vistos_al_resolver(&self) -> Vec<usize> {
        lock(&self.0).dead_letters_at_settle.clone()
    }
}

#[async_trait]
impl SuscripcionEventos for SuscripcionFalsa {
    fn consumidor(&self) -> &str {
        "notify_new_product"
    }

    fn lote_maximo(&self) -> u32 {
        lock(&self.0).max_batch.unwrap_or(u32::MAX)
    }

    async fn siguiente_lote(&mut self, max: u32) -> Result<Vec<Reservado>, ErrorTransporte> {
        let mut state = lock(&self.0);
        let take = state.due.len().min(max as usize);
        Ok(state.due.drain(..take).collect())
    }

    async fn resolver(
        &mut self,
        claimed: &Reservado,
        delivery: &Entrega,
    ) -> Result<(), ErrorTransporte> {
        let mut state = lock(&self.0);
        state.llamadas_resolver += 1;
        if state.fallando_al_resolver {
            return Err(ErrorTransporte::NoDisponible);
        }
        if let Some(sink) = &state.watched {
            let seen = sink.registros().len();
            state.dead_letters_at_settle.push(seen);
        }
        state.resueltos.push((claimed.sobre.id, delivery.clone()));
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct DestinoFalso {
    registros: Arc<Mutex<Vec<RegistroDescartado>>>,
    fallando: bool,
}

impl DestinoFalso {
    pub fn fallando() -> Self {
        Self {
            fallando: true,
            ..Self::default()
        }
    }

    pub fn registros(&self) -> Vec<RegistroDescartado> {
        lock(&self.registros).clone()
    }
}

#[async_trait]
impl DestinoDescartados for DestinoFalso {
    async fn registrar(&self, record: &RegistroDescartado) -> Result<(), ErrorTransporte> {
        if self.fallando {
            return Err(ErrorTransporte::NoDisponible);
        }
        lock(&self.registros).push(record.clone());
        Ok(())
    }
}

/// A handler that answers what it was told to.
pub enum Guion {
    Ack,
    Retry,
    /// Reads the payload as a number, which it never is: dead-letters it as invalid.
    DecodeAs,
    /// Panics on the first event, acks the rest.
    PanicOnce(Arc<AtomicBool>),
}

#[async_trait]
impl ManejadorEvento for Guion {
    async fn manejar(&self, event: &SobreEvento) -> ResultadoManejo {
        match self {
            Self::Ack => ResultadoManejo::Ack,
            Self::Retry => ResultadoManejo::Reintentar,
            Self::DecodeAs => match event.decodificar::<u64>() {
                Ok(_) => ResultadoManejo::Ack,
                Err(err) => ResultadoManejo::descartar(MOTIVO_PAYLOAD_INVALIDO, err.detalle),
            },
            Self::PanicOnce(panicked) => {
                if !panicked.swap(true, Ordering::SeqCst) {
                    panic!("handler bug");
                }
                ResultadoManejo::Ack
            }
        }
    }
}
