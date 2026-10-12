# 0018 — Domain events: unit of work, outbox, pluggable transport

**Status:** Accepted (2026-10-12). Spec: `PLAN.md` § Phase 13. Backlog: T-23 (T-29 is part of it).

## Context

A Puerto write use case calls `repository.save(&entity)` and returns. Nothing else learns that it
happened, so every reaction (send a mail, update a projection, notify another service) has to be
called from inside the use case. When one is added later, every existing write use case has to
change, which is the refactor Puerto exists to prevent. Mailer, jobs, webhooks and audit (the
pieces left after ADR-0017) all need the same thing: something to react to.

`document-api` (in `nexe-document`) runs this architecture in production. Every write goes through
a per-context Unit of Work that saves the aggregate and a `DomainEvent` row in an `outbox` table in
one Postgres transaction. An `OutboxWorker` relays unpublished rows in order to NATS JetStream,
stopping at the first failure. Consumers implement a `MessageHandler` that answers `Ack`, `Nak` or
`DeadLetter`. A pure function (`delivery_policy::decide`) applies a delivery cap and a growing
backoff, and nothing is retired without a `dead_letters` row (its ADR-0009). Three things in it do
not carry over to a generator:

- **The consumer side is bound to JetStream.** The relay talks to an `EventPublisher` port, but
  the consume loop is `async_nats` code. Puerto has to serve projects with no broker at all, and
  projects on Kafka.
- **The event payload is the domain model**, serialised with serde. That puts serde on every
  entity and value object, and a change to the model silently changes a public contract.
- **One global `DomainEvent` enum** that every context's events are lifted into. In a generated
  project it would be one more file regenerated from every entity, and it would couple them.

## Decision

### 1. Events require a database, and are opt-in

`puerto new --db --events db|kafka` writes `[project] events = "db"` (or `"kafka"`). `--events`
without `--db` is refused, and so is `events` without `db` in `puerto validate`. The outbox only
works inside a real transaction. An in-memory bus would look the same and guarantee nothing when
the process dies. A project without `events` is generated exactly as today.

### 2. Every write goes through a per-entity Unit of Work

`business/src/domain/<entity>/unit_of_work.rs` holds two ports:

```rust
#[async_trait]
pub trait ProductUnitOfWork: Send {
    async fn save(&mut self, entity: &Product) -> Result<(), RepositoryError>;
    async fn record(&mut self, event: ProductEvent) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait ProductUnitOfWorkFactory: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn ProductUnitOfWork>, RepositoryError>;
}
```

- Dropping a UoW without calling `commit` rolls it back, because a sqlx `Transaction` does. There
  is no `rollback` method: an early `?` is the rollback.
- In an `events` project the repository port **loses `save`** and only reads. A write that skips
  the outbox then fails to compile. That is the only check that holds when an agent writes the
  next use case.
- The CRUD `create`/`update`/`delete` use cases take `uow_factory`, plus `repository` when they
  read first. Each records `{Entity}Event::Created`/`Updated`/`Deleted` before `commit`.
- One aggregate per transaction (the DDD consistency boundary). A use case that must write two
  aggregates atomically is hand-written against a hand-written UoW port. The generator does not
  model cross-aggregate transactions in v1.

### 3. The event payload is its own contract, not the model

`business/src/domain/<entity>/events.rs` is **fully generated** from puerto.toml and regenerated
like `bootstrap.rs`:

- `{Entity}Snapshot` has primitive fields, `#[derive(Serialize, Deserialize)]` and the same shape
  as the DTO (value objects unwrapped, the id a `Uuid`). It is built from the entity.
- `enum {Entity}Event { Created(Snapshot), Updated(Snapshot), Deleted(Snapshot), … }`. Custom
  events declared in puerto.toml (`events = ["product_published"]`) add variants that carry the
  same snapshot.
- It implements `puerto_core::DomainEvent`: `event_type()` (`"product_created"`, past tense,
  snake_case), `aggregate_type()` (`"product"`), `aggregate_id()`, `schema_version()` (1) and
  `payload()` (JSON).

The domain models and value objects stay serde-free. `business` gains `serde`/`serde_json`, and
only `events.rs` uses them, in the same way DTOs isolate the HTTP contract (ADR-0012). There is no
global event enum. A consumer decodes the snapshot type of the event it subscribed to.

### 4. One envelope, two ports, the transport is an adapter

`puerto-core` holds the transport-neutral parts (ADR-0017 § 2: ports in core):

- `EventEnvelope`: `id` (the outbox row id, used as the dedup key downstream), `event_type`,
  `aggregate_type`, `aggregate_id`, `schema_version`, `occurred_at`, `payload`, plus `headers`.
  `headers` is a string map, so `correlation_id` today and `tenant_id` later (T-25) need no new
  field. It is `#[non_exhaustive]`, with a constructor.
- `EventHandler`: `async fn handle(&self, event: &EventEnvelope) -> HandlerOutcome`, with
  `HandlerOutcome::{Ack, Retry, DeadLetter { reason, detail }}`.
- The delivery policy, as `document-api` has it: a pure `decide(outcome, attempt, max)` plus a
  growing backoff (1 s, 5 s, 15 s, 60 s, 300 s, max 5 deliveries). Every transport applies the
  same function, so its tests and mutants cover all of them.
- The transport ports the worker drives: a publisher (the relay's output) and a subscription (a
  consumer's input). How they are shaped is settled in 13.1, with the code.

The adapters live in their own crates, so a project only pays for the one it uses:

| Transport | Crate | Relay "publishes" by | A consumer reads | Retry / dead letter |
|---|---|---|---|---|
| `db` | `puerto-sqlx` (`events` module) | inserting one `event_deliveries` row per subscribed consumer, in the same transaction that marks the outbox row published | claiming its rows with `FOR UPDATE SKIP LOCKED` | `attempts`, `next_attempt_at` and `status = 'dead'` on the delivery row |
| `kafka` | `puerto-kafka` (`rdkafka`) | producing to topic `<project>.<aggregate_type>`, keyed by `aggregate_id`, the envelope fields in headers | a consumer group named after the consumer, offsets committed by hand after the outcome | retried in-process with the same backoff (this blocks the partition and keeps the order), then a `dead_letters` row in Postgres and a commit |

RabbitMQ and NATS can come later as one crate each, implementing the same two ports, without
changing any generated code.

### 5. The relay and the consumers run in a second binary: `worker/`

An `events` project gets a `worker` crate next to `presentation`. Like the API, it is the
composition root of its own process, so it scales and is deployed separately:

- `worker/src/main.rs` (written once): `puerto_events::Worker::new(config).run(build_worker)`.
- `worker/src/generated/registry.rs` (AUTO-GENERATED from puerto.toml `[[consumer]]` entries):
  wires the relay, the transport and every consumer.
- `worker/src/consumers/<name>.rs` (written once by `puerto generate consumer`, then the user's
  own): an `EventHandler` that decodes the snapshot and calls a use case, with tests.
- The relay holds a Postgres advisory lock, so with several worker replicas exactly one relays
  and the outbox order is kept. Consumers scale with replicas. Per-aggregate order is guaranteed
  by Kafka's partition key; on the `db` transport it is guaranteed only with one replica per
  consumer.
- The worker serves `/health` on `WORKER_PORT` (`puerto_web::health`) and shuts down gracefully
  on SIGTERM (ADR-0008). A loop that stops turning shows up as not ready.

The loops (relay, consume, health, shutdown) are identical in every project, so they live in a
new runtime crate `puerto-events` (tokio, tracing), generic over the core ports.

### 6. Delivery is at-least-once, so handlers must be idempotent

No transport here guarantees exactly-once. The envelope `id` is stable across redeliveries. The
generated consumer stub says so, and its test delivers the same envelope twice. A dropped message
always leaves a dead-letter record, and a failed dead-letter write means the message is delivered
again (`document-api`'s ADR-0009, adopted unchanged).

## Alternatives rejected

- **The repository writes the events (`save(&entity, &events)`).** It is simpler for one
  aggregate, but there is no place to grow into a multi-write transaction, and "every write emits
  an event" lives in each adapter instead of in the use case, where tests can see it.
- **A generic `UnitOfWork` in `puerto-core`** (`save<T>(&mut self, entity: &T)`). That is not
  object-safe behind `Arc<dyn …>`. A generated per-entity port is plain, mockable and readable.
- **Serialising the domain model** (`document-api`'s choice). See Context.
- **Running the relay and consumers inside the API process.** It is simpler to deploy, but the API
  and the workers can then only scale together, and a slow consumer competes with requests for
  the pool. A dev-only in-process flag can come later without changing this design.
- **NATS/RabbitMQ first.** Postgres serves the "no broker" case, and Kafka the most common
  broker. The ports are what keeps the others cheap.

## Consequences

- An `events` project's writes cannot skip the outbox without a compile error, and a reaction is
  added as a consumer without touching the use case that emits the event.
- Mailer (Phase 14), jobs, webhooks and audit become consumers plus configuration rather than
  edits to write use cases.
- `business` depends on `serde`/`serde_json`. `puerto-core` grows ports and the delivery policy
  (`async-trait`, `serde_json`, `uuid`, `chrono`), so ADR-0017's dependency table for it is
  extended.
- New runtime crates `puerto-events` and `puerto-kafka`, released in lockstep (ADR-0017 § 4).
  `rdkafka` needs `cmake` and a C toolchain, which only Kafka projects and Puerto's CI pay for.
- On the `db` transport, a consumer registered later receives only the events relayed after it
  is deployed. Backfilling from the outbox is future work.
