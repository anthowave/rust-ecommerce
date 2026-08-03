// Placeholder for User model — will be fleshed out in Step 3.
// This stub lets the crate compile so we can verify the scaffold.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a user row from the database.
///
/// # Rust Concept: sqlx::FromRow
///
/// The derive macro generates code to map Postgres rows to this struct.
/// The field names must match the SQL column names (or use #[sqlx(rename = "...")]).
/// This is compile-time ORM — no reflection, no runtime mapping overhead.
///
/// **C++ analogy:** An ORM that generates C++ structs from your schema at build time.
/// **Go analogy:** sqlx's `StructScan` (jmoiron/sqlx), but Go does it at runtime via
/// struct tags and reflection. Rust does it at compile time via derive macros.
/// **Key advantage:** If you rename a column in your migration but forget to update
/// the struct, Rust gives you a compile error. Go would give you a runtime error.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String, // NEVER serialize this! We'll customize in Step 3.
    pub name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Public representation of a user (excludes sensitive fields).
/// This is what we return in API responses.
///
/// # Rust Concept: DTO (Data Transfer Object) pattern
///
/// We separate the DB model from the API response model. This prevents
/// accidentally serializing password_hash or internal fields. The conversion
/// from User to UserResponse is explicit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

/// Request payload for user registration.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String, // Plaintext from client — we hash it before storing
    pub name: String,
}

/// Request payload for login.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Request payload for updating user profile.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub name: Option<String>,
}
