//! The publisher and the subscription against a real broker — `make test/kafka` (Redpanda). Every
//! test uses a topic prefix of its own, so its topics and groups start empty.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use puerto_core::events::delivery::Delivery;
use puerto_core::events::transport::{
    Claimed, DeadLetterRecord, DeadLetterSink, EventPublisher, EventSubscription, TransportError,
};
use puerto_core::{DomainEvent, EventEnvelope};
use puerto_kafka::{KafkaConfig, KafkaPublisher, KafkaSubscription};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json::{Value, json};
use uuid::Uuid;

const SKIP: &str = "PUERTO_TEST_KAFKA_BROKERS not set — skipping (run `make test/kafka`)";
/// How long a subscription may take to join its group and see a record.
const WAIT: Duration = Duration::from_secs(30);

struct Happened {
    event_type: &'static str,
    id: Uuid,
    name: &'static str,
}

impl DomainEvent for Happened {
    fn event_type(&self) -> &'static str {
        self.event_type
    }
    fn aggregate_type(&self) -> &'static str {
        "product"
    }
    fn aggregate_id(&self) -> Uuid {
        self.id
    }
    fn schema_version(&self) -> u16 {
        1
    }
    fn payload(&self) -> Value {
        json!({ "id": self.id, "name": self.name })
    }
}

fn event(event_type: &'static str, id: Uuid, name: &'static str) -> EventEnvelope {
    EventEnvelope::new(&Happened {
        event_type,
        id,
        name,
    })
}

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<DeadLetterRecord>>>);

impl Sink {
    fn records(&self) -> Vec<DeadLetterRecord> {
        self.0.lock().unwrap().clone()
    }
}

#[async_trait]
impl DeadLetterSink for Sink {
    async fn record(&self, record: &DeadLetterRecord) -> Result<(), TransportError> {
        self.0.lock().unwrap().push(record.clone());
        Ok(())
    }
}

/// A config of its own, with the `product` topic created.
async fn kafka() -> Option<KafkaConfig> {
    let brokers = std::env::var("PUERTO_TEST_KAFKA_BROKERS").ok()?;
    let prefix = format!("t{}", Uuid::now_v7().simple());
    let config = KafkaConfig::from_vars("unused", |key| match key {
        "KAFKA_BROKERS" => Some(brokers.clone()),
        "KAFKA_TOPIC_PREFIX" => Some(prefix.clone()),
        _ => None,
    })
    .unwrap();
    config.ensure_topics(&["product"]).await.unwrap();
    Some(config)
}

fn subscribe(config: &KafkaConfig, sink: &Sink) -> KafkaSubscription {
    KafkaSubscription::new(
        config,
        "notify_new_product",
        &["product"],
        &["product_created", "product_published"],
        Arc::new(sink.clone()),
    )
    .unwrap()
}

/// The next claim, waiting for the group to be joined; `None` when nothing came within `wait`.
async fn next(subscription: &mut KafkaSubscription, wait: Duration) -> Option<Claimed> {
    let deadline = tokio::time::Instant::now() + wait;
    while tokio::time::Instant::now() < deadline {
        let mut batch = subscription.next_batch(10).await.unwrap();
        assert!(batch.len() <= 1, "one record per batch");
        if let Some(claimed) = batch.pop() {
            return Some(claimed);
        }
    }
    None
}

#[tokio::test]
#[ignore = "needs Kafka: make test/kafka"]
async fn kafka_subscriber_receives_the_published_envelopes_of_its_types_in_order() {
    let Some(config) = kafka().await else {
        return eprintln!("{SKIP}");
    };
    let id = Uuid::now_v7();
    let created = event("product_created", id, "Desk lamp");
    let updated = event("product_updated", id, "Desk lamp");
    let published = event("product_published", id, "Desk lamp");
    let publisher = KafkaPublisher::new(&config).unwrap();
    for event in [&created, &updated, &published] {
        publisher.publish(event).await.unwrap();
    }
    let mut subscription = subscribe(&config, &Sink::default());

    let first = next(&mut subscription, WAIT).await.unwrap();
    subscription.settle(&first, &Delivery::Ack).await.unwrap();
    let second = next(&mut subscription, WAIT).await.unwrap();
    subscription.settle(&second, &Delivery::Ack).await.unwrap();

    assert_eq!(first, Claimed::new(created, 1));
    assert_eq!(second, Claimed::new(published, 1));
    assert_eq!(next(&mut subscription, Duration::from_secs(2)).await, None);
}

#[tokio::test]
#[ignore = "needs Kafka: make test/kafka"]
async fn kafka_retry_holds_back_the_next_record_and_hands_itself_out_as_the_next_attempt() {
    let Some(config) = kafka().await else {
        return eprintln!("{SKIP}");
    };
    let id = Uuid::now_v7();
    let created = event("product_created", id, "Desk lamp");
    let published = event("product_published", id, "Desk lamp");
    let publisher = KafkaPublisher::new(&config).unwrap();
    publisher.publish(&created).await.unwrap();
    publisher.publish(&published).await.unwrap();
    let mut subscription = subscribe(&config, &Sink::default());

    let first = next(&mut subscription, WAIT).await.unwrap();
    let retry = Delivery::Retry {
        delay: Duration::from_secs(1),
    };
    subscription.settle(&first, &retry).await.unwrap();
    let while_waiting = subscription.next_batch(10).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let again = next(&mut subscription, WAIT).await.unwrap();
    subscription.settle(&again, &Delivery::Ack).await.unwrap();
    let then = next(&mut subscription, WAIT).await.unwrap();

    assert_eq!(while_waiting, vec![]);
    assert_eq!(again, Claimed::new(created, 2));
    assert_eq!(then, Claimed::new(published, 1));
}

#[tokio::test]
#[ignore = "needs Kafka: make test/kafka"]
async fn kafka_poisoned_record_is_dead_lettered_and_the_next_one_is_still_consumed() {
    let Some(config) = kafka().await else {
        return eprintln!("{SKIP}");
    };
    let raw: FutureProducer = rdkafka::ClientConfig::new()
        .set("bootstrap.servers", &config.brokers)
        .create()
        .unwrap();
    raw.send(
        FutureRecord::<str, str>::to(&config.topic("product")).payload("not an event"),
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    let created = event("product_created", Uuid::now_v7(), "Desk lamp");
    KafkaPublisher::new(&config)
        .unwrap()
        .publish(&created)
        .await
        .unwrap();
    let sink = Sink::default();
    let mut subscription = subscribe(&config, &sink);

    let claimed = next(&mut subscription, WAIT).await.unwrap();

    assert_eq!(claimed, Claimed::new(created, 1));
    let records = sink.records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        (
            records[0].event_id,
            records[0].consumer.as_str(),
            records[0].reason.as_str(),
            records[0].payload.as_str()
        ),
        (
            None,
            "notify_new_product",
            "payload_invalid",
            "not an event"
        )
    );
}

#[tokio::test]
#[ignore = "needs Kafka: make test/kafka"]
async fn kafka_restarted_subscription_does_not_redeliver_what_was_committed() {
    let Some(config) = kafka().await else {
        return eprintln!("{SKIP}");
    };
    let id = Uuid::now_v7();
    let created = event("product_created", id, "Desk lamp");
    let published = event("product_published", id, "Desk lamp");
    let publisher = KafkaPublisher::new(&config).unwrap();
    publisher.publish(&created).await.unwrap();
    publisher.publish(&published).await.unwrap();
    let mut subscription = subscribe(&config, &Sink::default());
    let first = next(&mut subscription, WAIT).await.unwrap();
    subscription.settle(&first, &Delivery::Ack).await.unwrap();
    let unsettled = next(&mut subscription, WAIT).await.unwrap();
    drop(subscription);

    let mut restarted = subscribe(&config, &Sink::default());
    let after_restart = next(&mut restarted, WAIT).await.unwrap();

    assert_eq!(unsettled, Claimed::new(published.clone(), 1));
    assert_eq!(after_restart, Claimed::new(published, 1));
}
