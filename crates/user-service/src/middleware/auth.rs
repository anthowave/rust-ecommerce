// =============================================================================
// Step 5: JWT Authentication Middleware
// =============================================================================
//
// # Rust Concept: Tower Layers (Middleware)
//
// A Tower Layer wraps a service. When a request arrives:
// 1. The middleware runs BEFORE the handler
// 2. It can modify the request (add extensions)
// 3. It can short-circuit (return 401 without calling the handler)
// 4. It passes control to the next layer/handler via `next.run(request).await`
//
// **C++ analogy:** Decorator pattern — wrapping a service with filters.
// **Go analogy:** `func(http.Handler) http.Handler` middleware pattern.
// **Express.js analogy:** `app.use((req, res, next) => { ... })`
//
// **Key Rust difference: TYPE SAFETY.** The middleware signature tells you
// exactly what it does. Request extensions are type-safe — you can't
// accidentally insert the wrong type into extensions.
//
// # Rust Concept: Request Extensions
//
// `request.extensions()` is a type-map — like a `HashMap<TypeId, Box<dyn Any>>`.
// You insert values by type and extract by type. If you insert `AuthUser`,
// you extract `.extract::<AuthUser>()`. This is type-safe: the compiler
// ensures you can't get a `String` where you inserted an `AuthUser`.
//
// **C++ analogy:** `boost::any` map keyed by `typeid`.
// **Go analogy:** `context.WithValue(ctx, key, value)` — but Go uses
// untyped interface keys. Rust's approach is compile-time safe.

use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::auth::validate_token;

/// Information about the authenticated user, injected into request extensions
/// by the auth middleware.
///
/// Handlers can extract this with:
/// ```
/// use axum::Extension;
/// async fn handler(Extension(auth_user): Extension<AuthUser>) { ... }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

/// The JWT auth middleware function.
///
/// For every request passing through this middleware:
/// 1. Extract the `Authorization: Bearer <token>` header
/// 2. Validate the JWT token
/// 3. On success: insert `AuthUser` into request extensions, pass to handler
/// 4. On failure: return 401 Unauthorized
///
/// # How it connects to handlers:
///
/// After this middleware runs, handlers can access `AuthUser` via:
/// ```
/// use axum::Extension;
/// async fn my_handler(Extension(auth_user): Extension<AuthUser>) -> impl IntoResponse {
///     // auth_user.user_id and auth_user.role are available
/// }
/// ```
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract the Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            // Validate the JWT token
            match validate_token(token, &state.settings) {
                Ok(claims) => {
                    // Parse the user ID from claims
                    match Uuid::parse_str(&claims.sub) {
                        Ok(user_id) => {
                            // Inject AuthUser into request extensions
                            // This is TYPE-SAFE: only AuthUser can be extracted at this type
                            request.extensions_mut().insert(AuthUser {
                                user_id,
                                role: claims.role,
                            });

                            // Pass to the next layer/handler
                            next.run(request).await
                        }
                        Err(_) => unauthorized_response("Invalid user ID in token"),
                    }
                }
                Err(_) => unauthorized_response("Invalid or expired token"),
            }
        }
        None => unauthorized_response("Missing Authorization header"),
    }
}

/// Build a 401 Unauthorized JSON response.
fn unauthorized_response(message: &str) -> Response {
    let body = json!({
        "error": {
            "message": message,
            "status": 401,
        }
    });

    (StatusCode::UNAUTHORIZED, Json(body)).into_response()
}
