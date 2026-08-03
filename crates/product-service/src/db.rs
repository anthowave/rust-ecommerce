// =============================================================================
// Phase 1, Step 3: Database Connection Pool
// =============================================================================
// KEY LESSON: SQLx Connection Pooling
// ====================================
// sqlx::PgPool is an async connection pool — it manages multiple connections
// to PostgreSQL, reusing them across queries. This is CRITICAL because:
//   - Creating a new TCP connection per query is slow
//   - Postgres has a max connection limit (~100 by default)
//   - Connection pools reuse connections, reducing overhead
//
// COMPARISON:
//   Go:     database/sql.DB — also a pool (but you need to manually set MaxOpenConns etc.)
//   Java:   HikariCP — the gold standard for JDBC connection pooling
//   Node:   pg-pool, knex pool — similar concept
//   Rust:   sqlx::PgPool — built-in pool, no separate library needed
//
// HOW IT WORKS:
//   - When you call pool.acquire().await, SQLx checks if there's an idle connection
//   - If yes, return it immediately
//   - If no, and we haven't hit max_connections, create a new one
//   - If we're at max_connections, wait (with timeout) for one to become free
//
// The pool is CLONE-FRIENDLY: cloning a PgPool doesn't create a new pool.
// It creates a new handle to the SAME pool (like Arc::clone). This means
// you can freely pass the pool to multiple handlers without worrying about
// ownership.

use sqlx::postgres::PgPoolOptions;
use tracing::info;

/// Create a connection pool for PostgreSQL.
///
/// KEY LESSON: The `async fn` signature
/// =====================================
/// This function is `async` — it returns a Future. The caller must `.await` it.
/// Inside, we use `.await` on `connect()` to yield to the Tokio runtime while
/// the TCP connection is established. During this time, Tokio can run other tasks.
///
/// In Go: this would be a blocking call (sql.Open doesn't actually connect —
/// you'd use db.Ping() which blocks the goroutine). In Rust async, `.await`
/// on I/O never blocks the OS thread — it yields to the runtime.
///
/// KEY LESSON: Result<T, E> return type
/// =====================================
/// Returning `Result<PgPool, sqlx::Error>` forces the CALLER to handle the
/// error case. They can't accidentally use the pool without checking if
/// connection succeeded. This is the Rust way: make errors un-ignorable.
///
/// In Go: you'd return (*sql.DB, error) but nothing forces the caller to
/// check the error. In Rust: the compiler won't let you use the Ok value
/// without handling the Result.
pub async fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    // KEY LESSON: Builder pattern
    // ============================
    // PgPoolOptions::new().max_connections(n).connect(url).await
    // This is the builder pattern: chain configuration methods, then call the
    // terminal method (connect). Each method returns Self (or a modified Self).
    // This is idiomatic Rust — see also: std::process::Command, reqwest::Client.
    //
    // In Go: you'd set fields on a struct: opts := PoolOptions{}; opts.MaxConns = n
    // In C++: similar builder pattern (fluent interface)

    info!(
        max_connections = max_connections,
        "Connecting to PostgreSQL..."
    );

    let pool = PgPoolOptions::new()
        // Maximum number of connections in the pool
        .max_connections(max_connections)
        // Timeout for acquiring a connection from the pool
        // If all connections are in use and we wait this long, return an error
        .acquire_timeout(std::time::Duration::from_secs(5))
        // Connect! This is where the magic happens:
        //   1. Resolve DNS
        //   2. Establish TCP connection
        //   3. Perform PostgreSQL handshake
        //   4. Authenticate
        //   5. Create initial connections in the pool
        .connect(database_url)
        .await?; // KEY LESSON: `?` on a Result<_, sqlx::Error> returns the error
    // to our caller. This is like Go's `if err != nil { return err }`.

    // KEY LESSON: The `info!` macro from `tracing`
    // =============================================
    // `info!` is a structured logging macro. Unlike Go's `log.Printf()` which
    // produces unstructured text, `info!` records key-value pairs as structured
    // fields. This enables:
    //   - Filtering logs by field values in production
    //   - JSON output for log aggregation systems (ELK, Loki)
    //   - Compile-time log level filtering (info! calls can be completely
    //     removed in release builds at trace/debug levels)
    //
    // Syntax: info!(field_name = value, "message with {} placeholders", arg)
    //         info!("message")  // simple message
    //         info!(%var)       // display-format the variable
    //         info!(?var)       // debug-format the variable
    info!("Successfully connected to PostgreSQL");

    Ok(pool)
}

// KEY LESSON: Type alias for shared application state
// ====================================================
// `type` is Rust's type alias — like `typedef` in C/C++, or `type` in Go.
// PgPool = sqlx::Pool<sqlx::Postgres>
// This is just for readability. No runtime cost.
pub type PgPool = sqlx::PgPool;
