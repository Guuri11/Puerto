-- Mail (ADR-0020): written by puerto. One row per mail a worker consumer sent for an event, so a
-- redelivered event does not send it again. Growing by one row per mail: pruning old rows is yours.

CREATE TABLE sent_mails (
    event_id UUID        NOT NULL,
    mail     TEXT        NOT NULL,
    sent_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, mail)
);
