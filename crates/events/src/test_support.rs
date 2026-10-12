//! Fakes of the transport ports, shared by the relay and consumer tests. Each one is a handle:
//! a clone sees what the loop under test did to the original.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use puerto_core::events::delivery::Delivery;
use puerto_core::events::transport::{
    Claimed, DeadLetterRecord, DeadLetterSink, EventPublisher, EventSubscription, Outbox,
    TransportError,
};
use puerto_core::events::{
    DomainEvent, EventEnvelope, EventHandler, HandlerOutcome, REASON_PAYLOAD_INVALID,
};
use serde_json::{Value, json};
use uuid::Uuid;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Happened(&'static str, Uuid);

impl DomainEvent for Happened {
    fn event_type(&self) -> &'static str {
        self.0
    }
    fn aggregate_type(&self) -> &'static str {
        "product"
    }
    fn aggregate_id(&self) -> Uuid {
        self.1
    }
    fn schema_version(&self) -> u16 {
        1
    }
    fn payload(&self) -> Value {
        json!({ "id": self.1, "name": "Desk lamp" })
    }
}

pub fn envelope(event_type: &'static str) -> EventEnvelope {
    EventEnvelope::new(&Happened(event_type, Uuid::now_v7()))
}

#[derive(Default)]
struct OutboxState {
    rows: Vec<(EventEnvelope, bool)>,
    marked: Vec<Uuid>,
    mark_calls: usize,
    not_leading: bool,
    unreadable: bool,
}

#[derive(Clone, Default)]
pub struct FakeOutbox(Arc<Mutex<OutboxState>>);

impl FakeOutbox {
    pub fn with(events: &[EventEnvelope]) -> Self {
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
    async fn try_lead(&self) -> Result<bool, TransportError> {
        Ok(!lock(&self.0).not_leading)
    }

    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<EventEnvelope>, TransportError> {
        let state = lock(&self.0);
        if state.unreadable {
            return Err(TransportError::Unavailable);
        }
        Ok(state
            .rows
            .iter()
            .filter(|(_, published)| !published)
            .take(limit as usize)
            .map(|(e, _)| e.clone())
            .collect())
    }

    async fn mark_published(&self, ids: &[Uuid]) -> Result<(), TransportError> {
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
impl EventPublisher for FakePublisher {
    async fn publish(&self, event: &EventEnvelope) -> Result<(), TransportError> {
        let mut state = lock(&self.0);
        if state.1.len() >= state.0 {
            return Err(TransportError::Unavailable);
        }
        state.1.push(event.id);
        Ok(())
    }
}

#[derive(Default)]
struct SubscriptionState {
    due: Vec<Claimed>,
    settled: Vec<(Uuid, Delivery)>,
    settle_calls: usize,
    failing_settle: bool,
    watched: Option<FakeSink>,
    dead_letters_at_settle: Vec<usize>,
    max_batch: Option<u32>,
}

#[derive(Clone, Default)]
pub struct FakeSubscription(Arc<Mutex<SubscriptionState>>);

impl FakeSubscription {
    pub fn with(due: Vec<Claimed>) -> Self {
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

    pub fn settled(&self) -> Vec<(Uuid, Delivery)> {
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
impl EventSubscription for FakeSubscription {
    fn consumer(&self) -> &str {
        "notify_new_product"
    }

    fn max_batch(&self) -> u32 {
        lock(&self.0).max_batch.unwrap_or(u32::MAX)
    }

    async fn next_batch(&mut self, max: u32) -> Result<Vec<Claimed>, TransportError> {
        let mut state = lock(&self.0);
        let take = state.due.len().min(max as usize);
        Ok(state.due.drain(..take).collect())
    }

    async fn settle(
        &mut self,
        claimed: &Claimed,
        delivery: &Delivery,
    ) -> Result<(), TransportError> {
        let mut state = lock(&self.0);
        state.settle_calls += 1;
        if state.failing_settle {
            return Err(TransportError::Unavailable);
        }
        if let Some(sink) = &state.watched {
            let seen = sink.records().len();
            state.dead_letters_at_settle.push(seen);
        }
        state.settled.push((claimed.envelope.id, delivery.clone()));
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct FakeSink {
    records: Arc<Mutex<Vec<DeadLetterRecord>>>,
    failing: bool,
}

impl FakeSink {
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }

    pub fn records(&self) -> Vec<DeadLetterRecord> {
        lock(&self.records).clone()
    }
}

#[async_trait]
impl DeadLetterSink for FakeSink {
    async fn record(&self, record: &DeadLetterRecord) -> Result<(), TransportError> {
        if self.failing {
            return Err(TransportError::Unavailable);
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
impl EventHandler for Scripted {
    async fn handle(&self, event: &EventEnvelope) -> HandlerOutcome {
        match self {
            Self::Ack => HandlerOutcome::Ack,
            Self::Retry => HandlerOutcome::Retry,
            Self::DecodeAs => match event.decode::<u64>() {
                Ok(_) => HandlerOutcome::Ack,
                Err(err) => HandlerOutcome::dead_letter(REASON_PAYLOAD_INVALID, err.detail),
            },
            Self::PanicOnce(panicked) => {
                if !panicked.swap(true, Ordering::SeqCst) {
                    panic!("handler bug");
                }
                HandlerOutcome::Ack
            }
        }
    }
}
