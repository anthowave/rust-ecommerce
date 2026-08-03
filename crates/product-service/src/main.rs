// =============================================================================
// Phase 1, Step 10: Main — The Entry Point
// =============================================================================
// KEY LESSON: Rust program entry point
// =====================================
// `fn main()` is the entry point, just like C/C++ (`int main()`), Go (`func main()`),
// or JavaScript (top-level execution in Node). The key difference:
//   - Rust's `main` can optionally return `Result<(), E>` for clean error handling
//   - But for async programs, we need `#[tokio::main]` to create the async runtime
//
// `#[tokio::main]` — this attribute macro transforms our `async fn main()` into
// a synchronous `fn main()` that:
//   1. Creates a Tokio runtime (multi-threaded by default)
//   2. Calls our async function on it
//   3. Blocks the main thread until our function completes
//   4. Handles graceful shutdown on Ctrl+C
//
// This is like Go's implicit runtime (you don't write it, but it's there).
// In Rust, we're explicit about which async runtime we use (Tokio vs async-std vs smol).

use anyhow::Context;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod db;
mod error;
mod handlers;
mod models;
mod routes;

/// Application entry point.
///
/// KEY LESSON: `#[tokio::main]` attribute
/// =======================================
/// This macro generates approximately:
/// ```ignore
/// fn main() {
///     let runtime = tokio::runtime::Runtime::new().unwrap();
///     runtime.block_on(async {
///         run_server().await
///     });
/// }
/// ```
/// It's the equivalent of Go's implicit runtime that manages goroutines,
/// but in Rust we're EXPLICIT about using Tokio. This gives us control:
/// we can configure thread count, work-stealing, and task scheduling.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ─── STEP 1: Initialize tracing (structured logging) ─────────────────────
    // KEY LESSON: tracing-subscriber initialization
    // ==============================================
    // `tracing_subscriber::fmt()` creates a subscriber that outputs logs to stdout.
    // `.with_env_filter(EnvFilter::from_default_env())` reads the `RUST_LOG`
    // environment variable to control log levels:
    //   RUST_LOG=debug          → debug level for all crates
    //   RUST_LOG=product_service=debug,sqlx=warn  → per-crate filtering
    //   RUST_LOG=info           → production default
    //
    // `.json()` would output JSON for log aggregators (ELK, Loki).
    // We use text format here for development readability.
    //
    // In Go: you'd configure `log/slog` with levels via `slog.SetDefault()`.
    // In JS: you'd configure `winston` or `pino` with log levels.
    // In Rust: `tracing-subscriber` is the standard, and it's configured once at startup.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false) // Don't show module path in logs (cleaner output)
        .init();

    info!("Starting Product Service...");

    // ─── STEP 2: Load configuration ──────────────────────────────────────────
    // Load from environment variables (APP_DATABASE_URL, APP_SERVER_PORT, etc.)
    // KEY LESSON: `.context()` from anyhow adds context to errors
    // ===========================================================
    // If `load_config()` fails, the error message will include "Failed to load config"
    // as context. This creates a chain of context from the error source to our
    // application-level message. Like Go's `fmt.Errorf("...: %w", err)` but more ergonomic.
    let settings = config::load_config().context("Failed to load config")?;

    info!(
        host = %settings.server.host,
        port = settings.server.port,
        "Configuration loaded"
    );

    // ─── STEP 3: Create database connection pool ─────────────────────────────
    let pool = db::create_pool(&settings.database.url, settings.database.max_connections)
        .await
        .context("Failed to create database pool")?;

    // KEY LESSON: Run migrations at startup
    // ======================================
    // `sqlx::migrate!()` runs all SQL files in the `migrations/` directory.
    // This is like `golang-migrate` auto-migration, or TypeORM `synchronize`.
    // In production, you'd typically run migrations separately, but for
    // learning/development, auto-migration is convenient.
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    info!("Database migrations applied successfully");

    // ─── STEP 4: Create shared application state ────────────────────────────
    // KEY LESSON: Arc::new() — creating shared ownership
    // ===================================================
    // `Arc::new(AppState { db: pool })` creates the state with shared ownership.
    // The pool is MOVED into the Arc — the pool itself is clone-friendly (as
    // explained in db.rs), but we put it in Arc for the whole AppState struct.
    //
    // This is conceptually similar to creating a singleton in Go/JS, but with
    // Rust's compile-time thread-safety guarantees.
    let state = Arc::new(handlers::products::AppState { db: pool });

    // ─── STEP 5: Create the router ──────────────────────────────────────────
    let router = routes::create_router(state);

    // ─── STEP 6: Bind and serve ──────────────────────────────────────────────
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    // KEY LESSON: `SocketAddr` parsing — like Go's `net.ResolveTCPAddr`
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    info!("Server listening on {addr}");

    // KEY LESSON: `axum::serve()` — the HTTP server
    // ==============================================
    // Unlike Go where `http.ListenAndServe` is a simple function, Axum's serve
    // is a future that runs until shutdown signal is received. This allows:
    //   - Graceful shutdown: finish in-flight requests, then stop
    //   - Multiple listeners: serve on multiple ports/protocols
    //   - Integration with health checks and readiness probes
    //
    // `.with_graceful_shutdown(shutdown_signal())` — listens for Ctrl+C (SIGINT)
    // or SIGTERM, then waits for in-flight requests to complete before exiting.
    // This prevents dropping requests mid-processing.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    Ok(())
}

/// Graceful shutdown signal handler.
///
/// KEY LESSON: tokio::signal — OS signal handling in async
/// ========================================================
/// `tokio::signal::ctrl_c()` returns a future that resolves when Ctrl+C is pressed.
/// This is like Go's `signal.Notify(ch, os.Interrupt)` but integrated into the async runtime.
///
/// In a real production service, you'd also handle SIGTERM (Kubernetes pod shutdown).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        info!("Received shutdown signal (Ctrl+C), shutting down gracefully...");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
        info!("Received SIGTERM, shutting down gracefully...");
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // KEY LESSON: tokio::select! — race two futures
    // ==============================================
    // `tokio::select!` waits for the FIRST future to complete, cancelling the other.
    // This is like Go's `select` statement for channels, or JavaScript's `Promise.race()`.
    // In this case: wait for either Ctrl+C or SIGTERM — whichever comes first.
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
