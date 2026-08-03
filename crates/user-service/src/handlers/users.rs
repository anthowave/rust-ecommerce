// =============================================================================
// Step 6: User Handlers — Get Me, Update Me, Get Public User
// =============================================================================

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use std::sync::Arc;
use uuid::Uuid;

use validator::Validate;

use crate::AppState;
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::user::{UpdateUserRequest, UserResponse, find_user_by_id, update_user};

/// GET /users/me
///
/// Returns the authenticated user's profile.
/// Requires a valid JWT access token.
///
/// # Rust Concept: Extension extractor
///
/// `Extension(auth_user): Extension<AuthUser>` extracts the `AuthUser` that
/// was inserted into request extensions by the auth middleware.
/// If the middleware didn't run (or didn't insert AuthUser), this extraction
/// would fail and Axum would return 500. That's why we only use this on
/// routes protected by the auth middleware.
///
/// **C++ analogy:** Getting data from a request context/thread-local storage.
/// **Go analogy:** `ctx.Value(authUserKey).(AuthUser)` — but Go requires
/// type assertion, while Rust is compile-time type-safe.
pub async fn get_me(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user = find_user_by_id(&state.pool, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "user",
            id: auth_user.user_id.to_string(),
        })?;

    Ok(Json(user.into()))
}

/// PUT /users/me
///
/// Updates the authenticated user's profile (name and/or email).
/// Only updates fields that are provided (partial update).
/// Requires a valid JWT access token.
pub async fn update_me(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    // Validate input if validation derive is available
    if let Err(e) = req.validate() {
        return Err(AppError::ValidationError(e.to_string()));
    }

    // Update in database
    let user = update_user(&state.pool, auth_user.user_id, &req).await?;

    Ok(Json(user.into()))
}

/// GET /users/:id
///
/// Returns a user's public profile by ID.
/// No authentication required — this is public information.
///
/// # Rust Concept: Path extractor
///
/// `Path(id): Path<Uuid>` extracts and parses a path parameter.
/// If the ID is not a valid UUID, Axum returns 400 automatically —
/// no manual validation needed.
///
/// **C++ analogy:** URL parameter parsing in a router.
/// **Go analogy:** `chi.URLParam(r, "id")` or `mux.Vars(r)["id"]`.
/// **Key difference:** Axum extracts AND validates the type at compile time.
/// If we change `id` from Uuid to i64, the compiler tells us everywhere
/// that needs updating.
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = find_user_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "user",
            id: id.to_string(),
        })?;

    Ok(Json(user.into()))
}
