use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use crate::error::AppError;

// =============================================================================
// Step 3: User Model, Postgres Enum Mapping & async-trait Repository
// =============================================================================

/// Maps to the Postgres `user_role` enum defined in our migration.
///
/// # Rust Concept: sqlx::Type for Postgres enums
///
/// `#[derive(sqlx::Type)]` + `#[sqlx(type_name = "user_role")]` tells SQLx
/// to serialize/deserialize this Rust enum to/from a Postgres enum type.
/// This is TYPE-SAFE at the database boundary: if you try to insert a role
/// value that doesn't exist in the Postgres enum, you get a compile error.
///
/// **IMPORTANT:** The Postgres enum type must EXIST before you can derive
/// `sqlx::Type` for it. We created `user_role` in our migration (Step 1).
///
/// **C++ analogy:** An enum that maps to a database CHECK constraint or
/// enum type via an ORM.
/// **Go analogy:** A string type with validation — but Go doesn't have
/// compile-time DB enum checking.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    User,
    Admin,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::User => write!(f, "user"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}

// =============================================================================
// User model — maps to the `users` table
// =============================================================================

/// Database row representation. Derives `FromRow` for SQLx row mapping.
///
/// # Rust Concept: Selective serialization
///
/// `#[serde(skip)]` on `password_hash` ensures it's NEVER included in API
/// responses, even if someone accidentally derives Serialize on User.
/// This is defense-in-depth: we primarily use `UserResponse` for API output,
/// but the skip annotation prevents catastrophic mistakes.
///
/// **C++ analogy:** Marking a field `private` and never exposing it.
/// **Go analogy:** Using `json:"-"` struct tags. Same idea.
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String, // NEVER serialized to clients
    pub name: String,
    #[sqlx(try_from = "UserRole")]
    pub role: UserRole, // Postgres enum → Rust enum
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// DTOs (Data Transfer Objects) — API request/response types
// =============================================================================

/// Public user representation (never contains password_hash).
///
/// # Rust Concept: DTO pattern — separate DB model from API model
///
/// Unlike Go where you might use the same struct with `json:"-"` tags,
/// in Rust we create a separate type. This is more explicit and prevents
/// accidentally leaking internal fields. The `From<User>` implementation
/// makes conversion ergonomic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

/// # Rust Concept: From trait for clean conversions
///
/// `impl From<User> for UserResponse` lets you write:
/// `let response: UserResponse = user.into();`
/// or `let response = UserResponse::from(user);`
///
/// **C++ analogy:** Conversion constructor or `operator T()`.
/// **Go analogy:** `func NewUserResponse(u User) UserResponse`.
/// **Key difference:** `From<T>` is part of the stdlib and integrates
/// with `.into()` and `?` operator for automatic error conversion.
impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

/// Request to register a new user.
///
/// # Rust Concept: validator crate (input validation)
///
/// `validator` adds declarative validation rules via derive macros.
/// Like `go-playground/validator` in Go or Joi/Zod in JavaScript.
/// The validation is explicit and composable — you can call `.validate()`
/// on any struct that derives `Validate`.
#[derive(Debug, Clone, Deserialize, validator::Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String, // Plaintext from client — hashed before DB insert

    #[validate(length(min = 1, max = 255, message = "Name is required"))]
    pub name: String,
}

/// Request to log in.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Request to update a user profile.
/// All fields are Option — only send what you want to change.
#[derive(Debug, Clone, Deserialize, validator::Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,

    #[validate(length(min = 1, max = 255, message = "Name cannot be empty"))]
    pub name: Option<String>,
}

/// Response for login/refresh — contains JWT tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String, // "Bearer"
    pub expires_in: i64,    // seconds until access token expires
    pub user: UserResponse,
}

/// Refresh token request.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

// =============================================================================
// Database Query Functions
// =============================================================================
// These follow the same patterns as Phase 1's product model but add:
// - Argon2 password verification integration (Step 4)
// - Soft delete queries
// - Unique constraint handling

/// Create a new user in the database.
///
/// Returns `AuthError::EmailAlreadyExists` if the email is already taken.
/// Uses `sqlx::query_as` to map the INSERT result back to a User struct
/// via the `RETURNING *` clause.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> Result<User, AppError> {
    let user = sqlx::query_as::<Postgres, User>(
        r#"INSERT INTO users (email, password_hash, name)
           VALUES ($1, $2, $3)
           RETURNING *"#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        // Check for unique constraint violation on email
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("users_email_key") {
                return AppError::ValidationError(format!("Email '{}' already registered", email));
            }
        }
        AppError::DatabaseError(e)
    })?;

    Ok(user)
}

/// Find a user by email (for login).
///
/// Returns `None` if no user found, or `Some(User)`.
/// Only returns non-deleted users (deleted_at IS NULL).
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<Postgres, User>(
        "SELECT * FROM users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e))?;

    Ok(user)
}

/// Find a user by ID.
pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<Postgres, User>(
        "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e))?;

    Ok(user)
}

/// Update a user's profile fields (email and/or name).
///
/// Uses a dynamic query builder pattern — only updates fields that are Some.
/// Returns the updated user.
///
/// # Rust Concept: QueryBuilder for dynamic SQL
///
/// Instead of writing multiple static queries for every combination of fields,
/// we build the query dynamically. `QueryBuilder` prevents SQL injection by
/// using parameterized queries (not string concatenation).
///
/// **C++ analogy:** Building a SQL string with parameter binding.
/// **Go analogy:** `squirrel` query builder or raw fmt.Sprintf with args.
/// **Key difference:** QueryBuilder is type-checked at compile time —
/// you can't accidentally bind a wrong-typed value.
pub async fn update_user(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateUserRequest,
) -> Result<User, AppError> {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::<Postgres>::new("UPDATE users SET updated_at = NOW()");

    if req.email.is_some() {
        builder.push(", email = ");
        builder.push_bind(req.email.as_deref());
    }
    if req.name.is_some() {
        builder.push(", name = ");
        builder.push_bind(req.name.as_deref());
    }

    builder.push(" WHERE id = ");
    builder.push_bind(id);
    builder.push(" AND deleted_at IS NULL RETURNING *");

    let user = builder
        .build_query_as::<User>()
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

    Ok(user)
}

/// Soft-delete a user (sets deleted_at = NOW()).
///
/// We never hard-delete users — this preserves data integrity
/// (orders, payments, etc. still reference the user).
pub async fn soft_delete_user(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

    Ok(())
}

// =============================================================================
// Refresh Token Database Operations
// =============================================================================

/// Store a refresh token in the database.
///
/// We store the SHA-256 HASH of the token, not the raw token.
/// This is the same principle as password hashing: if the DB is compromised,
/// the attacker can't steal valid refresh tokens.
pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
           VALUES ($1, $2, $3)"#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e))?;

    Ok(())
}

/// Find a non-revoked, non-expired refresh token by its hash.
pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<(Uuid, DateTime<Utc>)>, AppError> {
    let result = sqlx::query_as::<Postgres, (Uuid, DateTime<Utc>)>(
        r#"SELECT user_id, expires_at FROM refresh_tokens
           WHERE token_hash = $1
             AND revoked = FALSE
             AND expires_at > NOW()"#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DatabaseError(e))?;

    Ok(result)
}

/// Revoke all refresh tokens for a user (used on logout).
pub async fn revoke_user_tokens(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(e))?;

    Ok(())
}
