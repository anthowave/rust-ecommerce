mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod routes;

use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Settings;

/// Application state shared across all handlers via `axum::extract::State`.
///
/// # Rust Concept: Arc<T> (Atomic Reference Counted)
///
/// `Arc<AppState>` is the Rust pattern for shared application state.
/// Every call to `Arc::clone(&state)` only increments an atomic counter —
/// it does NOT clone the inner data. This is exactly `std::shared_ptr<T>`
/// in C++ or a reference to a singleton in Go.
///
/// **Why Arc and not Rc?** `Rc<T>` is single-threaded (non-atomic counter).
/// Axum runs handlers on a multi-threaded Tokio runtime, so we need
/// thread-safe reference counting. The compiler would reject `Rc<AppState>`
/// because `Rc` doesn't implement `Send`.
///
/// **C++ analogy:** `std::shared_ptr<T>` (always atomic).
/// **Go analogy:** A package-level variable or dependency injection.
/// **Key difference:** Rust lets you CHOOSE between `Rc` (fast, single-thread)
/// and `Arc` (atomic, multi-thread). C++ `shared_ptr` is always atomic.
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub settings: Settings,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured logging
    // RUST_LOG=info cargo run  → shows info! and above
    // RUST_LOG=debug cargo run → shows debug! and above
    // RUST_LOG=user_service=debug cargo run → only this crate at debug level
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load configuration from environment
    // This is where you'd set USER_SERVICE__DATABASE_URL, USER_SERVICE__JWT_SECRET, etc.
    let settings = Settings::from_env()?;

    // Create database connection pool
    let pool = db::create_pool(&settings.database_url).await?;

    // Run database migrations (compile-time checked!)
    // sqlx::migrate!() verifies the SQL files at compile time against your database.
    // If a migration file has a SQL syntax error, you get a COMPILE ERROR, not a runtime error.
    // C++/Go analogy: Like having your DBA review every migration at build time.
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let state = Arc::new(AppState { pool, settings });

    // Build the router
    let app = routes::create_router(state.clone());
    let addr = format!("{}:{}", state.settings.host, state.settings.port);
    // Parse the address into a SocketAddr
    let addr: std::net::SocketAddr = addr.parse()?;

    info!("User service starting on {}", addr);

    // Start the server with graceful shutdown on Ctrl+C
    // axum::serve takes a listener and the router.
    // with_graceful_shutdown ensures in-flight requests complete before the server stops.
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Signal handler for graceful shutdown.
///
/// Listens for Ctrl+C (SIGINT) or SIGTERM (Docker stop).
/// When triggered, Axum stops accepting new connections and waits
/// for in-flight requests to complete.
///
/// **C++ analogy:** Signal handler with `sigaction`.
/// **Go analogy:** `signal.NotifyContext` + `<-ctx.Done()`.
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
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        },
    }
}
