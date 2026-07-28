# Rust E-Commerce Microservices — Progressive Learning Plan

## Target Audience
Experienced software engineer with deep expertise in JavaScript, Go, and C/C++, new to Rust. This plan leverages existing knowledge of systems programming (C/C++), concurrency models (Go), and web development (JavaScript) to accelerate Rust mastery.

## Learning Philosophy
- **Concept-first, code-second**: Each phase introduces Rust concepts with explicit analogies to C++, Go, and JavaScript patterns you already know.
- **Progressive complexity**: Services are ordered so each one builds on concepts from the previous.
- **Explain the "why"**: Every design decision includes the Rust rationale (ownership, zero-cost abstractions, compile-time safety).
- **Compare and contrast**: Rust idioms are compared to their equivalents in languages you know.

---

## Phase 0: Rust Bootcamp (Days 1-2)

### Goal
Get comfortable with Rust's unique concepts before diving into services.

### Topics & Exercises

| # | Topic | C/C++/Go Analogy | Exercise |
|---|-------|-----------------|----------|
| 0.1 | Cargo, project structure, `cargo build/run/test`, workspaces | Similar to Go modules (go mod) or CMake, but integrated into the toolchain | `cargo new rust_ecommerce --workspace` |
| 0.2 | Ownership, borrowing, references (`&T`, `&mut T`), slices | C++ move semantics, `std::unique_ptr`, `std::shared_ptr`, `const&` vs `&`. Rust's borrow checker is like having a static analyzer that proves memory safety at compile time — no use-after-free, no double-free, no data races | Small CLI that reads/writes files, experimenting with moves and borrows |
| 0.3 | `struct`, `impl` blocks, `enum`, pattern matching with `match`, `Option<T>`, `Result<T, E>` | `struct` ≈ C struct with methods. `enum` ≈ C++ `std::variant` but first-class. `Option<T>` ≈ `std::optional<T>`. `Result<T,E>` ≈ Go's `(T, error)` return pattern but enforced by the type system | Define a `Product` struct with validation methods returning `Result` |
| 0.4 | Traits — defining shared behavior | Traits ≈ Go interfaces (duck typing) ≈ C++ abstract base classes (vtable). Key difference: traits can be implemented for types you don't own (orphan rule permitting) | Implement `std::fmt::Display` and `std::str::FromStr` for your types |
| 0.5 | Generics & trait bounds | Like C++ templates but with trait-based constraints. Monomorphization at compile time (like C++ templates) but with clear error messages. `where T: Trait` syntax | A generic `Repository<T>` trait bounded by `Serialize + Deserialize` |
| 0.6 | Error handling — `thiserror` (library errors) vs `anyhow` (application errors) | `thiserror` ≈ defining custom error types. `anyhow` ≈ Go's `fmt.Errorf("...: %w", err)` — easy error propagation with context. The `?` operator ≈ Go's `if err != nil { return err }` but implicit | Custom error types with `thiserror` derive macro |
| 0.7 | `Vec<T>`, `HashMap<K,V>`, `HashSet<T>`, iterators, closures | Iterators ≈ C++ ranges (C++20) ≈ JavaScript array methods (map, filter, collect). Closures ≈ C++ lambdas. Zero-cost abstraction: iterator chains compile to the same assembly as hand-written loops | Transform a collection of products using iterator combinators |
| 0.8 | Modules (`mod`, `use`, `pub`), visibility, crate structure | Modules ≈ C++ namespaces but with filesystem-aware privacy. `pub` ≈ `export` in JavaScript. Privacy is module-level by default (vs class-level in C++) | Organize code into `models`, `handlers`, `repository` modules |

### Key Crates to Explore
- `serde` / `serde_json` — Serialization (≈ Go's `encoding/json`, but derive-based)
- `thiserror` / `anyhow` — Error handling
- `tracing` / `tracing-subscriber` — Structured logging (≈ Go's `log/slog`)
- `tokio` — Async runtime (≈ Go's goroutine scheduler, but explicit)

---

## Phase 1: Product Catalog Service — "Rust Fundamentals in Practice"

### Why This Service First
- Pure CRUD — no auth, no inter-service dependencies
- Teaches the core web stack: HTTP → handlers → business logic → database
- Introduces async Rust, SQLx, serialization, and testing

### Rust Concepts Covered

| Concept | Explanation | C/C++/Go Analogy |
|---------|-------------|-----------------|
| **Async/await** | `async fn` returns a `Future` (a state machine). `.await` yields to the Tokio runtime. Unlike Go where goroutines are preemptively scheduled, Rust async is cooperative — you must `.await` to yield. Tokio uses a work-stealing thread pool, similar to Go's M:N scheduler (M goroutines on N OS threads) | Go: `go func()` creates a goroutine. Rust: `tokio::spawn(async { ... })`. Go channels ≈ `tokio::sync::mpsc`. Go `select` ≈ `tokio::select!` |
| **Axum extractors** | Type-safe request deserialization. `Json<T>`, `Query<T>`, `Path<T>`, `State<T>` are all "extractors" — they implement `FromRequest` and are resolved at compile time. If an extractor fails, Axum returns 400/500 automatically with zero boilerplate | Go frameworks use reflection or manual parsing. Axum's approach is more like dependency injection — declare what you need, Axum provides it. |
| **SQLx** | Compile-time SQL checking! `sqlx::query!("SELECT * FROM products WHERE id = $1")` is validated against your actual database at compile time. No runtime SQL typos. `sqlx::FromRow` derives row mapping automatically | Go's `database/sql` + `sqlx` (jmoiron) — but Go can't do compile-time SQL checking. C++ ORMs like ODB require code generation. |
| **Serde** | `#[derive(Serialize, Deserialize)]` — the macro generates all serialization code at compile time. No reflection, no runtime type inspection. Compares to Go's struct tags (`json:"name"`) but Go requires runtime reflection | C++: nlohmann/json or Boost.Serialization. Go: `encoding/json` with struct tags. Rust's approach is fully compile-time. |
| **`Arc<T>`** vs `Rc<T>` | `Arc` = Atomic Reference Counted (thread-safe). `Rc` = Reference Counted (single-threaded). You'll use `Arc` for shared application state across handlers. `Arc<T>` ≈ `std::shared_ptr<T>` in C++. Rust makes the thread-safety choice explicit in the type system | C++: `shared_ptr` uses atomic refcounts always (performance cost). Rust lets you choose `Rc` (fast, single-threaded) vs `Arc` (atomic, multi-threaded) |

### API Endpoints to Build
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/products` | Create a product |
| `GET` | `/products` | List products (with pagination, filtering) |
| `GET` | `/products/:id` | Get a single product |
| `PUT` | `/products/:id` | Update a product |
| `DELETE` | `/products/:id` | Soft-delete a product |
| `GET` | `/products/search?q=...` | Search products |

### Project Structure
```
crates/product-service/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, server setup
│   ├── config.rs         # Configuration loading (environment, files)
│   ├── db.rs             # Database connection pool setup
│   ├── error.rs          # Custom error types (thiserror)
│   ├── models/
│   │   ├── mod.rs
│   │   └── product.rs    # Product struct, DB queries
│   ├── handlers/
│   │   ├── mod.rs
│   │   └── products.rs   # Axum handler functions
│   └── routes.rs         # Router definition
├── migrations/           # SQLx migration files
└── tests/
    └── integration.rs    # Integration tests
```

### Key Learnings Checklist
- [ ] Understand Tokio's async runtime: how it differs from Go's goroutine scheduler
- [ ] Understand Axum's extractor pattern and how it eliminates boilerplate
- [ ] Understand SQLx compile-time query checking — write a type-wrong query and read the compile error
- [ ] Understand `Arc<AppState>` for shared state across handlers
- [ ] Understand Serde's derive macros and how they compare to Go struct tags
- [ ] Write unit tests and integration tests with `cargo test`
- [ ] Understand `tracing` spans and structured logging
- [ ] Understand `impl IntoResponse` and Axum's type-safe response model

---

## Phase 2: User Service — "Ownership, Lifetimes & Security"

### Why This Service Second
- Introduces authentication (JWT), which all subsequent services will use
- Deepens ownership understanding through password hashing, token management
- First taste of middleware (reusable across all services)

### Rust Concepts Covered

| Concept | Explanation | Analogy |
|---------|-------------|---------|
| **Lifetimes** | `&'a T` — the lifetime `'a` tells the compiler how long a reference is valid. Most of the time, Rust infers lifetimes (lifetime elision). You'll explicitly annotate lifetimes when the compiler can't infer them — typically in structs that hold references. This is how Rust achieves memory safety without garbage collection. | C++: dangling references are UB (undefined behavior). Go: GC prevents dangling pointers but adds overhead. Rust: compiler proves no dangling references at compile time — zero cost. |
| **`String` vs `&str`** | `String` = owned, heap-allocated, mutable string (≈ `std::string`). `&str` = borrowed string slice (≈ `std::string_view` in C++17). Rule of thumb: store `String` in structs, accept `&str` in function parameters. | C++: `std::string` vs `std::string_view`. Go: `string` — Go doesn't expose the distinction; all strings are immutable and reference-counted internally. |
| **Clone vs Copy** | `Copy` = bitwise copy is safe (integers, bools, small types). `Clone` = explicit `.clone()` call, can be expensive. Types with heap data (String, Vec) are `Clone` but not `Copy`. Types with references are `Copy` if the reference is `Copy` (which `&T` is). | C++: copy constructors. Go: all assignment is a copy, but slices/maps are reference types internally (confusing!). Rust makes this explicit. |
| **`impl Into<T>` vs `impl AsRef<T>`** | `Into<T>` = consumes self, converts. `AsRef<T>` = borrows self, returns reference. Used in function parameters for flexibility. `fn create_user(name: impl Into<String>)` accepts both `&str` and `String`. | C++: implicit conversions via constructors. Go: no function overloading, so you'd use interfaces. |
| **Middleware (Tower Layers)** | Axum uses `tower::Layer` pattern. A middleware wraps the service, intercepting requests/responses. `from_fn` creates middleware from async functions. This is where you'll implement JWT validation. | Go: HTTP middleware (e.g., `func(http.Handler) http.Handler`). Express.js: `app.use()`. Rust's approach is type-safe — the middleware type signature encodes what it does. |

### API Endpoints to Build
| Method | Path | Description | Auth Required |
|--------|------|-------------|---------------|
| `POST` | `/auth/register` | Register new user | No |
| `POST` | `/auth/login` | Login, returns JWT tokens | No |
| `POST` | `/auth/refresh` | Refresh access token | Refresh token |
| `POST` | `/auth/logout` | Invalidate tokens | Yes |
| `GET` | `/users/me` | Get current user profile | Yes |
| `PUT` | `/users/me` | Update profile | Yes |
| `GET` | `/users/:id` | Get public user profile | No (public) |

### Key Learnings Checklist
- [ ] Understand Argon2 password hashing and why it's preferred over bcrypt/scrypt
- [ ] Understand JWT structure: header, payload, signature
- [ ] Understand access token vs refresh token pattern
- [ ] Implement a reusable JWT auth middleware with Tower layers
- [ ] Understand `Clone` vs `Copy` through password types (passwords should NOT be Copy!)
- [ ] Understand `&str` vs `String` decisions in API design
- [ ] Handle token blacklisting with Redis (or in-memory for now)
- [ ] Understand how `tower::ServiceBuilder` composes middleware

---

## Phase 3: Shopping Cart Service — "State Management & Concurrency"

### Why This Service Third
- Stateful service with mutable shared state
- Deepens understanding of Rust's concurrency guarantees
- Introduces interior mutability patterns

### Rust Concepts Covered

| Concept | Explanation | Analogy |
|---------|-------------|---------|
| **Interior Mutability** | Rust's basic rule: you can have ONE mutable reference OR many immutable references. Interior mutability (`RefCell`, `Mutex`, `RwLock`) lets you mutate data through a shared (`&T`) reference — the check moves from compile-time to runtime. `RefCell<T>`: single-threaded, panics on violation. `Mutex<T>`: multi-threaded, blocks. `RwLock<T>`: multi-threaded, multiple readers or one writer. | C++: `std::mutex` + `std::lock_guard`. Go: `sync.Mutex` / `sync.RWMutex`. Key difference: Rust's `Mutex<T>` OWNS the data — you can't access `T` without locking. Go's `sync.Mutex` is separate from the data — you can forget to lock. |
| **`Arc<Mutex<T>>` pattern** | Shared ownership (`Arc`) + thread-safe mutation (`Mutex`). This is THE pattern for shared mutable state in async Rust. Every clone of the Arc points to the same allocation. The Mutex ensures only one task accesses T at a time. | C++: `std::shared_ptr<std::mutex>` + separate data. Go: a struct with `sync.Mutex` embedded. Rust's approach is more explicit and compiler-enforced. |
| **`tokio::sync::RwLock` vs `std::sync::RwLock`** | Tokio's RwLock is designed for async — `.read().await` and `.write().await` don't block the thread, they yield to the runtime. Std's RwLock blocks the OS thread, which is bad in async contexts. Rule: use `tokio::sync::RwLock` in async code, `std::sync::RwLock` in sync code. | This distinction doesn't exist in Go — all mutex operations are blocking but goroutines are cheap, so it's fine. In Rust async, blocking an OS thread starves other tasks on that thread. |
| **`DashMap`** | A concurrent hashmap — like `HashMap` wrapped in `RwLock` but with sharding for better performance. Each shard has its own lock, reducing contention. From the `dashmap` crate (not std). | Go: `sync.Map`. Java: `ConcurrentHashMap`. C++: `tbb::concurrent_hash_map`. |

### API Endpoints to Build
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/cart` | Get current user's cart |
| `POST` | `/cart/items` | Add item to cart |
| `PUT` | `/cart/items/:product_id` | Update quantity |
| `DELETE` | `/cart/items/:product_id` | Remove item from cart |
| `DELETE` | `/cart` | Clear cart |

### Key Learnings Checklist
- [ ] Understand the difference between `std::sync::Mutex` and `tokio::sync::Mutex`
- [ ] Understand why you should prefer `tokio::sync::RwLock` in async handlers
- [ ] Understand `Arc::clone()` — it only increments the reference counter, doesn't clone the data
- [ ] Compare `HashMap<String, Mutex<CartItem>>` vs `DashMap<String, CartItem>` — learn about lock granularity
- [ ] Implement cart expiry using `tokio::spawn` with `tokio::time::sleep`
- [ ] Understand Drop — what happens when the last `Arc` reference is dropped?
- [ ] Learn about `std::sync::atomic` for lock-free counters (e.g., cart item count)

---

## Phase 4: Order Service — "Domain Modeling & Transactions"

### Why This Service Fourth
- Complex domain logic with state machines
- First inter-service communication (calls Product Service for stock)
- Database transactions in Rust

### Rust Concepts Covered

| Concept | Explanation | Analogy |
|---------|-------------|---------|
| **Enums with data (sum types)** | Rust enums are tagged unions — each variant can hold different data. `enum OrderStatus { Pending, Confirmed { at: DateTime<Utc> }, Shipped { tracking: String }, Delivered, Cancelled { reason: String } }`. This is impossible to represent safely in Go or C (use union + tag). C++ has `std::variant` but it's clunky. | C: tagged unions (manual, error-prone). C++: `std::variant<...>` + `std::visit`. Go: interfaces or const iota enums (no data attached, poor man's solution). Rust enums are one of the language's killer features. |
| **Pattern matching exhaustiveness** | `match` on an enum MUST handle all variants (or use `_` wildcard). If you add a new variant, the compiler tells you every `match` that needs updating. This eliminates entire classes of bugs. | C/C++: `switch` on enums — compiler warns but doesn't enforce. Go: `switch` with no exhaustiveness. TypeScript: discriminated unions + `never` type = similar. |
| **Database transactions** | `sqlx::Transaction<'_, Postgres>` — the transaction owns the connection. All queries run on the transaction. `.commit()` or `.rollback()` must be called. Rust's type system ensures you can't accidentally use the connection after committing. | Go: `db.Begin()` returns `*sql.Tx`. You can forget to commit/rollback. Rust: the compiler forces you to handle the transaction result. |
| **`impl From<X> for Y`** | Conversion trait. `impl From<CreateOrderRequest> for Order { fn from(req: CreateOrderRequest) -> Self { ... } }`. This plus `.into()` makes conversions ergonomic and type-safe. | C++: conversion constructors and `operator T()`. Go: no standard conversion pattern — you write `func NewOrderFromRequest(req) Order`. |
| **HTTP client (reqwest)** | `reqwest` is Rust's HTTP client. Used to call other services. `reqwest::Client` is cheap to clone (uses `Arc` internally). The async API integrates with Tokio. | Go: `net/http` client. C++: libcurl. JavaScript: `fetch` / `axios`. |

### API Endpoints to Build
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/orders` | Place an order (validates stock via Product Service) |
| `GET` | `/orders` | List user's orders |
| `GET` | `/orders/:id` | Get order details |
| `PATCH` | `/orders/:id/status` | Update order status (state machine) |
| `POST` | `/orders/:id/cancel` | Cancel an order |

### Order State Machine
```
                    ┌──────────┐
                    │  Pending  │
                    └────┬─────┘
                         │
                    ┌────▼─────┐
               ┌────│ Confirmed │────┐
               │    └────┬─────┘    │
               │         │          │
          ┌────▼───┐ ┌───▼────┐ ┌───▼────────┐
          │Shipped │ │Canceled│ │PaymentFailed│
          └────┬───┘ └────────┘ └────────────┘
               │
          ┌────▼──────┐
          │ Delivered  │
          └───────────┘
```

### Key Learnings Checklist
- [ ] Master Rust enums: how they differ from C enums and Go's iota
- [ ] Implement a state machine with exhaustive pattern matching
- [ ] Understand `sqlx::Transaction` — how Rust ensures transaction safety
- [ ] Make inter-service HTTP calls with `reqwest`
- [ ] Understand `impl From<X> for Y` for clean conversions
- [ ] Handle distributed transaction failures (what if Product Service succeeds but Order Service crashes?)
- [ ] Learn about `rust_decimal` for precise monetary calculations (never use f64 for money!)
- [ ] Understand `#[sqlx(type_name = "order_status")]` for custom enum types in Postgres

---

## Phase 5: Payment Service — "External APIs & Error Resilience"

### Why This Service Fifth
- Integration with external services (Stripe/PayPal)
- Real-world error handling patterns
- Idempotency and retry logic

### Rust Concepts Covered

| Concept | Explanation | Analogy |
|---------|-------------|---------|
| **Retry with backoff** | `backon` crate provides retry with exponential backoff and jitter. `fetch_with_retry.retry(ExponentialBuilder::default()).await`. This is critical for external API calls. | Go: `cenkalti/backoff` library. JavaScript: `p-retry`. The pattern is the same — Rust's type system ensures errors are handled. |
| **Idempotency** | Payment APIs MUST be idempotent — if the client sends the same request twice (network retry), you process it once. Implemented with an idempotency key stored in Redis/DB before processing. | Same pattern in all languages. Rust's strength: the type system can encode "this operation requires an idempotency key" at compile time using newtypes. |
| **Newtype pattern** | `struct IdempotencyKey(String)` — a wrapper type that prevents mixing up a plain String with an idempotency key. Zero runtime cost, compile-time safety. `struct UserId(uuid::Uuid)`, `struct OrderId(uuid::Uuid)` — prevents passing a UserId where an OrderId is expected. | C++: strong typedefs (e.g., `BOOST_STRONG_TYPEDEF`). Go: named types (`type UserID string`). Rust's newtypes have zero overhead due to monomorphization. |
| **`Pin<Box<dyn Future>>`** | Advanced: needed for self-referential types or async recursion. You likely won't need this directly — frameworks handle it. But it's worth knowing: `Pin` guarantees a value won't be moved in memory, which is needed for futures that hold references to themselves. | No direct C++/Go equivalent. This is a Rust-specific concept arising from the move semantics. |

### API Endpoints to Build
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/payments` | Create a payment intent |
| `GET` | `/payments/:id` | Get payment status |
| `POST` | `/payments/:id/confirm` | Confirm payment |
| `POST` | `/payments/webhook` | Receive Stripe/PayPal webhooks |
| `GET` | `/payments` | List user's payments |

### Key Learnings Checklist
- [ ] Understand the newtype pattern and why it prevents bugs
- [ ] Implement idempotency with Redis or database
- [ ] Implement retry with exponential backoff using `backon`
- [ ] Understand webhook signature verification (HMAC)
- [ ] Handle partial failures gracefully
- [ ] Understand `rust_decimal` for money — never use floating point for currency!
- [ ] Learn about graceful shutdown: `tokio::signal::ctrl_c()` and `axum::serve().with_graceful_shutdown()`

---

## Phase 6: Notification Service — "Async Messaging & Event-Driven Architecture"

### Why This Service Sixth
- Event-driven rather than request-driven
- Message queues and pub/sub
- Deepens async understanding

### Rust Concepts Covered

| Concept | Explanation | Analogy |
|---------|-------------|---------|
| **Message queues (AMQP/RabbitMQ)** | `lapin` crate for RabbitMQ. Producer sends messages to an exchange, consumers listen on queues. This decouples services. The Notification Service doesn't need to know about Order/Payment internals. | Go: `streadway/amqp`. JavaScript: `amqplib`. Same protocol (AMQP 0-9-1), different client libraries. |
| **`tokio::select!`** | Race multiple futures — the first one to complete wins, others are cancelled. Useful for: "process messages OR handle shutdown signal OR health check". `select! { msg = consumer.next() => { ... }, _ = shutdown_signal => { ... } }` | Go: `select` on channels. JavaScript: `Promise.race()`. Rust's `select!` works on futures, which is more general than channels. |
| **Streams** | `futures::Stream` — an async iterator. `StreamExt` provides combinators: `.map()`, `.filter()`, `.for_each()`, `.buffer_unordered()`, `.chunks()`. `while let Some(item) = stream.next().await { ... }` | Go: channels are streams. JavaScript: `AsyncIterator` / `for await...of`. Rust's Stream trait is the async equivalent of `Iterator`. |
| **`tokio::sync::mpsc`** | Multi-producer, single-consumer channel for async communication between tasks. `tx.send(msg).await` — async send (waits if channel is full). `rx.recv().await` — async receive. Buffered channels have capacity. | Go: buffered channels (`make(chan T, N)`). Rust's mpsc is SPSC or MPSC — no multi-consumer (use `broadcast` for that). |

### Events to Handle
| Event | Payload | Action |
|-------|---------|--------|
| `user.registered` | `{ user_id, email }` | Send welcome email |
| `order.created` | `{ order_id, user_id, items, total }` | Send order confirmation |
| `order.shipped` | `{ order_id, tracking_number, carrier }` | Send shipping notification |
| `payment.succeeded` | `{ payment_id, order_id, amount }` | Send payment receipt |
| `payment.failed` | `{ payment_id, order_id, reason }` | Send payment failure notice |

### Key Learnings Checklist
- [ ] Understand AMQP basics: exchanges, queues, bindings, routing keys
- [ ] Understand `tokio::select!` and how it compares to Go's `select`
- [ ] Implement a consumer that gracefully shuts down
- [ ] Understand `Stream` vs `Iterator` — the async/sync distinction
- [ ] Implement templated emails with `tera` or `handlebars`
- [ ] Learn about `futures::stream::StreamExt` combinators
- [ ] Pattern: dead letter queue for failed messages

---

## Phase 7: API Gateway & Service Discovery

### Why This Phase Seventh
- Infrastructure-level concern
- Less Rust-specific learning, more architecture learning
- Integration point for all services

### Gateway Options
| Option | Rust Learning | Production Readiness | Recommendation |
|--------|---------------|---------------------|----------------|
| **Traefik** | Minimal Rust, learn config | Excellent | Use for production |
| **NGINX** | Minimal Rust, learn config | Excellent | Use for production |
| **Custom Rust proxy** | High Rust learning (Tower, http crate, connection pooling) | Requires effort | Build as optional bonus |
| **Kong** | Minimal Rust, learn config + Lua | Excellent | Viable alternative |

### Key Learnings Checklist
- [ ] Understand API Gateway patterns: routing, rate limiting, auth at edge
- [ ] Understand CORS configuration
- [ ] Implement request ID propagation for distributed tracing
- [ ] (Optional) Build a Rust reverse proxy with `hyper` + `http` crate
- [ ] Understand service discovery with Consul/DNS

---

## Phase 8: Docker, Monitoring & CI/CD — "Production Rust"

### Why This Phase Last
- Production concerns that wrap all services
- Dockerizing Rust is unique (multi-stage builds, `cargo chef`)
- Monitoring and observability

### Docker for Rust
```dockerfile
# Build stage
FROM rust:1.80-slim AS builder
RUN cargo install cargo-chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/product-service /usr/local/bin/
CMD ["product-service"]
```

**Why `cargo chef`?** Rust compilation is slow. `cargo chef` caches dependencies separately from source code, so dependency compilation is cached in Docker layers. Only your code recompiles on changes. This is equivalent to `go mod download` in Go or `npm ci` in Node.js — but for Rust.

### Monitoring Stack
| Tool | Purpose | Configuration |
|------|---------|---------------|
| **Prometheus** | Metrics collection | `axum-prometheus` crate adds `/metrics` endpoint |
| **Grafana** | Dashboards | Import Rust service dashboards |
| **Loki + Promtail** | Log aggregation | `tracing` emits structured JSON → Loki |
| **Tempo** (optional) | Distributed tracing | `tracing-opentelemetry` crate |

### CI/CD with GitHub Actions
```yaml
- cargo fmt --check        # Format checking
- cargo clippy -- -D warnings  # Linting (like golangci-lint)
- cargo test               # Unit + integration tests
- cargo build --release    # Release build with optimizations
- docker build ...         # Container image
- docker push ...          # Push to registry
```

### Key Learnings Checklist
- [ ] Understand multi-stage Docker builds for Rust
- [ ] Understand `cargo chef` for Docker layer caching
- [ ] Understand compile-time optimizations: `lto = true`, `codegen-units = 1`, `opt-level = "z"` vs `"s"` vs `3`
- [ ] Instrument services with Prometheus metrics
- [ ] Set up structured logging with `tracing` + Loki
- [ ] Write GitHub Actions CI/CD pipeline
- [ ] Understand `cargo audit` for vulnerability scanning
- [ ] Understand `cargo deny` for license compliance

---

## Phases 9-10: gRPC & Advanced Rust (Bonus)

### Phase 9: gRPC Migration
- Replace REST inter-service calls with gRPC
- **Crates:** `tonic` (gRPC server/client), `prost` (protobuf codegen)
- **Learn:** Protocol Buffers, streaming gRPC (unary, server streaming, client streaming, bidirectional)
- **Compares to:** Go's gRPC ecosystem (protoc-gen-go)

### Phase 10: Advanced Rust Concepts
- **Procedural macros**: Write your own `#[derive(...)]` macros
- **Declarative macros**: `macro_rules!` for code generation
- **`unsafe` Rust**: When and why to use it (FFI with C payment SDKs?)
- **Const generics**: Arrays with compile-time sizes, matrix operations
- **GATs (Generic Associated Types)**: Advanced trait patterns
- **Async traits**: The `async_trait` crate and why it's needed

---

## Project Structure (Final)

```
rust_ecommerce/
├── Cargo.toml                    # Workspace manifest
├── Cargo.lock
├── LEARNING_PLAN.md              # This document
├── docker-compose.yml
├── docker-compose.override.yml   # Dev overrides (hot reload, etc.)
├── .env.example
├── .github/
│   └── workflows/
│       ├── ci.yml                # Test + lint on PR
│       └── deploy.yml            # Build + push Docker images
├── crates/
│   ├── common/                   # Shared library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs          # Common error types
│   │       ├── auth.rs           # JWT middleware
│   │       ├── pagination.rs     # Pagination helpers
│   │       └── events.rs         # Event type definitions
│   ├── product-service/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   ├── src/
│   │   └── tests/
│   ├── user-service/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   ├── src/
│   │   └── tests/
│   ├── cart-service/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   ├── src/
│   │   └── tests/
│   ├── order-service/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   ├── src/
│   │   └── tests/
│   ├── payment-service/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   ├── src/
│   │   └── tests/
│   └── notification-service/
│       ├── Cargo.toml
│       ├── src/
│       └── tests/
├── scripts/
│   ├── init-db.sh               # Database setup
│   └── run-migrations.sh        # Migration runner
└── docs/
    ├── api-spec.md              # API documentation
    └── architecture.md          # Architecture decisions
```

---

## Quick Reference: Rust vs Your Known Languages

| Concept | C | C++ | Go | Rust |
|---------|---|-----|-----|------|
| Memory management | `malloc`/`free` | RAII, smart pointers | GC (tracing) | Ownership + Borrow Checker |
| Null/None | `NULL` ptr | `nullptr`, `std::optional` | `nil` | `Option<T>` |
| Error handling | Return codes | Exceptions, `std::expected` | `(T, error)` tuple | `Result<T, E>` |
| Polymorphism | Function pointers | Virtual methods, templates | Interfaces (duck typing) | Traits, generics (monomorphized) |
| Concurrency | pthreads | `std::thread`, `std::async` | Goroutines | `tokio::spawn` (async tasks) |
| Package manager | None | CMake/vcpkg/Conan | `go mod` | Cargo (built-in) |
| Build system | Make/CMake | CMake/Bazel | `go build` | Cargo (built-in) |
| Testing | Separate frameworks | GTest/Catch2 | `go test` | `cargo test` (built-in) |
| Linting | cppcheck/clang-tidy | clang-tidy | `golangci-lint` | `cargo clippy` (built-in) |
| Formatting | clang-format | clang-format | `gofmt` | `cargo fmt` (built-in) |

## Key Rust Philosophy Points (from a C++/Go perspective)

1. **"Make illegal states unrepresentable"** — Use enums with data, newtypes, and the type system to prevent bugs at compile time. This is the Rust equivalent of "if it compiles, it probably works."

2. **"Zero-cost abstractions"** — Like C++, Rust's high-level constructs (iterators, closures, async/await) compile down to the same machine code as hand-written loops. Unlike Go, which has a runtime cost for goroutines and interfaces.

3. **"Fearless concurrency"** — The type system prevents data races at compile time. `Send` and `Sync` traits encode thread-safety. If your code compiles without `unsafe`, it has no data races. This is a game-changer coming from C++/Go.

4. **"Explicit over implicit"** — Rust makes costs visible: `.clone()` is explicit, `Arc` reference counting is visible, `Mutex` locking is visible. Go and C++ hide some of these costs.

5. **"The compiler is your teacher"** — Rust's error messages are famously helpful. When the borrow checker rejects your code, it explains why and often suggests a fix. Treat compiler errors as learning opportunities, not obstacles.

---

## Getting Started

1. **Install Rust**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. **IDE**: VS Code with `rust-analyzer` extension (essential!)
3. **Toggle to ACT mode** to begin Phase 0 scaffolding and Phase 1 development.