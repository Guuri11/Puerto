# 0008 — Liveness, readiness and graceful shutdown in every project

**Status:** Accepted (2026-10-10)

## Context

Generated projects had no health endpoint and stopped on SIGTERM by dropping in-flight requests.
`document-api` learned the hard way that a liveness-only probe reports a stack whose database is
down as healthy.

## Decision

- `GET /health` — liveness: `200 ok` while the process answers.
- `GET /health/ready` — readiness: `200 ready`, or `503 not ready` when a dependency does not
  answer. In `--db` projects it runs `SELECT 1` (`infrastructure::db::ping`); without a database
  it is always ready.
- Both are plain poem routes outside the OpenAPI service, wired in `bootstrap.rs`.
- `main.rs` stops on Ctrl-C or SIGTERM with `run_with_graceful_shutdown` and an 8-second deadline,
  below `docker stop`'s default 10.

## Consequences

Orchestrators get the two probes they expect. Readiness is bounded by the pool's acquire timeout
(30 s) when the database hangs rather than refuses — slow to answer, but it does answer.
