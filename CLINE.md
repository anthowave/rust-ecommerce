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

### Phase 2: User Service — Authentication & Authorization ✅
- **Commits:** `628c142`, `0b79a5c`, `673c041`, `4bbd759`
- **Status:** Complete — builds successfully, 20 tests pass (6 user-service + 11 common + 3 product-service)
- **What was built:**
  - **Database Migration:** `users` table with `password_hash` (Argon2), `user_role` Postgres enum, soft delete; `refresh_tokens` table with FK to users, token hash storage
  - **Crate Scaffold:** `Cargo.toml` with argon2, jsonwebtoken, sha2, validator, redis; `main.rs` with `AppState`, `shutdown_signal`; `config.rs` with JWT settings; `db.rs` with pool builder
  - **Models:** `User` struct with `#[serde(skip)]` on `password_hash`; `UserRole` enum with `sqlx::Type` for Postgres enum mapping; DTOs (`UserResponse`, `CreateUserRequest`, `LoginRequest`, `UpdateUserRequest`, `AuthResponse`, `RefreshTokenRequest`); DB query functions (`create_user`, `find_user_by_email`, `find_user_by_id`, `update_user` with `QueryBuilder`, `soft_delete_user`, `store_refresh_token`, `find_refresh_token`, `revoke_user_tokens`)
  - **Auth Module:** Argon2 password hashing (`hash_password`, `verify_password`); JWT encode/decode (`create_access_token`, `create_refresh_token`, `validate_token`); SHA-256 token hashing for refresh token storage
  - **Auth Middleware:** Tower Layer via `axum::middleware::from_fn_with_state`; extracts `Authorization: Bearer <token>` header; validates JWT; injects `AuthUser` into request extensions (type-safe!)
  - **Handlers:** `register` (POST /auth/register), `login` (POST /auth/login), `refresh` (POST /auth/refresh with token rotation), `logout` (POST /auth/logout), `get_me` (GET /users/me), `update_me` (PUT /users/me), `get_user` (GET /users/:id)
  - **Routes:** Public (register, login, refresh, get_user, health) + Protected (logout, get_me, update_me) with JWT middleware; `TraceLayer` for request logging
- **API Endpoints:**
  | Method | Path | Auth | Description |
  |--------|------|------|-------------|
  | POST | /auth/register | No | Register new user |
  | POST | /auth/login | No | Login, returns JWT tokens |
  | POST | /auth/refresh | No | Refresh access token |
  | POST | /auth/logout | Yes | Invalidate tokens |
  | GET | /users/me | Yes | Get current user profile |
  | PUT | /users/me | Yes | Update profile |
  | GET | /users/:id | No | Get public user profile |
  | GET | /health | No | Health check |
- **Rust Concepts Taught:**
  - **Lifetimes:** Claims struct uses owned `String` (not `&str`) — why lifetimes exist and when to avoid them
  - **`String` vs `&str`:** Config stores `String` (owns data), functions accept `&str` (borrows); hands-on ownership lesson from the ownership error in main.rs
  - **`Clone` vs `Copy`:** `Settings` derives `Clone` but NOT `Copy` — heap data can't be implicitly copied; `Arc::clone()` is cheap (atomic counter bump)
  - **`impl Into<String>`:** `create_access_token()` accepts both `&str` and `String` via trait bounds
  - **Tower Middleware (Layers):** `from_fn_with_state` creates a Tower Layer from an async function; route-layered auth; type-safe request extensions for `AuthUser`
  - **`async-trait`:** Not needed — all our traits were simple enough. Noted as a future concept.
  - **Argon2:** Modern password hashing (memory-hard, GPU/ASIC resistant); salt auto-embedded in PHC format hash string
  - **JWT:** `jsonwebtoken` crate — encode with `EncodingKey`, decode with `DecodingKey`, `Validation::default()`
  - **Refresh token rotation:** Old token revoked on refresh; token hash stored in DB (not raw token)
  - **Validator crate:** Declarative input validation with `#[validate(email)]`, `#[validate(length(min = 8))]`
  - **Postgres enum mapping:** `#[derive(sqlx::Type)]` + `#[sqlx(type_name = "user_role")]`
  - **`From<T>` trait:** `impl From<User> for UserResponse` for clean `.into()` conversions
  - **Request Extensions:** Type-safe per-request storage — middleware inserts `AuthUser`, handlers extract it
  - **`#[serde(skip)]`:** Defense-in-depth to prevent password hash leakage
  - **Trait scope:** `Validate::validate()` requires `use validator::Validate` — unlike Go, trait methods only available when trait is in scope
- **Key bug/lesson:** `Arc` move-after-borrow error — Rust's ownership system caught us trying to use `state` after moving it into `create_router()`. Fix: `state.clone()` (cheap Arc refcount bump).

## Pending Phases

### Phase 3: Shopping Cart Service ✅
- **Commit:** `9b52bba`
- **Status:** Complete — builds successfully, 31 tests pass workspace-wide (11 cart-service + 11 common + 3 product-service + 6 user-service)
- **What was built:**
  - In-memory cart store using `Arc<RwLock<HashMap<Uuid, Cart>>>` — NO database, pure memory
  - Full CRUD API: GET /cart, POST /cart/items, PUT /cart/items/:product_id, DELETE /cart/items/:product_id, DELETE /cart, GET /health
  - JWT auth middleware (reused pattern from user-service) — all cart routes require valid JWT
  - CartItem model: product_id, name (denormalized), unit_price, quantity, added_at
  - Cart model: user_id, items, updated_at, total calculation
  - CartStore methods: get_cart, get_or_create_cart, add_item (with quantity increment), update_quantity (remove on 0), remove_item, clear_cart
  - Response DTOs: CartResponse, CartItemResponse with camelCase JSON serialization
  - Service-specific CartError enum with IntoResponse (CartNotFound, ItemNotFound, InsufficientStock, ValidationError, Unauthorized, Internal)
- **API Endpoints:**
  | Method | Path | Auth | Description |
  |--------|------|------|-------------|
  | GET | /cart | Yes | Get current user's cart (auto-creates if empty) |
  | POST | /cart/items | Yes | Add item to cart (increments if exists) |
  | PUT | /cart/items/:product_id | Yes | Update item quantity (0 = remove) |
  | DELETE | /cart/items/:product_id | Yes | Remove item from cart |
  | DELETE | /cart | Yes | Clear entire cart |
  | GET | /health | No | Health check |
- **Key concepts covered:**
  - **Interior Mutability:** `RwLock<T>` lets you mutate through `&T` — borrow check moves to runtime
  - **`Mutex<T>` owns data:** Unlike Go (separate mutex + data), Rust's `RwLock<T>` WRAPS the data — you CANNOT access T without locking. Compiler enforced.
  - **`tokio::sync::RwLock` vs `std::sync::RwLock`:** Tokio's version is async-aware — `.read().await` yields instead of blocking the OS thread. Critical distinction for async code.
  - **RAII Lock Guards:** `.read().await` returns `RwLockReadGuard` — auto-unlocks when guard drops. No `defer mutex.RUnlock()` needed (unlike Go).
  - **`Arc::clone()` is cheap:** Only bumps an atomic counter — doesn't copy the HashMap. This is why passing `CartStore` to Axum State works.
  - **Read-then-write pattern:** `get_or_create_cart` uses read lock first (fast path), write lock only if create needed (slow path). Minimizes contention.
  - **`#[tokio::test]`:** Async tests need Tokio runtime — `#[tokio::test]` provides it (unlike `#[test]` for sync tests)
  - **Arc move-after-borrow bug (again!):** Same ownership lesson from Phase 2 — `state` moved into `create_router()` then used after move. Fix: `state.clone()` (cheap Arc refcount bump)

### Phase 4-10
(Not yet started — see `LEARNING_PLAN.md`)

## Git Log

```
9b52bba Phase 3: Shopping Cart Service — Interior Mutability & Concurrency (Arc<RwLock<HashMap>>)
4bbd759 Phase 2 (Steps 5-7): Auth middleware (Tower Layer), handlers (register/login/refresh/logout/get_me/update_me/get_user), routes (public + protected with JWT middleware) — complete User Service
673c041 Phase 2 (Step 4): Auth module — Argon2 password hashing, JWT encode/decode/validate, SHA-256 token hashing, 5 unit tests
0b79a5c Phase 2 (Step 3): User model with DB queries — UserRole enum, DTOs, QueryBuilder, refresh token ops
628c142 Phase 2 (Step 1-2): User Service scaffold — migration, config, db, error, models, middleware stubs, handlers stubs, routes stub
9f75ad1 Phase 1: Product Catalog Service — CRUD API with Axum, SQLx, tracing
67fecde Setup: Cline memory bank (CLINE.md) and teaching rules (.clinerules)
a37e354 Phase 0: Rust Bootcamp