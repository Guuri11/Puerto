//! The publisher and the subscription against a real broker — `make test/kafka` (Redpanda). Every
//! test uses a topic prefix of its own, so its topics and groups start empty.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use puerto_kafka::{ConfigKafka, PublicadorKafka, SuscripcionKafka};
use puerto_nucleo::eventos::entrega::Entrega;
use puerto_nucleo::eventos::transporte::{
    DestinoDescartados, ErrorTransporte, PublicadorEventos, RegistroDescartado, Reservado,
    SuscripcionEventos,
};
use puerto_nucleo::{EventoDominio, SobreEvento};
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

impl EventoDominio for Happened {
    fn tipo_evento(&self) -> &'static str {
        self.event_type
    }
    fn tipo_agregado(&self) -> &'static str {
        "product"
    }
    fn id_agregado(&self) -> Uuid {
        self.id
    }
    fn version_esquema(&self) -> u16 {
        1
    }
    fn payload(&self) -> Value {
        json!({ "id": self.id, "name": self.name })
    }
}

fn event(event_type: &'static str, id: Uuid, name: &'static str) -> SobreEvento {
    SobreEvento::new(&Happened {
        event_type,
        id,
        name,
    })
}

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<RegistroDescartado>>>);

impl Sink {
    fn registros(&self) -> Vec<RegistroDescartado> {
        self.0.lock().unwrap().clone()
    }
}

#[async_trait]
impl DestinoDescartados for Sink {
    async fn registrar(&self, record: &RegistroDescartado) -> Result<(), ErrorTransporte> {
        self.0.lock().unwrap().push(record.clone());
        Ok(())
    }
}

/// A config of its own, with the `product` topic created.
async fn kafka() -> Option<ConfigKafka> {
    let brokers = std::env::var("PUERTO_TEST_KAFKA_BROKERS").ok()?;
    let prefix = format!("t{}", Uuid::now_v7().simple());
    let config = ConfigKafka::from_variables("unused", |key| match key {
        "KAFKA_BROKERS" => Some(brokers.clone()),
        "KAFKA_TOPIC_PREFIX" => Some(prefix.clone()),
        _ => None,
    })
    .unwrap();
    config.ensure_topics(&["product"]).await.unwrap();
    Some(config)
}

fn subscribe(config: &ConfigKafka, sink: &Sink) -> SuscripcionKafka {
    SuscripcionKafka::new(
        config,
        "notify_new_product",
        &["product"],
        &["product_created", "product_published"],
        Arc::new(sink.clone()),
    )
    .unwrap()
}

/// The next claim, waiting for the group to be joined; `None` when nothing came within `wait`.
async fn next(suscripcion: &mut SuscripcionKafka, wait: Duration) -> Option<Reservado> {
    let deadline = tokio::time::Instant::now() + wait;
    while tokio::time::Instant::now() < deadline {
        let mut batch = suscripcion.siguiente_lote(10).await.unwrap();
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
    let publicados = event("product_published", id, "Desk lamp");
    let publicador = PublicadorKafka::new(&config).unwrap();
    for event in [&created, &updated, &publicados] {
        publicador.publicar(event).await.unwrap();
    }
    let mut suscripcion = subscribe(&config, &Sink::default());

    let first = next(&mut suscripcion, WAIT).await.unwrap();
    suscripcion.resolver(&first, &Entrega::Ack).await.unwrap();
    let second = next(&mut suscripcion, WAIT).await.unwrap();
    suscripcion.resolver(&second, &Entrega::Ack).await.unwrap();

    assert_eq!(first, Reservado::new(created, 1));
    assert_eq!(second, Reservado::new(publicados, 1));
    assert_eq!(next(&mut suscripcion, Duration::from_secs(2)).await, None);
}

#[tokio::test]
#[ignore = "needs Kafka: make test/kafka"]
async fn kafka_retry_holds_back_the_next_record_and_hands_itself_out_as_the_next_attempt() {
    let Some(config) = kafka().await else {
        return eprintln!("{SKIP}");
    };
    let id = Uuid::now_v7();
    let created = event("product_created", id, "Desk lamp");
    let publicados = event("product_published", id, "Desk lamp");
    let publicador = PublicadorKafka::new(&config).unwrap();
    publicador.publicar(&created).await.unwrap();
    publicador.publicar(&publicados).await.unwrap();
    let mut suscripcion = subscribe(&config, &Sink::default());

    let first = next(&mut suscripcion, WAIT).await.unwrap();
    let retry = Entrega::Reintentar {
        espera: Duration::from_secs(1),
    };
    suscripcion.resolver(&first, &retry).await.unwrap();
    let while_waiting = suscripcion.siguiente_lote(10).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let again = next(&mut suscripcion, WAIT).await.unwrap();
    suscripcion.resolver(&again, &Entrega::Ack).await.unwrap();
    let then = next(&mut suscripcion, WAIT).await.unwrap();

    assert_eq!(while_waiting, vec![]);
    assert_eq!(again, Reservado::new(created, 2));
    assert_eq!(then, Reservado::new(publicados, 1));
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
    PublicadorKafka::new(&config)
        .unwrap()
        .publicar(&created)
        .await
        .unwrap();
    let sink = Sink::default();
    let mut suscripcion = subscribe(&config, &sink);

    let claimed = next(&mut suscripcion, WAIT).await.unwrap();

    assert_eq!(claimed, Reservado::new(created, 1));
    let registros = sink.registros();
    assert_eq!(registros.len(), 1);
    assert_eq!(
        (
            registros[0].id_evento,
            registros[0].consumidor.as_str(),
            registros[0].motivo.as_str(),
            registros[0].payload.as_str()
        ),
        (
            None,
            "notify_new_product",
            "payload_invalido",
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
    let publicados = event("product_published", id, "Desk lamp");
    let publicador = PublicadorKafka::new(&config).unwrap();
    publicador.publicar(&created).await.unwrap();
    publicador.publicar(&publicados).await.unwrap();
    let mut suscripcion = subscribe(&config, &Sink::default());
    let first = next(&mut suscripcion, WAIT).await.unwrap();
    suscripcion.resolver(&first, &Entrega::Ack).await.unwrap();
    let unsettled = next(&mut suscripcion, WAIT).await.unwrap();
    drop(suscripcion);

    let mut restarted = subscribe(&config, &Sink::default());
    let after_restart = next(&mut restarted, WAIT).await.unwrap();

    assert_eq!(unsettled, Reservado::new(publicados.clone(), 1));
    assert_eq!(after_restart, Reservado::new(publicados, 1));
}
