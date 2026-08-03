// =============================================================================
// Step 7: Routes & Router Composition
// =============================================================================
//
// # Rust Concept: Router Composition with Tower Layers
//
// Axum's Router uses a composable pattern:
// - `.route(path, handler)` — registers a route
// - `.route_layer(layer)` — applies middleware to ALL routes in that router
// - `.merge(other_router)` — combines two routers
// - `.with_state(state)` — attaches shared application state
//
// This is where we see the POWER of Rust's type system:
// - The router type encodes which state it holds
// - Middleware layers are type-checked for compatibility
// - Compile-time guarantees that no handler gets wrong state
//
// **C++ analogy:** A template-based routing system where routes are
// type-checked at compile time.
// **Go analogy:** `http.NewServeMux` + middleware wrapping — but Go
// doesn't type-check state or middleware compatibility.
// **Express.js analogy:** `app.use()` + `app.get/post()` — but
// Express doesn't type-check handler signatures.

use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::AppState;
use crate::handlers::auth;
use crate::handlers::users;
use crate::middleware::auth::auth_middleware;

/// Build the application router with all routes and middleware.
///
/// Route structure:
/// ```
/// PUBLIC (no auth):
///   POST /auth/register    — Create account
///   POST /auth/login       — Sign in
///   POST /auth/refresh     — Get new access token
///
/// PROTECTED (requires valid JWT):
///   POST /auth/logout      — Sign out (revoke tokens)
///   GET  /users/me         — Get my profile
///   PUT  /users/me         — Update my profile
///
/// PUBLIC:
///   GET  /users/:id        — Get public user profile
///   GET  /health           — Health check
/// ```
pub fn create_router(state: Arc<AppState>) -> Router {
    // Middleware for logging all requests
    // TraceLayer adds request/response logging with tracing spans
    let trace_layer = TraceLayer::new_for_http();

    // Protected routes — all need a valid JWT
    //
    // # Rust Concept: middleware::from_fn_with_state
    //
    // `from_fn_with_state` creates a Tower Layer from our auth middleware function.
    // Unlike `from_fn` (no state), this variant passes the AppState to the middleware.
    // This is necessary because our middleware needs access to JWT settings (secret, etc.)
    // which are stored in AppState.
    //
    // The layer is applied to all routes in the router via `.route_layer()`.
    // Any route under this router will first pass through the auth middleware.
    let protected_routes = Router::new()
        .route("/auth/logout", post(auth::logout))
        .route("/users/me", get(users::get_me))
        .route("/users/me", put(users::update_me))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Public routes — no authentication required
    let public_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh))
        .route("/users/{id}", get(users::get_user))
        .route("/health", get(health_check));

    // Merge all routers and apply global middleware
    //
    // # Rust Concept: Router.merge()
    //
    // `merge` combines two routers. The order matters:
    // - If two routes conflict, the one merged LAST wins
    // - State must be the same type for both routers
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(trace_layer)
        .with_state(state)
}

/// Health check endpoint — used by load balancers and monitoring.
///
/// Returns: `{"status": "ok", "service": "user-service"}`
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "user-service"
    }))
}
