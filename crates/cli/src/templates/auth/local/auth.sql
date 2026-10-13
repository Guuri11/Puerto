-- Local accounts (ADR-0021 § 2): written by puerto, after `users`. The password hash lives apart
-- from the user, so `users` carries no secret. A session row keeps the SHA-256 of its token, never
-- the token: a copy of this table opens no session. Both go with their user. Expired sessions are
-- refused, not deleted: pruning them (`DELETE FROM sessions WHERE expires_at < now()`) is yours.

CREATE TABLE credentials (
    user_id       UUID        PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    password_hash TEXT        NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    token_hash BYTEA       PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);
