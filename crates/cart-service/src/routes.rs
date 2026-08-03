// =============================================================================
// PHASE 3: Cart Service Routes
// =============================================================================
// KEY LESSON: Axum Router with middleware layered on routes
// ==========================================================
// Axum's Router allows granular middleware application:
//   - `.route_layer()` applies middleware to ALL routes on a router
//   - `.merge()` combines routers (one with auth, one without)
//   - Method chaining: `.get().post().put().delete()`
//
// **Express.js analogy:** `router.use(authMiddleware)`
// **Go analogy:** Grouping routes with middleware in chi/mux

use axum::{Router, middleware, routing::get};
use std::sync::Arc;

use crate::AppState;
use crate::handlers;
use crate::middleware::auth::auth_middleware;

/// Build the cart service router.
///
/// All cart routes require JWT authentication — a user can only access
/// their own cart. The auth middleware extracts the user ID from the JWT
/// and injects it into request extensions.
///
/// KEY LESSON: `middleware::from_fn_with_state`
/// ==============================================
/// `from_fn_with_state` creates a Tower Layer from an async function that
/// needs access to shared state (the JWT secret lives in AppState).
/// The middleware function receives `State<Arc<AppState>>` just like handlers.
///
/// KEY LESSON: Handler state vs middleware state
/// ==============================================
/// Both handlers and middleware receive `State<Arc<AppState>>` — they share
/// the same state. Handlers extract what they need (CartStore), middleware
/// extracts what it needs (Settings.jwt_secret). The compiler ensures the
/// state type matches.
pub fn create_router(state: Arc<AppState>) -> Router {
    // Authenticated routes — all cart operations require a valid JWT
    let protected_routes = Router::new()
        .route(
            "/cart",
            get(handlers::cart::get_cart).delete(handlers::cart::clear_cart),
        )
        .route("/cart/items", axum::routing::post(handlers::cart::add_item))
        .route(
            "/cart/items/{product_id}",
            axum::routing::put(handlers::cart::update_quantity).delete(handlers::cart::remove_item),
        )
        // KEY LESSON: route_layer applies middleware to this sub-router only
        // This is more precise than applying it at the top level — only
        // the routes that actually need auth get the middleware.
        // Note: from_fn_with_state is given state separately so the middleware
        // can extract State<Arc<AppState>> even before with_state is called.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Health check — no auth required
    let public_routes = Router::new().route("/health", get(health_check));

    // Merge both routers and provide shared state
    Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state)
}

/// GET /health — Health check endpoint (no auth required).
async fn health_check() -> &'static str {
    "OK"
}
