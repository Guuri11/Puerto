# Architecture Decision Records

One file per decision that is **not visible in the code**: a trade-off, a convention chosen over an
alternative, a measured result. The code says *what*; an ADR says *why*, so a later session — a
person's or an agent's — inherits the reason instead of re-deriving it or reverting it.

Format: `NNNN-title.md` with **Context**, **Decision**, **Consequences**. Never rewrite an accepted
ADR; supersede it with a new one and link both ways.

| # | Decision | Status |
|---|---|---|
| [0001](0001-enum-value-objects-implement-fromstr.md) | Enum value objects implement `FromStr` | Accepted |
| [0002](0002-response-enums-allow-large-enum-variant.md) | Response enums allow `large_enum_variant` | Accepted |
| [0003](0003-unique-indexes-are-partial.md) | Unique indexes are partial (`WHERE deleted = false`) | Accepted |
| [0004](0004-generate-use-case-never-overwrites.md) | `generate use-case` never overwrites and patches `routes.rs` | Accepted |
| [0005](0005-sql-reserved-words-only-in-db-projects.md) | SQL reserved words rejected only in `--db` projects | Accepted |
| [0006](0006-logging-through-tracing-not-a-logger-port.md) | Logging through `tracing`, not a `Logger` port | Accepted |
| [0007](0007-error-contract-code-in-name-422.md) | Error contract: code in `name`, validation is 422 | Accepted |
| [0008](0008-liveness-readiness-graceful-shutdown.md) | Liveness, readiness and graceful shutdown in every project | Accepted |
| [0009](0009-generated-projects-pin-their-toolchain.md) | Generated projects pin their toolchain and use edition 2024 | Accepted |
| [0010](0010-generated-agent-rules-are-tested.md) | Generated agent rules have a root entry point and are tested | Accepted |
| [0011](0011-templates-own-shape-types-own-decisions.md) | Templates own the shape of generated code; `types.rs` owns per-field decisions | Accepted |
| [0012](0012-presentation-holds-use-case-ports.md) | Presentation holds use case ports; routes are tested with mocks | Accepted |
| [0013](0013-web-frontend-react-daisyui-bun.md) | Web frontend: React + daisyUI on Bun; structure by Puerto, wire types from OpenAPI | Accepted |
| [0014](0014-entity-ids-are-generated-newtypes.md) | Entity ids are generated newtypes | Accepted |
| [0015](0015-repositories-return-one-repository-error.md) | Repositories return one `RepositoryError` | Accepted |
| [0016](0016-lists-are-offset-pages.md) | Lists are offset pages, capped in the domain | Accepted |
| [0017](0017-runtime-crates-next-to-the-generator.md) | Runtime crates next to the generator, one per layer | Accepted |
| [0018](0018-domain-events-outbox-pluggable-transport.md) | Domain events: unit of work, outbox, pluggable transport (`db`, `kafka`) | Accepted |
| [0019](0019-generated-code-is-formatted-by-cargo-fmt.md) | Generated code is formatted by `cargo fmt` | Accepted |
