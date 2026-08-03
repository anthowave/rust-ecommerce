use axum::response::IntoResponse;

// Re-export the common error type so handlers can use it directly.
// This pattern avoids duplication: all services share the same base error type
// (not_found, validation_error, internal_error), but each service can add
// service-specific error variants if needed.
pub use common::error::AppError;

/// Auth-specific error types that extend beyond the shared AppError.
///
/// # Rust Concept: thiserror derive macro
///
/// `thiserror` auto-generates `std::fmt::Display` and `std::error::Error`
/// implementations. Each variant's `#[error("...")]` defines the display message.
/// This is equivalent to Go's `fmt.Errorf` or C++'s `std::runtime_error`,
/// but with the added benefit of pattern matching on error kinds.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid email or password")]
    InvalidCredentials,

    #[error("Email already registered")]
    EmailAlreadyExists,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Token has been revoked")]
    TokenRevoked,

    #[error("User not found")]
    UserNotFound,

    #[error("Unauthorized — authentication required")]
    Unauthorized,
}

/// Convert AuthError into an HTTP response.
///
/// # Rust Concept: impl IntoResponse for custom error types
///
/// By implementing `IntoResponse`, we can use `AuthError` directly in Axum
/// handler return types with the `?` operator (if we also implement `From<AuthError>`
/// for something that wraps it, or return `Result<_, AuthError>` directly).
///
/// **Why not put this in common?** The orphan rule: we could put it in common,
/// but auth-specific error responses don't belong in a shared library.
/// Each service owns its domain-specific errors.
///
/// **C++ analogy:** Overloading a conversion operator.
/// **Go analogy:** Implementing the `http.Handler` interface for your error type.
impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AuthError::InvalidCredentials => {
                (axum::http::StatusCode::UNAUTHORIZED, self.to_string())
            }
            AuthError::EmailAlreadyExists => (axum::http::StatusCode::CONFLICT, self.to_string()),
            AuthError::InvalidToken => (axum::http::StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::TokenRevoked => (axum::http::StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::UserNotFound => (axum::http::StatusCode::NOT_FOUND, self.to_string()),
            AuthError::Unauthorized => (axum::http::StatusCode::UNAUTHORIZED, self.to_string()),
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
