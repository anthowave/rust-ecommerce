// =============================================================================
// PHASE 3: Cart Service — Main Entry Point
// =============================================================================
// KEY LESSON: Wiring everything together
// =======================================
// The main function ties together:
//   1. Configuration (environment variables)
//   2. Logging (tracing subscriber)
//   3. AppState (settings + cart store)
//   4. Router (with auth middleware)
//   5. HTTP server (with graceful shutdown)
//
// This is the same pattern as product-service and user-service.
// Consistency across services makes the codebase predictable.
//
// # How Tokio's main macro works:
//
// `#[tokio::main]` transforms `async fn main()` into a regular `fn main()`
// that creates the Tokio runtime and runs the async function to completion.
//
// Behind the scenes:
// ```ignore
// fn main() {
//     let rt = tokio::runtime::Runtime::new().unwrap();
//     rt.block_on(async_main());
// }
// ```
//
// **Go analogy:** The Go runtime is always running — no explicit setup needed.
// **C++ analogy:** Creating an event loop or thread pool manually.
// **Rust:** You choose the runtime (Tokio, async-std, smol) and configure it.

mod auth;
mod config;
mod error;
mod handlers;
mod middleware;
mod models;
mod routes;

use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::Settings;
use models::cart::CartStore;

/// Application state shared across all handlers via Axum's `State` extractor.
///
/// KEY LESSON: Arc<AppState> — shared ownership
/// ==============================================
/// AppState is wrapped in `Arc` (Atomic Reference Counting) so all handlers
/// and middleware can access it concurrently. `Arc::clone()` is cheap —
/// it only bumps an atomic counter, doesn't copy the data.
///
/// Compare:
///   - C++: shared_ptr<AppState>
///   - Go: pass by pointer (all goroutines share the same struct)
///   - Rust: Arc<T> — explicit, visible reference counting
///
/// # Why AppState instead of separate State extractors?
///
/// Axum allows multiple `State<T>` extractors, but grouping everything
/// into AppState is simpler. The middleware needs settings (JWT secret),
/// the handlers need CartStore. Both are in AppState.
pub struct AppState {
    pub settings: Settings,
    pub cart_store: CartStore,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ─── 1. Load .env file (if present) ─────────────────────────────────────
    // dotenvy::dotenv() reads a `.env` file and sets environment variables.
    // It's OK if the file doesn't exist — the function returns Ok.
    let _ = dotenvy::dotenv();

    // ─── 2. Initialize structured logging ───────────────────────────────────
    // tracing-subscriber with env-filter lets you control log levels per module.
    // Set RUST_LOG=cart_service=debug to see debug logs from cart service.
    // Set RUST_LOG=info to see info-level from all crates.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .init();

    // ─── 3. Load configuration ──────────────────────────────────────────────
    let settings = Settings::from_env()?;
    info!(
        "Cart Service starting on {}:{}",
        settings.host, settings.port
    );

    // ─── 4. Create the in-memory cart store ─────────────────────────────────
    // KEY LESSON: CartStore holds Arc<RwLock<HashMap<...>>>
    // The store is created here and will be shared across all handlers.
    // Cloning CartStore is cheap (Arc clone) — no data duplication.
    let cart_store = CartStore::new();

    // ─── 5. Build shared application state ──────────────────────────────────
    let state = Arc::new(AppState {
        settings,
        cart_store,
    });

    // ─── 6. Build the router ────────────────────────────────────────────────
    // KEY LESSON: Arc::clone() is cheap — only bumps an atomic refcount.
    // We clone state before passing it to create_router so we can still
    // use state below. This is the same pattern as user-service's main.rs.
    let app = routes::create_router(state.clone());

    // ─── 7. Bind and serve with graceful shutdown ───────────────────────────
    // We can still use state here because we only gave a clone to create_router.
    // The actual data (AppState) is shared via Arc, not copied.
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", state.settings.host, state.settings.port))
            .await?;

    info!("Listening on {}", listener.local_addr()?);

    // KEY LESSON: Graceful shutdown
    // ==============================
    // `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())`
    // tells the server to:
    //   1. Stop accepting new connections when shutdown signal fires
    //   2. Wait for in-flight requests to complete
    //   3. Close all connections cleanly
    //
    // This is critical for zero-downtime deploys. Without graceful shutdown,
    // killing the process drops in-flight requests.
    //
    // **Go analogy:** `http.Server.Shutdown(ctx)` with a signal handler.
    // **C++ analogy:** Custom signal handling with a flag to stop accept loop.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Cart Service shut down gracefully");
    Ok(())
}

/// Wait for a shutdown signal (Ctrl+C or SIGTERM).
///
/// KEY LESSON: tokio::signal — async signal handling
/// ===================================================
/// `tokio::signal::ctrl_c()` returns a Future that resolves when Ctrl+C is pressed.
/// This integrates with Tokio's event loop — no blocking, no threads needed.
///
/// **Go analogy:** `signal.Notify(ch, os.Interrupt, syscall.SIGTERM)`
/// **C++ analogy:** `signal()` handler (but must be careful about async-signal-safety)
/// **Rust:** Tokio handles signals as async events — much cleaner.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
        _ = terminate => info!("Received SIGTERM, shutting down..."),
    }
}
