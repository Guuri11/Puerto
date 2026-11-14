//! The unit of work, the outbox and the `db` transport against a real server — `make test/db`.
//! Every test runs in a schema of its own, with `migrations/outbox.sql` applied.

use std::collections::HashSet;
use std::time::Duration;

use puerto_nucleo::eventos::entrega::Entrega;
use puerto_nucleo::eventos::transporte::{
    DestinoDescartados, Outbox, PublicadorEventos, RegistroDescartado, SuscripcionEventos,
};
use puerto_nucleo::{ErrorRepositorio, EventoDominio, SobreEvento};
use puerto_sqlx::eventos::{
    DescartadosPostgres, EntregasPostgres, OutboxPostgres, SuscripcionPostgres,
};
use puerto_sqlx::uow::TransaccionPostgres;
use puerto_sqlx::{MIGRACION_OUTBOX, mapear_error_sqlx};
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

const SKIP: &str = "PUERTO_TEST_DATABASE_URL not set — skipping (run `make test/db`)";

/// A pool whose connections all see a fresh schema holding the outbox tables and `products`.
async fn database() -> Option<PgPool> {
    let server = std::env::var("PUERTO_TEST_DATABASE_URL").ok()?;
    let url = format!("{server}/postgres");
    let schema = format!("events_{}", Uuid::now_v7().simple());
    let admin = PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;

    let options: PgConnectOptions = url.parse().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect_with(options.options([("search_path", schema.as_str())]))
        .await
        .unwrap();
    sqlx::raw_sql(MIGRACION_OUTBOX)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE products (id UUID PRIMARY KEY, name TEXT NOT NULL UNIQUE)")
        .execute(&pool)
        .await
        .unwrap();
    Some(pool)
}

struct ProductCreated {
    id: Uuid,
    name: &'static str,
}

impl EventoDominio for ProductCreated {
    fn tipo_evento(&self) -> &'static str {
        "product_created"
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

/// What a generated `Pg{Entity}UnitOfWork` does: the entity's write, then its event.
async fn save_product(
    uow: &mut TransaccionPostgres,
    name: &'static str,
) -> Result<Uuid, ErrorRepositorio> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO products (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(uow.tx())
        .await
        .map_err(|err| mapear_error_sqlx(&err))?;
    uow.registrar(&ProductCreated { id, name }).await?;
    Ok(id)
}

async fn count(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

/// Writes one product and its event per name through committed units of work, oldest first, and
/// returns the products' ids — the events' `aggregate_id`s.
async fn write_events(pool: &PgPool, names: &[&'static str]) -> Vec<Uuid> {
    let mut ids = Vec::new();
    for name in names {
        let mut uow = TransaccionPostgres::begin(pool).await.unwrap();
        ids.push(save_product(&mut uow, name).await.unwrap());
        uow.commit().await.unwrap();
    }
    ids
}

async fn publish_all(pool: &PgPool, deliveries: &EntregasPostgres) {
    let outbox = OutboxPostgres::new(pool.clone());
    for event in outbox.buscar_sin_publicar(1000).await.unwrap() {
        deliveries.publicar(&event).await.unwrap();
    }
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_committed_unit_of_work_writes_the_entity_and_its_outbox_row() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };

    let mut uow = TransaccionPostgres::begin(&pool).await.unwrap();
    let id = save_product(&mut uow, "Desk lamp").await.unwrap();
    uow.commit().await.unwrap();

    assert_eq!(count(&pool, "SELECT count(*) FROM products").await, 1);
    let eventos = OutboxPostgres::new(pool.clone())
        .buscar_sin_publicar(10)
        .await
        .unwrap();
    assert_eq!(eventos.len(), 1);
    assert_eq!(eventos[0].tipo_evento, "product_created");
    assert_eq!(eventos[0].tipo_agregado, "product");
    assert_eq!(eventos[0].id_agregado, id);
    assert_eq!(eventos[0].payload, json!({ "id": id, "name": "Desk lamp" }));
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_dropped_unit_of_work_writes_neither_the_entity_nor_its_event() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };

    {
        let mut uow = TransaccionPostgres::begin(&pool).await.unwrap();
        save_product(&mut uow, "Desk lamp").await.unwrap();
    }

    assert_eq!(count(&pool, "SELECT count(*) FROM products").await, 0);
    assert_eq!(count(&pool, "SELECT count(*) FROM outbox").await, 0);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_unit_of_work_rejecting_a_duplicate_records_nothing() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;

    let mut uow = TransaccionPostgres::begin(&pool).await.unwrap();
    let result = save_product(&mut uow, "Desk lamp").await;
    drop(uow);

    assert_eq!(result, Err(ErrorRepositorio::Duplicado));
    assert_eq!(count(&pool, "SELECT count(*) FROM outbox").await, 1);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_recorded_envelope_keeps_its_headers() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let event = SobreEvento::new(&ProductCreated {
        id: Uuid::now_v7(),
        name: "Desk lamp",
    })
    .con_cabecera("correlation_id", "req-1");

    let mut uow = TransaccionPostgres::begin(&pool).await.unwrap();
    uow.registrar_sobre(&event).await.unwrap();
    uow.commit().await.unwrap();

    let stored = OutboxPostgres::new(pool.clone())
        .buscar_sin_publicar(10)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id, event.id);
    assert_eq!(stored[0].cabeceras, event.cabeceras);
    // Postgres keeps microseconds.
    assert_eq!(
        stored[0].ocurrido_en.timestamp_micros(),
        event.ocurrido_en.timestamp_micros()
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_outbox_returns_unpublished_rows_in_write_order_and_skips_marked_ones() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let products = write_events(&pool, &["a", "b", "c", "d"]).await;
    let outbox = OutboxPostgres::new(pool.clone());
    let aggregates = |eventos: Vec<SobreEvento>| -> Vec<Uuid> {
        eventos.iter().map(|e| e.id_agregado).collect()
    };

    let first_two = outbox.buscar_sin_publicar(2).await.unwrap();
    let all = outbox.buscar_sin_publicar(10).await.unwrap();
    outbox
        .marcar_publicados(&[all[0].id, all[2].id])
        .await
        .unwrap();
    outbox.marcar_publicados(&[all[0].id]).await.unwrap();
    let left = outbox.buscar_sin_publicar(10).await.unwrap();

    assert_eq!(aggregates(first_two), products[..2]);
    assert_eq!(aggregates(left), [products[1], products[3]]);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_relay_lock_has_one_leader_at_a_time() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    // Advisory locks span the whole server: a key no other test uses.
    let key = i64::from_le_bytes(Uuid::now_v7().as_bytes()[8..].try_into().unwrap());
    let first = OutboxPostgres::new(pool.clone()).con_clave_bloqueo(key);
    let second = OutboxPostgres::new(pool.clone()).con_clave_bloqueo(key);

    assert!(first.intentar_liderar().await.unwrap());
    assert!(first.intentar_liderar().await.unwrap());
    assert!(!second.intentar_liderar().await.unwrap());

    drop(first);
    let mut took_over = false;
    for _ in 0..50 {
        if second.intentar_liderar().await.unwrap() {
            took_over = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(took_over, "the lock was not released with its connection");
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_fan_out_writes_one_delivery_per_consumer_and_marks_the_event_published() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    let deliveries = EntregasPostgres::new(pool.clone())
        .enrutar("notify_new_product", &["product_created"])
        .enrutar("audit", &["product_created"])
        .enrutar("billing", &["order_paid"]);

    publish_all(&pool, &deliveries).await;

    let consumers: Vec<String> =
        sqlx::query_scalar("SELECT consumidor FROM entregas_eventos ORDER BY consumidor")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(consumers, ["audit", "notify_new_product"]);
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM outbox WHERE publicado_en IS NULL"
        )
        .await,
        0
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_failed_fan_out_leaves_the_event_unpublished_and_no_delivery() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    sqlx::raw_sql(
        "ALTER TABLE entregas_eventos ADD CONSTRAINT no_audit CHECK (consumidor <> 'audit')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let deliveries = EntregasPostgres::new(pool.clone())
        .enrutar("notify_new_product", &["product_created"])
        .enrutar("audit", &["product_created"]);
    let event = OutboxPostgres::new(pool.clone())
        .buscar_sin_publicar(1)
        .await
        .unwrap()
        .remove(0);

    let result = deliveries.publicar(&event).await;

    assert!(result.is_err());
    assert_eq!(
        count(&pool, "SELECT count(*) FROM entregas_eventos").await,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM outbox WHERE publicado_en IS NULL"
        )
        .await,
        1
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_event_nobody_subscribed_to_is_still_marked_published() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;

    publish_all(&pool, &EntregasPostgres::new(pool.clone())).await;

    assert_eq!(
        count(&pool, "SELECT count(*) FROM entregas_eventos").await,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM outbox WHERE publicado_en IS NULL"
        )
        .await,
        0
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_claims_come_in_write_order_and_count_as_a_delivery() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let ids = write_events(&pool, &["a", "b", "c"]).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;
    let mut subscription = SuscripcionPostgres::new(pool.clone(), "audit");

    let claimed = subscription.siguiente_lote(10).await.unwrap();
    let again = subscription.siguiente_lote(10).await.unwrap();

    let claimed_ids: Vec<Uuid> = claimed.iter().map(|c| c.sobre.id_agregado).collect();
    assert_eq!(claimed_ids, ids);
    assert!(claimed.iter().all(|c| c.intento == 1));
    assert!(again.is_empty(), "a claim is hidden for its lease");
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_unsettled_claim_is_delivered_again_after_its_lease() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;
    let mut subscription =
        SuscripcionPostgres::new(pool.clone(), "audit").con_reserva(Duration::ZERO);

    let first = subscription.siguiente_lote(10).await.unwrap();
    let second = subscription.siguiente_lote(10).await.unwrap();

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].sobre.id, first[0].sobre.id);
    assert_eq!(second[0].intento, 2);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_concurrent_claims_never_return_the_same_delivery() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let names: Vec<&'static str> = (0..40)
        .map(|n| &*Box::leak(format!("product-{n}").into_boxed_str()))
        .collect();
    write_events(&pool, &names).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;

    let claim_all = |pool: PgPool| async move {
        let mut subscription = SuscripcionPostgres::new(pool, "audit");
        let mut ids = Vec::new();
        loop {
            let batch = subscription.siguiente_lote(3).await.unwrap();
            if batch.is_empty() {
                return ids;
            }
            ids.extend(batch.into_iter().map(|c| c.sobre.id));
        }
    };
    let (a, b, c) = tokio::join!(
        tokio::spawn(claim_all(pool.clone())),
        tokio::spawn(claim_all(pool.clone())),
        tokio::spawn(claim_all(pool.clone())),
    );

    let all: Vec<Uuid> = [a.unwrap(), b.unwrap(), c.unwrap()].concat();
    let unique: HashSet<Uuid> = all.iter().copied().collect();
    assert_eq!(all.len(), 40);
    assert_eq!(unique.len(), 40);
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_ack_retires_the_delivery() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;
    let mut subscription =
        SuscripcionPostgres::new(pool.clone(), "audit").con_reserva(Duration::ZERO);

    let claimed = subscription.siguiente_lote(1).await.unwrap().remove(0);
    subscription
        .resolver(&claimed, &Entrega::Ack)
        .await
        .unwrap();

    assert!(subscription.siguiente_lote(10).await.unwrap().is_empty());
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM entregas_eventos WHERE estado = 'hecho'"
        )
        .await,
        1
    );
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_retry_moves_the_next_attempt_forward_and_counts_the_attempt() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;
    let mut subscription =
        SuscripcionPostgres::new(pool.clone(), "audit").con_reserva(Duration::ZERO);

    let claimed = subscription.siguiente_lote(1).await.unwrap().remove(0);
    let retry = Entrega::Reintentar {
        espera: Duration::from_secs(30),
    };
    subscription.resolver(&claimed, &retry).await.unwrap();

    assert!(subscription.siguiente_lote(10).await.unwrap().is_empty());
    let (attempts, status, wait): (i32, String, f64) = sqlx::query_as(
        "SELECT intentos, estado, EXTRACT(EPOCH FROM siguiente_intento_en - now())::float8 \
         FROM entregas_eventos",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attempts, 1);
    assert_eq!(status, "pendiente");
    assert!(wait > 25.0 && wait <= 30.0, "next attempt in {wait} s");
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_dead_delivery_keeps_one_dead_letter_when_recorded_twice() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    write_events(&pool, &["Desk lamp"]).await;
    publish_all(
        &pool,
        &EntregasPostgres::new(pool.clone()).enrutar("audit", &["product_created"]),
    )
    .await;
    let mut subscription =
        SuscripcionPostgres::new(pool.clone(), "audit").con_reserva(Duration::ZERO);
    let sink = DescartadosPostgres::new(pool.clone());

    let claimed = subscription.siguiente_lote(1).await.unwrap().remove(0);
    let registrar = RegistroDescartado::para_evento(
        "audit",
        &claimed.sobre,
        claimed.intento,
        "payload_invalido",
        "missing field `price`",
    );
    sink.registrar(&registrar).await.unwrap();
    sink.registrar(&registrar).await.unwrap();
    let dead = Entrega::Descartar {
        motivo: "payload_invalido",
        detalle: "missing field `price`".into(),
    };
    subscription.resolver(&claimed, &dead).await.unwrap();

    assert!(subscription.siguiente_lote(10).await.unwrap().is_empty());
    let (status, last_error): (String, String) =
        sqlx::query_as("SELECT estado, ultimo_error FROM entregas_eventos")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "descartado");
    assert_eq!(last_error, "payload_invalido: missing field `price`");
    let (event_id, consumer, event_type, attempts, reason): (Uuid, String, String, i32, String) =
        sqlx::query_as(
            "SELECT id_evento, consumidor, tipo_evento, intentos, motivo FROM descartados",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(event_id, claimed.sobre.id);
    assert_eq!(consumer, "audit");
    assert_eq!(event_type, "product_created");
    assert_eq!(attempts, 1);
    assert_eq!(reason, "payload_invalido");
}

#[tokio::test]
#[ignore = "needs Postgres: make test/db"]
async fn postgres_unreadable_messages_each_keep_a_dead_letter() {
    let Some(pool) = database().await else {
        return eprintln!("{SKIP}");
    };
    let sink = DescartadosPostgres::new(pool.clone());

    for raw in ["not json", "{"] {
        let registrar = RegistroDescartado::ilegible("audit", raw, 1, "payload_invalido", "eof");
        sink.registrar(&registrar).await.unwrap();
    }

    assert_eq!(count(&pool, "SELECT count(*) FROM descartados").await, 2);
}
