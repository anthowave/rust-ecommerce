// =============================================================================
// Phase 1, Step 4: Error Handling
// =============================================================================
// The IntoResponse implementation for AppError is now in the `common` crate
// (crates/common/src/error.rs). This is required by Rust's ORPHAN RULE:
// you must implement a trait in the same crate as EITHER the trait OR the type.
// Since AppError is defined in `common`, the IntoResponse impl lives there.
//
// This module exists so the `mod error` declaration in main.rs still works.
// All error-related code is centralized in common/src/error.rs.

// Re-export AppError from common so handlers can use it
pub use common::error::AppError;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    // Helper to extract the response body as a string
    async fn body_as_string(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_not_found_error_response() {
        let err = AppError::NotFound {
            entity: "Product",
            id: "123".to_string(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        let body = body_as_string(response).await;
        assert!(body.contains("Product"));
        assert!(body.contains("123"));
    }

    #[tokio::test]
    async fn test_validation_error_response() {
        let err = AppError::ValidationError("Name is required".to_string());
        let response = err.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        let body = body_as_string(response).await;
        assert!(body.contains("Name is required"));
    }
}
