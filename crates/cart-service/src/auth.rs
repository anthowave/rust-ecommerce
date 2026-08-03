// =============================================================================
// PHASE 3: Cart Service — JWT Auth Module
// =============================================================================
// The cart service needs to validate JWTs issued by the user-service.
// It uses the same `jsonwebtoken` crate but only validates tokens —
// it doesn't issue them (that's the user-service's job).
//
// KEY LESSON: Service boundaries and auth
// ========================================
// Each service independently validates JWTs using the same shared secret.
// This is "symmetric key" JWT — all services share the same secret.
// In production, you'd use asymmetric keys (RS256) where only the auth
// service has the private key, and other services have the public key.
//
// But for learning, symmetric HS256 is simpler and still teaches the concepts.

use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::config::Settings;
use crate::error::CartError;

/// JWT Claims for access tokens (same structure as user-service's Claims).
///
/// KEY LESSON: Why owned String, not &str?
/// =========================================
/// During deserialization, Claims is created FROM the token bytes.
/// If we used &str, the claims would BORROW from the token buffer,
/// which gets dropped after validation. Owned String avoids this.
/// This is the same pattern as user-service's Claims struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user ID (UUID as string)
    pub sub: String,
    /// User's role (for authorization checks)
    pub role: String,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
}

/// Validates a JWT token and returns the Claims.
///
/// This function:
/// 1. Verifies the HMAC-SHA256 signature using the JWT secret
/// 2. Checks the expiration time (exp claim) auto-validated by jsonwebtoken
/// 3. Returns parsed Claims if valid
///
/// This is essentially the same function as user-service's `validate_token`,
/// but adapted for cart-service's own Settings and error types.
///
/// # Why duplicate this code instead of sharing via common crate?
///
/// Good question! Each service has its own Settings struct, and the
/// validate_token function depends on `&Settings`. We could move Claims
/// and validate_token to the common crate, but then common would need
/// to know about jsonwebtoken and the Settings struct.
///
/// For now (Phase 3), keeping it per-service is simpler. If we had 5+
/// services, we'd refactor this into common. This is the "Rule of Three"
/// — don't abstract until you've duplicated at least 3 times.
pub fn validate_token(token: &str, settings: &Settings) -> Result<Claims, CartError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                tracing::debug!("Token has expired");
            }
            _ => {
                tracing::warn!(error = %e, "Token validation failed");
            }
        }
        CartError::Unauthorized("Invalid or expired token".to_string())
    })?;

    Ok(token_data.claims)
}
