# Infrastructure Layer — AI Agent Notes

This crate contains **adapters** — implementations of the ports defined in the domain. No business logic lives here.

## Layer Structure

```
infrastructure/src/
└── <entity>/
    ├── repository.rs    # Implements domain repository trait
    └── mod.rs
```

## Critical Rules

- **No business logic** — repositories only translate between domain objects and external storage.
- **`from_repository()` is the only door** from persistence into the domain — never call `Entity::new()` inside a repository.
- **`InMemoryRepository` is the default** — replace with a real adapter (SQLx, HTTP, etc.) when external storage is added.
- Repositories depend on `business` only. Never import from `presentation`.

## In-Memory Repository Pattern

```rust
pub struct InMemoryEntityRepository;

#[async_trait]
impl EntityRepositoryTrait for InMemoryEntityRepository {
    async fn find_by_id(&self, _id: Uuid) -> Result<Option<Entity>, EntityError> {
        Ok(None)  // Stub — replace with real storage
    }
}
```

## SQLx Migration Path (when adding DB)

When replacing the in-memory adapter with SQLx:

1. Add `sqlx` to `infrastructure/Cargo.toml`
2. Create `infrastructure/src/<entity>/entity.rs` — flat `#[derive(sqlx::FromRow)]` struct
3. Implement conversions: `TryFrom<EntityDb> for Entity` + `From<&Entity> for EntityDb`
4. Replace `InMemoryRepository` with the SQLx implementation
5. Run migrations: `sqlx migrate run`

## Adding a New Entity Adapter

Run `harbor generate repository <Name>` to scaffold this layer in isolation. It reads `project.db` from `harbor.toml` to decide InMemory vs Pg — no flag needed.

Manual steps if not using the CLI:
1. Create `infrastructure/src/<entity>/repository.rs` implementing the domain trait
2. Add the entity module to `infrastructure/src/lib.rs`
