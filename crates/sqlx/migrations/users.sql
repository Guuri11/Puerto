-- Every authenticated caller (ADR-0021 § 3 of Puerto). `(issuer, subject)` is the provider's
-- name for the person; in an OIDC project `roles` is not read — the token's roles decide.
CREATE TABLE users (
    id UUID PRIMARY KEY,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT,
    name TEXT,
    roles TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE (issuer, subject)
);
