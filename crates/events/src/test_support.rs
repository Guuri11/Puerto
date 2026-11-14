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
    marked: Vec<Uuid>,
    mark_calls: usize,
    not_leading: bool,
    unreadable: bool,
}

#[derive(Clone, Default)]
pub struct FakeOutbox(Arc<Mutex<OutboxState>>);

impl FakeOutbox {
    pub fn with(events: &[SobreEvento]) -> Self {
        let outbox = Self::default();
        lock(&outbox.0).rows = events.iter().cloned().map(|e| (e, false)).collect();
        outbox
    }

    pub fn not_leading(self) -> Self {
        lock(&self.0).not_leading = true;
        self
    }

    pub fn unreadable(self) -> Self {
        lock(&self.0).unreadable = true;
        self
    }

    pub fn marked(&self) -> Vec<Uuid> {
        lock(&self.0).marked.clone()
    }

    pub fn mark_calls(&self) -> usize {
        lock(&self.0).mark_calls
    }

    pub fn unpublished(&self) -> Vec<Uuid> {
        lock(&self.0)
            .rows
            .iter()
            .filter(|(_, published)| !published)
            .map(|(e, _)| e.id)
            .collect()
    }
}

#[async_trait]
impl Outbox for FakeOutbox {
    async fn intentar_liderar(&self) -> Result<bool, ErrorTransporte> {
        Ok(!lock(&self.0).not_leading)
    }

    async fn buscar_sin_publicar(&self, limit: u32) -> Result<Vec<SobreEvento>, ErrorTransporte> {
        let state = lock(&self.0);
        if state.unreadable {
            return Err(ErrorTransporte::NoDisponible);
        }
        Ok(state
            .rows
            .iter()
            .filter(|(_, published)| !published)
            .take(limit as usize)
            .map(|(e, _)| e.clone())
            .collect())
    }

    async fn marcar_publicados(&self, ids: &[Uuid]) -> Result<(), ErrorTransporte> {
        let mut state = lock(&self.0);
        state.mark_calls += 1;
        state.marked.extend_from_slice(ids);
        for (event, published) in state.rows.iter_mut() {
            *published |= ids.contains(&event.id);
        }
        Ok(())
    }
}

/// Accepts a number of events, then fails each one.
#[derive(Clone, Default)]
pub struct FakePublisher(Arc<Mutex<(usize, Vec<Uuid>)>>);

impl FakePublisher {
    pub fn accepting(capacity: usize) -> Self {
        let publisher = Self::default();
        publisher.accept(capacity);
        publisher
    }

    /// Accepts `capacity` more events from now on.
    pub fn accept(&self, capacity: usize) {
        let mut state = lock(&self.0);
        state.0 = state.1.len().saturating_add(capacity);
    }

    pub fn published(&self) -> Vec<Uuid> {
        lock(&self.0).1.clone()
    }
}

#[async_trait]
impl PublicadorEventos for FakePublisher {
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
    settled: Vec<(Uuid, Entrega)>,
    settle_calls: usize,
    failing_settle: bool,
    watched: Option<FakeSink>,
    dead_letters_at_settle: Vec<usize>,
    max_batch: Option<u32>,
}

#[derive(Clone, Default)]
pub struct FakeSubscription(Arc<Mutex<SubscriptionState>>);

impl FakeSubscription {
    pub fn with(due: Vec<Reservado>) -> Self {
        let subscription = Self::default();
        lock(&subscription.0).due = due;
        subscription
    }

    /// Hands out one claim per batch, as a transport that has to keep an order does.
    pub fn one_at_a_time(self) -> Self {
        lock(&self.0).max_batch = Some(1);
        self
    }

    pub fn failing_settle(self) -> Self {
        lock(&self.0).failing_settle = true;
        self
    }

    /// Notes how many dead letters `sink` held each time a claim was settled.
    pub fn watching(self, sink: &FakeSink) -> Self {
        lock(&self.0).watched = Some(sink.clone());
        self
    }

    pub fn settled(&self) -> Vec<(Uuid, Entrega)> {
        lock(&self.0).settled.clone()
    }

    pub fn settle_calls(&self) -> usize {
        lock(&self.0).settle_calls
    }

    pub fn dead_letters_seen_at_settle(&self) -> Vec<usize> {
        lock(&self.0).dead_letters_at_settle.clone()
    }
}

#[async_trait]
impl SuscripcionEventos for FakeSubscription {
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
        state.settle_calls += 1;
        if state.failing_settle {
            return Err(ErrorTransporte::NoDisponible);
        }
        if let Some(sink) = &state.watched {
            let seen = sink.records().len();
            state.dead_letters_at_settle.push(seen);
        }
        state.settled.push((claimed.sobre.id, delivery.clone()));
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct FakeSink {
    records: Arc<Mutex<Vec<RegistroDescartado>>>,
    failing: bool,
}

impl FakeSink {
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }

    pub fn records(&self) -> Vec<RegistroDescartado> {
        lock(&self.records).clone()
    }
}

#[async_trait]
impl DestinoDescartados for FakeSink {
    async fn registrar(&self, record: &RegistroDescartado) -> Result<(), ErrorTransporte> {
        if self.failing {
            return Err(ErrorTransporte::NoDisponible);
        }
        lock(&self.records).push(record.clone());
        Ok(())
    }
}

/// A handler that answers what it was told to.
pub enum Scripted {
    Ack,
    Retry,
    /// Reads the payload as a number, which it never is: dead-letters it as invalid.
    DecodeAs,
    /// Panics on the first event, acks the rest.
    PanicOnce(Arc<AtomicBool>),
}

#[async_trait]
impl ManejadorEvento for Scripted {
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
