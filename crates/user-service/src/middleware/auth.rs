// Placeholder for JWT auth middleware — will be fleshed out in Step 5.

use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Placeholder middleware that passes through all requests.
/// In Step 5, this will validate JWTs and inject AuthUser into request extensions.
pub async fn auth_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    // TODO Step 5: Extract and validate JWT from Authorization header
    // TODO Step 5: On success, insert AuthUser into request.extensions()
    // TODO Step 5: On failure, return 401
    next.run(request).await
}