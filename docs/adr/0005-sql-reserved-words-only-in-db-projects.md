# 0005 — SQL reserved words are rejected only in `--db` projects

**Status:** Accepted (2026-10-10)

## Context

Field names become unquoted column names in the generated migration and queries. `desc TEXT`
broke `sqlx migrate run`. The options were quoting every identifier, or rejecting the names.

## Decision

Reject PostgreSQL's reserved key words (the "reserved" and "reserved, can be function or type"
categories) as field names — in the CLI and in `puerto validate` — **only when `[project] db` is
true**. Projects without a database keep `order`, `user`, `desc` as field names.

## Consequences

Quoting would have accepted every name but made every hand-written query against these tables
need quotes too; rejecting keeps the SQL plain. A no-db project that later adds a database will be
told by `puerto validate` which fields to rename.
