# 0016 — Lists are offset pages, capped in the domain

**Status:** Accepted (2026-10-11).

## Context

`find_all()` returned every non-deleted row and `GET /products` serialised them all: one large
table made the endpoint, the database and the browser slow at once. `document-api` has a
`PageRequest` value object that clamps the limit and a `PaginatedResult<T>` (backlog T-27).

## Decision

- `business/src/domain/shared/pagination.rs`: `PageRequest` (private `limit` / `offset`, both
  `u32`) built only through `PageRequest::new(Option<u32>, Option<u32>)`, which applies
  `DEFAULT_PAGE_LIMIT = 20` and clamps the limit to `1..=MAX_PAGE_LIMIT` (100). The cap is a domain
  rule: no route, use case or adapter can ask for more. `Page<T> { items, total, limit, offset }`.
- **Offset, not cursor.** A generated CRUD screen wants "page 3 of 7" and a total; offset gives
  both with one `LIMIT/OFFSET` and one `COUNT(*)`, and works with any order. Cursor pagination
  is cheaper on deep pages of very large tables and stable under concurrent inserts — when a list
  needs it, it is a hand-written use case with its own port method.
- `u32`, not `u64`/`i64`: the values reach SQL as `i64::from(u32)` with no cast, poem validates
  the query as non-negative, and four billion rows of offset is beyond what offset paging serves.
  `total` is `u64`.
- Order is `created_at DESC, id` — `id` breaks ties, so pages do not overlap when two rows share a
  timestamp.
- The HTTP shape is `GET /products?limit=&offset=` → `{ items, total, limit, offset }`, `limit`
  being the one actually applied. `web/` gets `Page<T>` in `src/domain/shared/page.ts`, the list
  screen keeps the offset and a static `Pagination` component moves it.

## Consequences

- Every list costs two queries. The count is over the same `WHERE deleted = false`; for very large
  tables an estimate would be cheaper, and that is a hand-written change.
- `limit` above the cap is clamped silently, not rejected: the response's `limit` tells the client
  what it got.
- Filtering and sorting are not generated; they are use-case parameters to add by hand.
