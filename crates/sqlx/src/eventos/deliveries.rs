//! The `db` transport: no broker, one `event_deliveries` row per event and subscribed consumer.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use puerto_nucleo::SobreEvento;
use puerto_nucleo::eventos::entrega::Entrega;
use puerto_nucleo::eventos::transporte::{
    ErrorTransporte, PublicadorEventos, Reservado, SuscripcionEventos,
};
use sqlx::PgPool;

use super::{ENVELOPE_COLUMNS, envelope_from_pg, unavailable};

/// How long a claimed delivery stays invisible to other claims. A worker that dies while
/// handling one leaves it due again after this — at-least-once, never lost.
pub const RESERVA_POR_DEFECTO: Duration = Duration::from_secs(60);

/// The relay's publisher on the `db` transport: fans each event out to the consumers subscribed
/// to its type and marks it published, in one transaction.
pub struct EntregasPostgres {
    pool: PgPool,
    /// event type → the consumers subscribed to it, in the order they subscribed.
    routes: BTreeMap<String, Vec<String>>,
}

impl EntregasPostgres {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            routes: BTreeMap::new(),
        }
    }

    /// Subscribes `consumer` to `event_types` — one `[[consumer]]` of puerto.toml.
    pub fn enrutar(mut self, consumer: &str, event_types: &[&str]) -> Self {
        for event_type in event_types {
            let consumers = self.routes.entry((*event_type).to_string()).or_default();
            if !consumers.iter().any(|c| c == consumer) {
                consumers.push(consumer.to_string());
            }
        }
        self
    }

    /// Who receives an event of this type. None is fine: the event is marked published anyway.
    pub fn consumidores_de(&self, event_type: &str) -> &[String] {
        self.routes.get(event_type).map_or(&[], Vec::as_slice)
    }
}

#[async_trait]
impl PublicadorEventos for EntregasPostgres {
    async fn publicar(&self, event: &SobreEvento) -> Result<(), ErrorTransporte> {
        let context = "deliveries: publish";
        let mut tx = self.pool.begin().await.map_err(unavailable(context))?;
        sqlx::query(
            "INSERT INTO entregas_eventos (id_evento, consumidor) \
             SELECT $1, consumidor FROM unnest($2::text[]) AS consumidor \
             ON CONFLICT DO NOTHING",
        )
        .bind(event.id)
        .bind(self.consumidores_de(&event.tipo_evento))
        .execute(&mut *tx)
        .await
        .map_err(unavailable(context))?;
        sqlx::query(
            "UPDATE outbox SET publicado_en = now() WHERE id = $1 AND publicado_en IS NULL",
        )
        .bind(event.id)
        .execute(&mut *tx)
        .await
        .map_err(unavailable(context))?;
        tx.commit().await.map_err(unavailable(context))
    }
}

/// One consumer's input on the `db` transport.
pub struct SuscripcionPostgres {
    pool: PgPool,
    consumer: String,
    reserva: Duration,
}

impl SuscripcionPostgres {
    pub fn new(pool: PgPool, consumer: impl Into<String>) -> Self {
        Self {
            pool,
            consumer: consumer.into(),
            reserva: RESERVA_POR_DEFECTO,
        }
    }

    /// How long a claim is kept from other workers; longer than the slowest handler.
    pub fn con_reserva(mut self, reserva: Duration) -> Self {
        self.reserva = reserva;
        self
    }
}

#[async_trait]
impl SuscripcionEventos for SuscripcionPostgres {
    fn consumidor(&self) -> &str {
        &self.consumer
    }

    /// Claims the due deliveries, oldest event first. A claim counts as a delivery (`attempts`
    /// goes up now, not when it is settled), so a worker that crashes mid-handle still uses one
    /// of the event's deliveries, and the claim is hidden for the lease.
    async fn siguiente_lote(&mut self, max: u32) -> Result<Vec<Reservado>, ErrorTransporte> {
        let context = "deliveries: next_batch";
        let rows = sqlx::query(&format!(
            "WITH due AS ( \
                 SELECT d.id_evento FROM entregas_eventos d \
                 JOIN outbox o ON o.id = d.id_evento \
                 WHERE d.consumidor = $1 AND d.estado = 'pendiente' AND d.siguiente_intento_en <= now() \
                 ORDER BY o.secuencia LIMIT $2 \
                 FOR UPDATE OF d SKIP LOCKED \
             ), claimed AS ( \
                 UPDATE entregas_eventos d \
                 SET intentos = d.intentos + 1, \
                     siguiente_intento_en = now() + make_interval(secs => $3) \
                 FROM due WHERE d.id_evento = due.id_evento AND d.consumidor = $1 \
                 RETURNING d.id_evento, d.intentos \
             ) \
             SELECT {ENVELOPE_COLUMNS}, claimed.intentos FROM claimed \
             JOIN outbox o ON o.id = claimed.id_evento \
             ORDER BY o.secuencia"
        ))
        .bind(&self.consumer)
        .bind(i64::from(max))
        .bind(self.reserva.as_secs_f64())
        .fetch_all(&self.pool)
        .await
        .map_err(unavailable(context))?;
        rows.iter()
            .map(|row| {
                let envelope = envelope_from_pg(row, context)?;
                let attempts: i32 = sqlx::Row::try_get(row, "intentos")
                    .map_err(|_| ErrorTransporte::DatosInvalidos)?;
                let attempt =
                    u32::try_from(attempts).map_err(|_| ErrorTransporte::DatosInvalidos)?;
                Ok(Reservado::new(envelope, attempt))
            })
            .collect()
    }

    async fn resolver(
        &mut self,
        claimed: &Reservado,
        delivery: &Entrega,
    ) -> Result<(), ErrorTransporte> {
        let (id, consumer) = (claimed.sobre.id, &self.consumer);
        let result = match delivery {
            Entrega::Ack => {
                sqlx::query(
                    "UPDATE entregas_eventos SET estado = 'hecho', ultimo_error = NULL \
                     WHERE id_evento = $1 AND consumidor = $2",
                )
                .bind(id)
                .bind(consumer)
                .execute(&self.pool)
                .await
            }
            Entrega::Reintentar { espera } => {
                sqlx::query(
                    "UPDATE entregas_eventos \
                     SET siguiente_intento_en = now() + make_interval(secs => $3) \
                     WHERE id_evento = $1 AND consumidor = $2",
                )
                .bind(id)
                .bind(consumer)
                .bind(espera.as_secs_f64())
                .execute(&self.pool)
                .await
            }
            Entrega::Descartar { motivo, detalle } => {
                sqlx::query(
                    "UPDATE entregas_eventos SET estado = 'descartado', ultimo_error = $3 \
                     WHERE id_evento = $1 AND consumidor = $2",
                )
                .bind(id)
                .bind(consumer)
                .bind(format!("{motivo}: {detalle}"))
                .execute(&self.pool)
                .await
            }
        };
        result.map_err(unavailable("deliveries: settle"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    fn lazy_pool() -> PgPool {
        PgPoolOptions::new()
            .connect_lazy("postgres://puerto@127.0.0.1:1/puerto")
            .unwrap()
    }

    #[tokio::test]
    async fn should_route_an_event_to_every_consumer_subscribed_to_its_type() {
        let deliveries = EntregasPostgres::new(lazy_pool())
            .enrutar(
                "notify_new_product",
                &["product_created", "product_published"],
            )
            .enrutar("audit", &["product_created"]);

        assert_eq!(
            deliveries.consumidores_de("product_created"),
            ["notify_new_product", "audit"]
        );
        assert_eq!(
            deliveries.consumidores_de("product_published"),
            ["notify_new_product"]
        );
    }

    #[tokio::test]
    async fn should_route_to_nobody_when_no_consumer_subscribed() {
        let deliveries = EntregasPostgres::new(lazy_pool()).enrutar("audit", &["product_created"]);

        assert!(deliveries.consumidores_de("product_deleted").is_empty());
    }

    #[tokio::test]
    async fn should_route_once_when_a_consumer_subscribes_twice() {
        let deliveries = EntregasPostgres::new(lazy_pool())
            .enrutar("audit", &["product_created", "product_created"])
            .enrutar("audit", &["product_created"]);

        assert_eq!(deliveries.consumidores_de("product_created"), ["audit"]);
    }

    #[tokio::test]
    async fn should_name_its_consumer_and_default_lease_when_subscribed() {
        let subscription = SuscripcionPostgres::new(lazy_pool(), "audit");

        assert_eq!(subscription.consumidor(), "audit");
        assert_eq!(subscription.reserva, RESERVA_POR_DEFECTO);
        assert_eq!(
            subscription.con_reserva(Duration::from_secs(5)).reserva,
            Duration::from_secs(5)
        );
    }
}
