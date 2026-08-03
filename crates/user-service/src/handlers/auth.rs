// =============================================================================
// Step 6: Auth Handlers — Register, Login, Refresh, Logout
// =============================================================================

use axum::{Extension, Json, extract::State};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use crate::auth::{
    create_access_token, create_refresh_token, hash_password, hash_token, validate_token,
    verify_password,
};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::models::user::{
    AuthResponse, CreateUserRequest, LoginRequest, RefreshTokenRequest, UserResponse, create_user,
    find_refresh_token, find_user_by_email, revoke_user_tokens, store_refresh_token,
};

/// POST /auth/register
///
/// Creates a new user account. Validates input, hashes password with Argon2,
/// stores in database, and returns JWT tokens.
///
/// # Rust Concept: validator crate
///
/// `req.validate()` returns `Result<(), ValidationErrors>`. We map validation
/// errors to `AppError::ValidationError` for consistent error responses.
/// This is like Go's `validator.Struct(req)` or Joi/Zod in JavaScript.
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Validate input
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Hash password with Argon2 (salt auto-generated)
    let password_hash =
        hash_password(&req.password).map_err(|e| AppError::Unauthorized(e.to_string()))?;

    // Create user in database
    let user = create_user(&state.pool, &req.email, &password_hash, &req.name).await?;

    // Generate tokens
    let access_token =
        create_access_token(user.id.to_string(), user.role.to_string(), &state.settings)
            .map_err(|e| AppError::Unauthorized(e.to_string()))?;
    let refresh_token = create_refresh_token(user.id.to_string(), &state.settings)
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;

    // Store refresh token hash in database
    let token_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + chrono::Duration::days(state.settings.refresh_token_expiry_days);
    store_refresh_token(&state.pool, user.id, &token_hash, expires_at).await?;

    let user_response: UserResponse = user.into();

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.settings.access_token_expiry_minutes * 60,
        user: user_response,
    }))
}

/// POST /auth/login
///
/// Authenticates a user with email and password.
/// Returns JWT tokens on success, 401 on failure.
///
/// # Security Note
///
/// Always return the same error for both "user not found" and "invalid password".
/// This prevents user enumeration attacks where attackers can determine which
/// emails are registered by observing different error messages.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Find user by email
    let user = find_user_by_email(&state.pool, &req.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    let is_valid = verify_password(&req.password, &user.password_hash)
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;

    if !is_valid {
        return Err(AppError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    // Generate tokens
    let access_token =
        create_access_token(user.id.to_string(), user.role.to_string(), &state.settings)
            .map_err(|e| AppError::Unauthorized(e.to_string()))?;
    let refresh_token = create_refresh_token(user.id.to_string(), &state.settings)
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;

    // Store refresh token hash
    let token_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + chrono::Duration::days(state.settings.refresh_token_expiry_days);
    store_refresh_token(&state.pool, user.id, &token_hash, expires_at).await?;

    let user_response: UserResponse = user.into();

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.settings.access_token_expiry_minutes * 60,
        user: user_response,
    }))
}

/// POST /auth/refresh
///
/// Exchanges a refresh token for a new access token.
/// The refresh token is rotated: old one is revoked, new one is issued.
///
/// # Refresh Token Rotation
///
/// We implement refresh token rotation for security:
/// 1. Validate the old refresh token
/// 2. Issue a new access token AND a new refresh token
/// 3. The old refresh token is implicitly invalidated (new one replaces it)
///
/// This limits the damage if a refresh token is stolen: the legitimate user
/// will notice when their refresh token stops working, and the attacker
/// can only use the stolen token until the next refresh.
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Hash the incoming refresh token to look it up in DB
    let token_hash = hash_token(&req.refresh_token);

    // Find the stored token
    let (user_id, _expires_at) = find_refresh_token(&state.pool, &token_hash)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired refresh token".to_string()))?;

    // Validate the JWT itself
    let claims = validate_token(&req.refresh_token, &state.settings)
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;

    // Parse user ID from claims
    let claim_user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid token claims".to_string()))?;

    // Security: ensure the token's user_id matches the DB lookup
    if claim_user_id != user_id {
        return Err(AppError::Unauthorized("Token user mismatch".to_string()));
    }

    // Find the user (to get their current role)
    let user = crate::models::user::find_user_by_id(&state.pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "user",
            id: user_id.to_string(),
        })?;

    // Generate new tokens
    let access_token =
        create_access_token(user.id.to_string(), user.role.to_string(), &state.settings)
            .map_err(|e| AppError::Unauthorized(e.to_string()))?;
    let new_refresh_token = create_refresh_token(user.id.to_string(), &state.settings)
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;

    // Store new refresh token hash
    let new_token_hash = hash_token(&new_refresh_token);
    let expires_at = Utc::now() + chrono::Duration::days(state.settings.refresh_token_expiry_days);
    store_refresh_token(&state.pool, user_id, &new_token_hash, expires_at).await?;

    // Revoke the old refresh token (rotation)
    // We revoke only this specific token, not all tokens
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

    let user_response: UserResponse = user.into();

    Ok(Json(AuthResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.settings.access_token_expiry_minutes * 60,
        user: user_response,
    }))
}

/// POST /auth/logout
///
/// Revokes ALL refresh tokens for the authenticated user.
/// Requires a valid access token (auth middleware).
///
/// After logout, the user must log in again to get new tokens.
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<serde_json::Value>, AppError> {
    revoke_user_tokens(&state.pool, auth_user.user_id).await?;

    Ok(Json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}
