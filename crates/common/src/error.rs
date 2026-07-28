// =============================================================================
// PHASE 0.6: ERROR HANDLING WITH thiserror
// =============================================================================
// KEY LESSON: thiserror — derive macro for custom error types
// =============================================================
// `thiserror` is a crate that provides `#[derive(Error)]` — it auto-generates
// the `std::fmt::Display` and `std::error::Error` trait implementations.
//
// This is for LIBRARY errors — errors that other code will match on.
// Compare to:
//   - Go: defining custom error types with `Error() string` method
//   - C++: subclassing std::exception
//   - JS: extending Error class
//
// The `#[error("...")]` attribute defines the Display message.
// The `#[from]` attribute auto-generates `From<SourceError> for YourError`,
//   enabling the `?` operator for automatic conversion.
// The `#[error(transparent)]` attribute forwards Display and source to the inner error.

use thiserror::Error;

/// Common error types shared across all services.
///
/// KEY LESSON: Error enum design
/// ==============================
/// This enum has variants for different error categories.
/// Each variant can carry metadata (like the entity ID that wasn't found).
/// This is more expressive than Go's `errors.New("not found")` — in Go, you can't
/// attach structured data to an error without parsing the error string.
///
/// The `#[error("...")]` attribute uses format string syntax.
/// `{0}` refers to the first field of the variant.
/// `{entity}` refers to a named field.
#[derive(Debug, Error)]
pub enum AppError {
    /// Entity not found (e.g., product with ID X doesn't exist).
    #[error("{entity} with id {id} not found")]
    NotFound { entity: &'static str, id: String },

    /// Validation error — the request data is invalid.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// Database error — wraps the underlying database error.
    /// `#[from]` means: if you call `?` on a `sqlx::Error`, it automatically
    /// converts to `AppError::DatabaseError`. This is like Go's `fmt.Errorf("...: %w", err)`
    /// but the conversion is automatic.
    #[error("database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// Authentication error.
    #[error("authentication error: {0}")]
    Unauthorized(String),

    /// Forbidden — authenticated but not authorized.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// External service error (e.g., payment gateway failure).
    #[error("external service error: {0}")]
    ExternalServiceError(String),

    /// Internal server error — something unexpected happened.
    /// `#[error(transparent)]` means: Display and source() are forwarded to the inner error.
    /// Use this for errors you don't want to expose details about.
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

// KEY LESSON: Implementing Axum's IntoResponse for our error type
// ================================================================
// This allows AppError to be returned directly from Axum handlers.
// Axum will automatically convert it to an HTTP response with the correct status code.
// This is the Rust equivalent of exception-to-HTTP mapping in Spring Boot or
// error handling middleware in Express.js — but it's type-safe and explicit.
//
// We'll implement this in Phase 1 when we set up Axum.
// For now, this is a placeholder showing the pattern.
impl AppError {
    /// Returns the HTTP status code for this error.
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// KEY LESSON: anyhow — for application-level error handling
// ===========================================================
// `anyhow` is a crate for APPLICATION code (binaries, not libraries).
// It provides `anyhow::Result<T>` = `Result<T, anyhow::Error>`.
// `anyhow::Error` can hold ANY error type (like Go's `error` interface).
// Use `.context("...")` to add context: `file.read().context("failed to read config")?`
// Use `.with_context(|| format!("..."))` for lazy context formatting.
// Use `anyhow::bail!("...")` for early returns: like Go's `return fmt.Errorf("...")`.
//
// RULE OF THUMB:
//   - Library code: use `thiserror` to define specific error types
//   - Application code: use `anyhow` for general error propagation
//   - Compare: Go's `fmt.Errorf` vs custom error types; C++ exceptions vs error codes
