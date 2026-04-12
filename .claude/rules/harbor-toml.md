# harbor.toml — Schema & Identifier Derivation

## Schema

```toml
[project]
name = "my-app"          # project name (hyphen-separated, matches Cargo binary name)

[[entity]]
name = "Product"         # PascalCase entity name
use_cases = ["create_product", "delete_product"]  # snake_case action names
db = true                # optional — omit for InMemory, set true for SQLx/Postgres
```

- One `[[entity]]` block per DDD entity
- `name` is canonical PascalCase — all other identifiers derived from it
- `use_cases` entries are snake_case action names — match the file/module name exactly
- `db = true` → generates `PgEntityRepository` + `entity.rs`; absent/false → `InMemoryEntityRepository`
- Managed by Harbor CLI — do not add entities manually unless also running `harbor generate bootstrap`

---

## Derivation Table

Given `name = "OrderItem"` and `use_cases = ["create_order_item"]`:

| Identifier | Value | Used in |
|------------|-------|---------|
| `pascal` | `OrderItem` | Struct names, impl names |
| `snake` | `order_item` | File names, module names, variable names |
| `uc` | `create_order_item` | Use case field name, file name |
| `uc_pascal` | `CreateOrderItem` | Use case type prefix |
| `UseCaseTrait` | `CreateOrderItemUseCaseTrait` | Domain trait |
| `UseCaseImpl` | `CreateOrderItemUseCaseImpl` | Application impl struct |
| `UseCaseParams` | `CreateOrderItemParams` | Input params struct |
| `RepositoryTrait` | `OrderItemRepositoryTrait` | Domain repository trait |
| `InMemoryRepo` | `InMemoryOrderItemRepository` | Infrastructure impl |
| `ApiStruct` | `OrderItemApi` | Presentation routes struct |

### Derivation rules

```
pascal  = PascalCase(name)                         // "OrderItem"
snake   = pascal_to_snake(pascal)                  // "order_item"
uc      = use_cases[i]                             // "create_order_item"
uc_pascal = PascalCase(uc)                         // "CreateOrderItem"
```

`PascalCase` splits on `_` and `-`, capitalises each word, joins:
- `"order_item"` → `"OrderItem"`
- `"create-product"` → `"CreateProduct"`

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
   - `db = true`  → `use infrastructure::{snake}::repository::Pg{pascal}Repository;`
3. Import API struct: `use crate::api::{snake}::routes::{pascal}Api;`
4. Function signature:
   - **No db entities** → `pub fn build_app() -> Route` (sync)
   - **Any db entity** → `pub async fn build_app() -> Route` (reads `DATABASE_URL`, creates pool internally)
5. Wire in `build_app()`:
   - **Single use case**: inline repo
   - **Multiple use cases**: bind repo once, clone for each — `Arc::clone(&{snake}_repo)`
6. `OpenApiService::new(...)` argument:
   - **Single entity**: `greeting_api`
   - **Multiple entities**: `(greeting_api, product_api, ...)`

---

## CLI Commands That Touch harbor.toml

| Command | Effect |
|---------|--------|
| `harbor new [--name] [--db]` | Creates harbor.toml from template with initial Greeting entity |
| `harbor generate scaffold <Name>` | Appends `[[entity]]` block (`db` omitted), regenerates bootstrap.rs |
| `harbor generate scaffold <Name> --db` | Appends `[[entity]]` block with `db = true`, regenerates bootstrap.rs |
| `harbor generate bootstrap` | Reads harbor.toml, regenerates bootstrap.rs (no other changes) |
| `harbor generate use-case <Entity> <action>` | Appends action to entity's `use_cases`, regenerates bootstrap.rs |
| `harbor generate migration <name>` | Creates migration file — does not touch harbor.toml |
