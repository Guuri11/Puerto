# puerto.toml — Schema & Identifier Derivation

## Schema

```toml
[project]
name = "my-app"          # project name (hyphen-separated, matches Cargo binary name)
web = true               # web/ frontend present — set by `puerto new`, absent with --no-web

[[entity]]
name = "Product"         # PascalCase entity name
use_cases = ["create_product", "list_products"]   # snake_case action names
db = true                # optional — omit for InMemory, set true for SQLx/Postgres

[[entity.fields]]
name = "name"            # snake_case field name
type = "String"          # Rust type from the type registry

[[entity.fields]]
name = "price"
type = "i64"

[[entity.fields]]
name = "sku"
type = "String"
unique = true            # generates unique DB constraint

[[entity.fields]]
name = "description"
type = "Option<String>"  # nullable field

[[entity.fields]]
name = "tags"
type = "Vec<String>"     # array field
```

- One `[[entity]]` block per DDD entity
- `name` is canonical PascalCase — all other identifiers derived from it
- `use_cases` entries are snake_case action names — match the file/module name exactly
- `db = true` → generates `PgEntityRepository` + `entity.rs`; absent/false → `InMemoryEntityRepository`
- `[[entity.fields]]` is an optional list of typed fields. If absent or empty, entities default to `name: String` (backward compatible)
- Field `name` must be valid snake_case (lowercase letters, digits, underscores; cannot start with a digit)
- Field `type` must match the type registry — run `puerto validate` to check
- Field `unique = true` generates a **partial** unique index (`… WHERE deleted = false`, SQL only):
  rows are soft-deleted, so a deleted row does not keep its value taken. Cannot be combined with
  `Option<T>` (warned by `puerto validate`)
- Field names that are PostgreSQL reserved words (`desc`, `order`, `user`, `select`, …) are rejected
  in `--db` projects — they become unquoted column names. Duplicate field names are rejected
- `HashMap<String, String>` is a `JSONB` column; the `{Entity}Db` row field is
  `sqlx::types::Json<HashMap<String, String>>` and the repository's `SELECT` overrides its type
- Managed by Puerto CLI — do not add entities manually unless also running `puerto generate bootstrap`

### CLI Field Syntax

```bash
puerto generate scaffold Product name:String price:i64! sku:String
```

- Fields are passed as trailing arguments in `name:Type` format
- Append `!` after the type to mark the field as `unique = true` (e.g., `sku:String!`)
- Parsed by `parse_field_arg()` in `puerto_toml.rs`
- Validated against the type registry before scaffold proceeds

---

## Type Registry

| Rust Type | SQL Type | OpenAPI | Default (tests) |
|-----------|----------|---------|------------------|
| `String` | `TEXT` | `string` | `"example"` |
| `i64` | `BIGINT` | `integer(int64)` | `42` |
| `bool` | `BOOLEAN` | `boolean` | `true` |
| `f64` | `DOUBLE` | `number(double)` | `1.5` |
| `Uuid` | `UUID` | `string(uuid)` | `Uuid::new_v4()` |
| `DateTime<Utc>` | `TIMESTAMPTZ` | `string(date-time)` | `Utc::now()` |
| `Option<String>` | `TEXT` (nullable) | `string?` | `None` |
| `Option<i64>` | `BIGINT` (nullable) | `integer(int64)?` | `None` |
| `Option<bool>` | `BOOLEAN` (nullable) | `boolean?` | `None` |
| `Option<f64>` | `DOUBLE` (nullable) | `number(double)?` | `None` |
| `Option<DateTime<Utc>>` | `TIMESTAMPTZ` (nullable) | `string(date-time)?` | `None` |
| `Vec<String>` | `TEXT[]` | `array[string]` | `vec![]` |
| `Vec<i64>` | `BIGINT[]` | `array[integer]` | `vec![]` |
| `HashMap<String, String>` | `JSONB` | `object` | `HashMap::new()` |

### System Fields (always present)

Every entity automatically includes these fields — do **not** add them to `entity.fields`:

```rust
pub id: Uuid,
pub created_at: DateTime<Utc>,
pub updated_at: DateTime<Utc>,
pub deleted: bool,
pub deleted_at: Option<DateTime<Utc>>,
```

### Type Resolution

`resolve_type(field_type: &str) -> Result<&TypeMapping, Error>` in `generators/types.rs`:
- Looks up the type string in the registry
- Returns `TypeMapping` with `rust_type`, `sql_type`, `sql_nullable`, `openapi_type`, `openapi_format`, `default_expr`, `needs_import`
- Unknown types produce a descriptive error listing all valid types

`validate_fields(fields: &[Field]) -> Result<(), Vec<String>>` validates all fields at once.

`literal_import(field) -> Option<&'static str>` returns the import a value *literal* of the field
needs (`chrono::Utc` for `Utc::now()`), as opposed to `TypeMapping::needs_import`, which is what
naming the field's *type* needs.

---

## Derivation Table

Given `name = "OrderItem"` and `use_cases = ["create_order_item"]`:

| Identifier        | Value                         | Used in                                  |
| ----------------- | ----------------------------- | ---------------------------------------- |
| `pascal`          | `OrderItem`                   | Struct names, impl names                 |
| `snake`           | `order_item`                  | File names, module names, variable names |
| `plural`          | `order_items`                 | SQL table, REST paths (`/order_items`)    |
| `plural_pascal`   | `OrderItems`                  | Doc comments, `ListOrderItems…` types    |
| `uc`              | `create_order_item`           | Use case field name, file name           |
| `uc_pascal`       | `CreateOrderItem`             | Use case type prefix                     |
| `UseCaseTrait`    | `CreateOrderItemUseCaseTrait` | Domain trait                             |
| `UseCaseImpl`     | `CreateOrderItemUseCaseImpl`  | Application impl struct                  |
| `UseCaseParams`   | `CreateOrderItemParams`       | Input params struct                      |
| `RepositoryTrait` | `OrderItemRepositoryTrait`    | Domain repository trait                  |
| `InMemoryRepo`    | `InMemoryOrderItemRepository` | Infrastructure impl                      |
| `ApiStruct`       | `OrderItemApi`                | Presentation routes struct               |

### Derivation rules

```
pascal  = PascalCase(name)                         // "OrderItem"
snake   = pascal_to_snake(pascal)                  // "order_item"
plural  = pluralize(snake)                         // "order_items"
uc      = use_cases[i]                             // "create_order_item"
uc_pascal = PascalCase(uc)                         // "CreateOrderItem"
```

`PascalCase` splits on `_` and `-`, capitalises each word, joins:

- `"order_item"` → `"OrderItem"`
- `"create-product"` → `"CreateProduct"`

`pluralize` (`generators/naming.rs`) inflects only the last word: consonant + `y` → `ies`
(`categories`), `s`/`x`/`z`/`ch`/`sh` → `es` (`statuses`, `boxes`), a few irregulars (`people`,
`children`) and uncountables (`data`, `series`); otherwise `s`. `naming::crud_use_case` names the scaffolded use cases — `list` after the plural
(`list_order_items`), the rest after the entity (`create_order_item`). `naming::entity_ctx` puts
`pascal`, `snake`, `plural` and `plural_pascal` in every template context.

`pascal_to_snake` inserts `_` before each uppercase letter (except first), lowercases all:

- `"OrderItem"` → `"order_item"`
- `"Product"` → `"product"`

---

## File Paths Derived from Entity

Given `name = "Product"`, `use_cases = ["create_product"]`:

```
business/src/domain/product/model.rs
business/src/domain/product/errors.rs
business/src/domain/product/repository.rs
business/src/domain/product/use_cases/create_product.rs
# use_cases modules declared inline in business/src/lib.rs — no use_cases.rs file

business/src/application/product/create_product.rs

infrastructure/src/product/repository.rs

presentation/src/api/product.rs
presentation/src/api/product/dto.rs
presentation/src/api/product/routes.rs
presentation/src/api/product/responses.rs
presentation/src/api/product/error_mapper.rs
```

---

## bootstrap.rs Generation Logic

`generated/bootstrap.rs` is generated from the full entity list. For each entity:

1. Import use case impls: `use business::application::{snake}::{uc}::{uc_pascal}UseCaseImpl;`
2. Import repo:
   - `db = false` → `use infrastructure::{snake}::repository::InMemory{pascal}Repository;`
   - `db = true` → `use infrastructure::{snake}::repository::Pg{pascal}Repository;`
3. Import API struct: `use crate::api::{snake}::routes::{pascal}Api;`
4. Function signature:
   - **No db entities** → `pub fn build_app() -> Route` (sync)
   - **Any db entity** → `pub async fn build_app() -> Route` (reads `DATABASE_URL`, creates pool internally)
5. Wire in `build_app()`:
   - **Single use case**: inline repo
   - **Multiple use cases**: bind repo once, clone for each — `Arc::clone(&{snake}_repo)`
6. `openapi_service(...)` argument — the one place `OpenApiService::new` is called:
   - **Single entity**: `greeting_api`
   - **Multiple entities**: `(greeting_api, product_api, ...)`
7. **`[project] web = true`** → a `#[cfg(test)]` test `openapi_spec_matches_docs_api_openapi_json`
   builds every `{Entity}Api` from `Mock{Uc}UseCase`s (every use case trait must ship one) and
   compares `openapi_service(...).spec()` with `docs/api/openapi.json`: missing → written,
   `UPDATE_OPENAPI=1` → rewritten, different → fails (`make web/sync`). No server, no database.

---

## CLI Commands That Touch puerto.toml

| Command                                         | Effect                                                                            |
| ----------------------------------------------- | --------------------------------------------------------------------------------- |
| `puerto new [--name] [--db\|--no-db] [--no-demo] [--no-web]` | Creates puerto.toml from template with initial Greeting entity                    |
| `puerto generate scaffold <Name>`               | Upserts the `[[entity]]` block, regenerates bootstrap.rs                          |
| `puerto generate scaffold <Name> -- name:Type`  | Same, with `[[entity.fields]]` from the trailing arguments                        |
| `puerto generate domain <Name> -- name:Type`    | Registers the entity **with its fields** so the later layered commands see them   |
| `puerto generate bootstrap [--allow-missing]`   | Regenerates bootstrap.rs (and `web/…/core/generated/*`); refuses when a declared entity has no code on disk |
| `puerto generate use-case <Entity> <action>`    | Appends action to `use_cases`, adds the field to `{Entity}Api`, regenerates bootstrap.rs. Refuses an action that already exists |
| `puerto generate value-object <Name> <type>`    | Appends a `[[value_object]]` block. Refuses invalid names/types and duplicates    |
| `puerto generate web <Name> [--force]`          | Web projects: the entity's frontend slice + `core/generated/*`. Does not touch puerto.toml; refuses an undeclared or non-CRUD entity |
| `puerto generate migration <name>`              | Creates a migration file (`--db` projects only) — does not touch puerto.toml. The version is unique even within one second |
| `puerto validate`                               | Validates names, types, duplicates, and that every declared entity exists on disk |

`db` is a **project-level** setting (`[project] db`), chosen once by `puerto new --db`. There is
no `--db` flag on `generate scaffold`: it reads the project setting from puerto.toml.
