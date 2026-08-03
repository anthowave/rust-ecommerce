// =============================================================================
// PHASE 3: Cart Service Error Types
// =============================================================================
// KEY LESSON: Service-specific error types
// ==========================================
// Each service defines its own error enum with the `thiserror` derive macro.
// This gives type-safe error handling specific to the domain.
//
// The cart service wraps the common `AppError` for shared error patterns
// (NotFound, ValidationError, Unauthorized) but also defines cart-specific
// variants (e.g., InsufficientStock).
//
// COMPARISON:
//   Go: custom error types implementing the `error` interface
//   C++: subclassing std::exception
//   Rust: enums with `#[derive(Error)]` — exhaustively matchable errors

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;
use tracing::error;

/// Cart service error enum.
///
/// KEY LESSON: Error design with enums
/// ====================================
/// Each variant represents a category of error. The `#[error("...")]` attribute
/// defines the Display message. Named fields allow structured error data.
///
/// Why enums instead of separate struct types?
/// - Enums are exhaustive: `match` ensures all cases are handled
/// - Single type for all errors: functions return `Result<T, CartError>`
/// - Pattern matching on error variants for different HTTP status codes
#[derive(Debug, Error)]
pub enum CartError {
    /// Cart not found for the given user.
    #[error("cart not found for user {user_id}")]
    CartNotFound { user_id: uuid::Uuid },

    /// Item not found in the cart.
    #[error("item with product_id {product_id} not found in cart")]
    ItemNotFound { product_id: String },

    /// Not enough stock for the requested quantity.
    #[error("insufficient stock: requested {requested}, available {available}")]
    InsufficientStock { requested: i32, available: i32 },

    /// Validation error — invalid request data.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// Authentication error — missing or invalid JWT.
    #[error("authentication error: {0}")]
    Unauthorized(String),

    /// Internal server error — something unexpected.
    #[error("internal error: {0}")]
    Internal(String),
}

// KEY LESSON: Mapping errors to HTTP responses
// ==============================================
// `impl IntoResponse for CartError` converts each error variant into an
// appropriate HTTP status code and JSON body. This is the Rust equivalent
// of exception-to-HTTP mapping in web frameworks.
//
// Why implement IntoResponse here instead of using common::AppError?
// Because CartError has cart-specific variants (InsufficientStock) that
// AppError doesn't know about. Each service owns its error-to-HTTP mapping.
//
// The `match` must be exhaustive — if we add a new variant to CartError,
// the compiler will tell us to add a case here. No runtime surprises.
impl IntoResponse for CartError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            CartError::CartNotFound { .. } => {
                let msg = self.to_string();
                (StatusCode::NOT_FOUND, msg)
            }
            CartError::ItemNotFound { .. } => {
                let msg = self.to_string();
                (StatusCode::NOT_FOUND, msg)
            }
            CartError::InsufficientStock { .. } => {
                let msg = self.to_string();
                (StatusCode::UNPROCESSABLE_ENTITY, msg)
            }
            CartError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            CartError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            CartError::Internal(_) => {
                error!(error = %self, "Internal cart service error");
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
