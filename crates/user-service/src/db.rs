use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

/// Create a PostgreSQL connection pool.
///
/// # Rust Concept: Builder Pattern
///
/// `PgPoolOptions::new().max_connections().connect()` is the Rust builder
/// pattern — each method returns `Self` (or a modified version), allowing
/// method chaining. The final `.connect().await?` consumes the builder and
/// produces the Pool.
///
/// **C++ analogy:** Named Parameter Idiom / fluent interface.
/// **Go analogy:** Functional options pattern (e.g., `sql.Open("postgres", url)`
/// followed by `db.SetMaxOpenConns(N)`).
///
/// **Key difference:** In Rust, the builder method chain must end with a
/// "build" or "connect" call that consumes the builder. You can't accidentally
/// forget to call it — the compiler enforces this through ownership.
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("Failed to create database connection pool")?;

    info!("Database connection pool established");
    Ok(pool)
}