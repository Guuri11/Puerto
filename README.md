<p align="center">
  <img src="assets/brand/github-social.svg" alt="Puerto — Scaffold. Structure. Ship." width="100%"/>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-2024-orange?logo=rust&logoColor=white" alt="Rust 2024"/>
  <img src="https://img.shields.io/badge/architecture-DDD%20%2B%20Clean%20Arch-6366f1" alt="DDD + Clean Architecture"/>
  <img src="https://img.shields.io/badge/api-poem--openapi-0891b2" alt="poem-openapi"/>
  <img src="https://img.shields.io/badge/async-tokio-0891b2?logo=tokio" alt="tokio"/>
</p>

---

A Rust full-stack framework that brings the **delightful developer experience** of Laravel and Ruby on Rails to a **Domain-Driven Design** architecture.

---

## Core Principles

### 1. Delightful Coding Experience

Getting started should take seconds, not hours. `puerto new my-app` gives you a production-ready workspace with the right structure, dependencies, and conventions already in place. No boilerplate, no decision fatigue.

### 2. Truly AI-Ready

Puerto is built for the age of AI-assisted development. Rust's strict compiler acts as an **instant feedback loop** — when an AI agent generates wrong code, the compiler catches it with a precise, actionable error. No silent failures, no runtime surprises. The stronger the types, the smarter the agent.

### 3. Convention over Configuration

Puerto makes the right choices for you. Directory layout, error patterns, dependency injection, testing conventions — all standardized. When every Puerto project looks the same, AI agents and human developers can navigate any codebase from day one.

---

## Why DDD Instead of MVC

MVC is a great starting point, but it doesn't scale well. As applications grow, business logic leaks into controllers and models, making code harder to test and reason about.

Puerto generates projects around **Domain-Driven Design** and **Clean Architecture**:

- **Domain** is pure Rust — no framework dependencies, no infrastructure concerns. Business rules live here.
- **Application** orchestrates the domain — use cases are explicit, testable units of behavior.
- **Infrastructure** adapts external systems (databases, HTTP clients) to domain contracts.
- **Presentation** exposes the application via HTTP — it's just another adapter.

The result: a codebase where business logic is isolated, always testable, and never coupled to the framework.

---

## Architecture

Every Puerto project is a Cargo workspace with three crates:

```
my-app/
├── business/          # Pure Rust — domain models, errors, use cases
│   └── src/
│       ├── domain/    # Entities, repository traits, use case traits
│       └── application/ # Use case implementations
├── infrastructure/    # Adapters — repositories, HTTP clients, etc.
│   └── src/
└── presentation/      # HTTP API — poem-openapi routes, DTOs, error mapping
    └── src/
```

**Dependency rule (inward only):**

```
Presentation → Infrastructure → Application → Domain
```

The domain depends on nothing. Everything else depends on the domain.

---

## Getting Started

```bash
# Install Puerto
cargo install puerto

# Scaffold a new project
puerto new my-app

# Enter the project and run
cd my-app
cargo run
```

Visit `http://localhost:8080` for the Swagger UI, or call the API directly:

```bash
curl http://localhost:8080/api/greetings/World
```

### Entity Fields

Scaffold entities with typed fields — the type system flows from `puerto.toml` through all layers:

```bash
puerto generate scaffold Product -- name:String price:i64! sku:String
```

This creates a `Product` entity with custom fields in `puerto.toml`, and generates typed structs in every DDD layer (domain model, DTOs, repository rows, SQL migrations). Supported types include `String`, `i64`, `bool`, `f64`, `Uuid`, `DateTime<Utc>`, `Option<T>`, `Vec<T>`, and `HashMap<String, String>`. Append `!` to mark a field as unique (e.g., `sku:String!`).

### Value Objects

Wrap primitive fields in strongly-typed Value Objects — generated across all layers automatically:

```bash
# Regular VO
puerto generate scaffold Order -- amount:Amount:f64 status:Status:enum:Pending/Confirmed/Cancelled

# Nullable / array VOs
puerto generate scaffold User -- email:Email:String nickname:Nick:opt:String tags:Tag:vec:String

# Shared VOs — declared once, reusable across entities
puerto generate value-object Email String
puerto generate scaffold User -- email:Email   # type inferred from the shared VO declaration
```

Each VO generates a domain struct with private inner value, `new()` returning `Result`, and `value()` / `as_str()` accessors. Enum VOs generate `from_str()` / `as_str()` pattern. Infrastructure and presentation layers use primitives; VOs are constructed and extracted at the boundary automatically.

Validate your `puerto.toml` at any time:

```bash
puerto validate
```

Visit `http://localhost:8080` for the Swagger UI, or call the API directly:

```bash
curl http://localhost:8080/api/greetings/World
```

---

## Project Structure (Puerto itself)

Puerto is also a Cargo workspace:

```
puerto/
└── crates/
    └── cli/                # The puerto binary (cargo-generate wrapper)
        ├── src/            # CLI source
        └── template/       # The "basic" project template (edit directly here)
```

| Command          | Description                                                |
| ---------------- | ---------------------------------------------------------- |
| `make run`       | Run the puerto CLI                                         |
| `make test`      | Fast structural tests                                      |
| `make test/full` | Full test: generates a project + compiles + runs its tests |
| `make lint`      | Clippy with `-D warnings`                                  |
| `make format`    | rustfmt                                                    |
| `make check`     | cargo check                                                |
