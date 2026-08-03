// =============================================================================
// PHASE 3: Cart Service — JWT Auth Middleware
// =============================================================================
// Same pattern as user-service, adapted for cart-service's AppState and error types.
//
// KEY LESSON: Middleware reusability via Tower Layers
// ====================================================
// The auth middleware follows the same Tower Layer pattern regardless of service.
// The core logic (extract Bearer token, validate JWT, inject AuthUser) is identical.
// Only the State type and error type differ — everything else is the same.
//
// This demonstrates Rust's type system: the middleware is TYPE-SAFE because
// it declares exactly what State it needs. If we tried to use cart-service's
// middleware in user-service, it wouldn't compile — the State type differs.

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

/// Information about the authenticated user, injected into request extensions.
///
/// KEY LESSON: Request Extensions — type-safe per-request storage
/// ================================================================
/// Axum's Request has an `extensions()` method that returns a type-map.
/// You insert values by TYPE (e.g., `request.extensions_mut().insert(AuthUser{...})`)
/// and extract by TYPE (e.g., `Extension(auth_user): Extension<AuthUser>`).
///
/// This is like Go's `context.WithValue(ctx, key, value)` but TYPE-SAFE.
/// In Go, the key is an untyped `interface{}` — you can accidentally use the
/// wrong key type. In Rust, the compiler guarantees you extract the right type.
///
/// **C++ analogy:** `boost::any` cast back to the exact type.
/// **Go analogy:** `context.WithValue` with typed keys.
/// **Rust benefit:** Compile-time type safety — no runtime type assertions.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
}

/// The JWT auth middleware function.
///
/// For every request passing through this middleware:
/// 1. Extract the `Authorization: Bearer <token>` header
/// 2. Validate the JWT token using the shared secret
/// 3. On success: insert `AuthUser` into request extensions, pass to handler
/// 4. On failure: return 401 Unauthorized JSON response
///
/// # Key Pattern
///
/// This function uses `State(state): State<Arc<AppState>>` — Axum's extractor
/// for shared application state. The middleware extracts what it needs (JWT secret)
/// without knowing about the cart store — separation of concerns.
///
/// # How it connects to handlers
///
/// After this middleware runs, handlers can access `AuthUser` via:
/// ```
/// use axum::Extension;
/// async fn my_handler(Extension(auth_user): Extension<AuthUser>) -> impl IntoResponse {
///     // auth_user.user_id is a Uuid
///     // auth_user.role is a String
/// }
/// ```
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract the Authorization header and strip the "Bearer " prefix
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            // Validate the JWT token against our shared secret
            match validate_token(token, &state.settings) {
                Ok(claims) => {
                    // Parse the user ID from claims.sub (which is a UUID string)
                    match Uuid::parse_str(&claims.sub) {
                        Ok(user_id) => {
                            // KEY LESSON: Type-safe request extensions
                            // Insert AuthUser into the request's type-map.
                            // Handlers extract it via `Extension<AuthUser>`.
                            // The compiler ensures only AuthUser can be extracted
                            // at this type — no accidental type mismatches.
                            request.extensions_mut().insert(AuthUser {
                                user_id,
                                role: claims.role,
                            });

                            // Pass the modified request to the next layer/handler
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