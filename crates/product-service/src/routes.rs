// =============================================================================
// Phase 1, Step 9: Router — Wiring Routes to Handlers
// =============================================================================
// KEY LESSON: Axum Router
// =========================
// `axum::Router` is like Express's `app` or Go Gin's `router`.
// It's a tree of routes, each mapping an HTTP method + path to a handler function.
//
// The KEY DIFFERENCE from Express/Gin:
//   - Routes are TYPE-CHECKED at compile time. If a handler's signature doesn't
//     match the router's state type, you get a COMPILE ERROR.
//   - Nested routers: `Router::nest("/prefix", sub_router)` groups routes.
//   - Middleware is applied at the router level, with compile-time guarantees.
//
// COMPARISON:
//   Express:  app.get('/products', handler)           — untyped, runtime errors
//   Go Gin:   router.GET("/products", handler)        — untyped, runtime errors
//   Axum:     router.get("/products", get::<Handler>) — typed, compile-time safety

use axum::Router;
use axum::routing::{delete, get, post, put};
use std::sync::Arc;

use crate::handlers::products::{self, AppState};

/// Create the application router.
///
/// KEY LESSON: `Router::new().with_state(state)` pattern
/// ======================================================
/// `with_state(Arc<AppState>)` stores the shared state in the router.
/// Every handler that uses `State<Arc<AppState>>` gets it from here.
/// The type system ensures:
///   1. If a handler asks for `State<Arc<AppState>>`, the router MUST have `with_state(Arc<AppState>)`.
///   2. If the types don't match (e.g., `State<Foo>` vs `with_state(Bar)`), COMPILE ERROR.
///   3. You CAN'T accidentally add a handler that needs state to a router without it.
///
/// In Go/Express, you'd just panic at runtime if state is missing.
/// In Axum, you get a compile error before the server ever starts.
pub fn create_router(state: Arc<AppState>) -> Router {
    // KEY LESSON: Method routing
    // ===========================
    // `get(handler)` — matches GET requests
    // `post(handler)` — matches POST requests
    // `put(handler)` — matches PUT requests
    // `delete(handler)` — matches DELETE requests
    //
    // Path parameters: `/products/{id}` — like Express's `:id`, Gin's `:id`
    // Axum uses `{param}` syntax (similar to Rust's format strings)

    Router::new()
        // Health check — no auth needed, minimal overhead
        .route("/health", get(products::health_check))
        // CRUD routes for products
        // KEY LESSON: Route ordering matters! More specific routes before less specific.
        // `/products/{id}` must come AFTER `/products` (which is a prefix match).
        .route(
            "/products",
            get(products::list_products).post(products::create_product),
        )
        .route(
            "/products/{id}",
            get(products::get_product)
                .put(products::update_product)
                .delete(products::delete_product),
        )
        // KEY LESSON: `with_state()` — injects shared state into the router
        // This is where the Arc<AppState> is stored. All handlers that declare
        // `State<Arc<AppState>>` in their parameters get access to it.
        .with_state(state)
}
