# Cline Memory Bank — Rust E-Commerce Learning Project

> **Purpose:** Track progress across sessions so Cline can pick up where it left off.

## Project Overview
- **Project:** Scalable E-Commerce Platform (microservices in Rust)
- **Goal:** Learn and master Rust through progressive, hands-on development
- **Learner:** Experienced in JavaScript, Go, C/C++; new to Rust
- **Approach:** Teach Rust concepts step-by-step with C++/Go/JS analogies, then implement

## Learning Plan
See `LEARNING_PLAN.md` for the full phased plan.

## Completed Phases

### Phase 0: Rust Bootcamp ✅
- **Commit:** `a37e354`
- **Status:** Complete — all 11 unit tests + 3 doc-tests pass
- **What was built:**
  - Cargo workspace with `common` (library) and `product-service` (binary) crates
  - `common/src/lib.rs`: Tutorial covering ownership, borrowing, structs, enums, `Option<T>`, `Result<T,E>`, traits, generics, iterators, modules — all with C++/Go/JS analogies
  - `common/src/error.rs`: `AppError` enum using `thiserror`, status code mapping for Axum
  - `product-service/src/main.rs`: Placeholder for Phase 1
  - `.gitignore`, workspace `Cargo.toml` with all future dependencies pre-declared
- **Key crates introduced:** `serde`, `thiserror`, `anyhow`, `uuid`, `chrono`, `rust_decimal`, `sqlx`, `axum`, `tokio`
- **Key concepts covered:**
  - Ownership/borrowing vs C++ smart pointers/Go GC
  - Enums as sum types (vs C unions, C++ `std::variant`, Go iota)
  - `Option<T>` vs null/nil
  - `Result<T,E>` vs Go's `(T, error)` and C++ exceptions
  - Traits vs Go interfaces/C++ virtual classes
  - Static dispatch (monomorphization) vs dynamic dispatch (vtable)
  - `thiserror` for library errors, `anyhow` for application errors
  - Iterator combinators as zero-cost abstractions

### Phase 1: Product Catalog Service ✅
- **Commit:** `9f75ad1`
- **Status:** Complete — builds successfully, all common tests pass
- **What was built:**
  - Full CRUD REST API for products: `POST/GET/PUT/DELETE /products`, `GET /health`
  - `config.rs`: Settings loaded from env vars with `config` crate + `#[derive(Deserialize)]`
  - `db.rs`: SQLx `PgPool` with builder pattern (`PgPoolOptions::new().max_connections().connect()`)
  - `models/product.rs`: `Product` struct with `sqlx::FromRow`, DTOs (`CreateProductRequest`, `UpdateProductRequest`), DB query functions using `query_as` and `QueryBuilder`
  - `handlers/products.rs`: Axum handlers with extractors (`State`, `Json`, `Path`, `Query`), `AppState` struct
  - `routes.rs`: Axum Router with method chaining (`.get().post().put().delete()`)
  - `main.rs`: `#[tokio::main]`, tracing init, config loading, pool creation, graceful shutdown
  - Database migration: `products` table with UUID, Postgres enum, timestamps, soft delete, indexes
- **Key concepts covered:**
  - `async/await` + Tokio runtime (cooperative concurrency vs Go goroutines, JS event loop)
  - Axum extractors (type-safe request parsing, no manual JSON/param parsing)
  - SQLx `FromRow` derive and `query_as` (runtime-checked SQL mapping)
  - `Arc<AppState>` (shared state like C++ `shared_ptr`, Go singleton)
  - `impl IntoResponse` for `AppError` (centralized error-to-HTTP mapping, orphan rule)
  - `tracing` with `info!`/`error!` macros (structured logging with spans)
  - `anyhow::Context` for adding context to errors (`.context("...")?`)
  - Orphan rule: traits must be implemented in the crate that defines them OR the type
  - `Default` derive for config structs
  - `serde` annotations (`#[serde(default)]`, `rename_all = "snake_case"`)

## Pending Phases

### Phase 2: User Service (Next)
- Authentication (JWT, Argon2 password hashing)
- New Rust concepts: lifetimes, `String` vs `&str`, `Clone` vs `Copy`, Tower middleware, JWT
- See `LEARNING_PLAN.md` Phase 2 section for details

### Phase 3-10
(Not yet started — see `LEARNING_PLAN.md`)

## Git Log

```
9f75ad1 Phase 1: Product Catalog Service — CRUD API with Axum, SQLx, tracing
67fecde Setup: Cline memory bank (CLINE.md) and teaching rules (.clinerules)
a37e354 Phase 0: Rust Bootcamp
