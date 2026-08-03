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
// We implement IntoResponse HERE (in the common crate where AppError is defined)
// because of Rust's ORPHAN RULE: you can't implement a foreign trait (IntoResponse
// from axum) for a foreign type (AppError from common) in a third crate.
// The implementation must be in the same crate as EITHER the trait OR the type.
//
// Since AppError is our type (in common), we implement IntoResponse here.
// Every service uses this single implementation. No duplicate code.
//
// This is the Rust equivalent of exception-to-HTTP mapping in Spring Boot or
// error handling middleware in Express.js — but it's type-safe and explicit.
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tracing::error;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound { entity, id } => {
                let msg = format!("{entity} with id {id} not found");
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::DatabaseError(_) => {
                error!(error = %self, "Database error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::ExternalServiceError(_msg) => {
                error!(error = %self, "External service error");
                (
                    StatusCode::BAD_GATEWAY,
                    "External service temporarily unavailable".to_string(),
                )
            }
            AppError::InternalError(_) => {
                error!(error = %self, "Internal error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = json!({
            "error": {
                "message": message,
                "status": status.as_u16(),
            }
        });

        (status, Json(body)).into_response()
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
