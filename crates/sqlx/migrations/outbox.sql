-- Domain events (ADR-0018): written by puerto. The units of work insert into `outbox` in the same
-- transaction as the entity; the worker relays it and records what it could not deliver.

CREATE TABLE outbox (
    id             UUID        PRIMARY KEY,
    seq            BIGSERIAL   NOT NULL,          -- relay order
    aggregate_type TEXT        NOT NULL,
    aggregate_id   UUID        NOT NULL,
    event_type     TEXT        NOT NULL,
    schema_version INT         NOT NULL,
    payload        JSONB       NOT NULL,
    headers        JSONB       NOT NULL DEFAULT '{}',
    occurred_at    TIMESTAMPTZ NOT NULL,
    published_at   TIMESTAMPTZ
);
CREATE INDEX outbox_unpublished ON outbox (seq) WHERE published_at IS NULL;

CREATE TABLE event_deliveries (                  -- db transport only
    event_id        UUID        NOT NULL REFERENCES outbox (id),
    consumer        TEXT        NOT NULL,
    status          TEXT        NOT NULL DEFAULT 'pending',   -- pending | done | dead
    attempts        INT         NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error      TEXT,
    PRIMARY KEY (event_id, consumer)
);
CREATE INDEX event_deliveries_due ON event_deliveries (consumer, next_attempt_at) WHERE status = 'pending';

CREATE TABLE dead_letters (                      -- every transport
    id           UUID        PRIMARY KEY,
    event_id     UUID,
    consumer     TEXT        NOT NULL,
    event_type   TEXT,
    attempts     INT         NOT NULL,
    reason       TEXT        NOT NULL,
    detail       TEXT        NOT NULL,
    payload      TEXT        NOT NULL,           -- trimmed to 4000 chars
    recorded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- One dead letter per event and consumer, so a redelivered failure is not recorded twice. A
-- message that was not an envelope has no event_id: NULLs never conflict.
CREATE UNIQUE INDEX dead_letters_event_consumer ON dead_letters (event_id, consumer);
