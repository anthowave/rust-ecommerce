// Placeholder for router definition — will be fleshed out in Step 7.

use crate::AppState;
use axum::Router;
use std::sync::Arc;

/// Build the application router with all routes.
///
/// # Rust Concept: Tower Layers (Middleware Composition)
///
/// In Step 7, we'll use `tower::ServiceBuilder` to compose middleware layers:
/// - `TraceLayer` for request logging (already in product-service)
/// - JWT auth middleware on protected routes
/// - CORS configuration
///
/// **C++ analogy:** Composing decorators/filters around a service.
/// **Go analogy:** `http.NewServeMux` + middleware wrapping.
/// **Express.js analogy:** `app.use()` middleware chain.
///
/// The key Rust difference: layers are type-checked at compile time.
/// If a layer expects a certain request type and you give it the wrong
/// type, the compiler catches it — not a runtime panic.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new().with_state(state)
}
