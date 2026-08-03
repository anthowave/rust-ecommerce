// =============================================================================
// Step 4: Auth Module — Argon2 Password Hashing, JWT Encoding/Decoding
// =============================================================================
//
// New Rust Concepts:
//
// 1. Lifetimes — The Claims struct uses owned String fields (not &str with
//    lifetimes) because the claims data must outlive the encoding/decoding
//    function calls. If we used &str, the borrow checker would require
//    lifetime annotations tying the Claims lifetime to the token string.
//
// 2. Clone vs Copy — Passwords and tokens should be Clone but NOT Copy.
//    You don't want to accidentally duplicate credentials via implicit copies.
//    Rust makes this explicit: you must call .clone().
//
// 3. impl Into<String> — Some functions accept impl Into<String> so callers
//    can pass either &str or String. This flexibility is a common Rust pattern.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Settings;
use crate::error::AuthError;

/// Hashes a plaintext password using Argon2.
///
/// # Rust Concept: Argon2 — the modern password hashing algorithm
///
/// Argon2 won the Password Hashing Competition in 2015. It has three variants:
/// - Argon2i — resistant to side-channel attacks
/// - Argon2d — resistant to GPU/ASIC attacks (maximizes memory hardness)
/// - Argon2id — hybrid (recommended default)
///
/// **Why Argon2 over bcrypt/scrypt?**
/// - Argon2 is memory-hard (configurable memory usage), not just CPU-hard
/// - bcrypt is limited to 4KB memory — easy for GPUs/ASICs
/// - scrypt is memory-hard but older and less analyzed
/// - Argon2 was designed specifically to resist GPU/ASIC/FPGA attacks
///
/// The salt is generated using OsRng (cryptographically secure) and
/// embedded in the hash string (PHC format: $argon2id$v=19$m=...$t=...$p=...$salt$hash)
///
/// **C++ analogy:** libsodium's crypto_pwhash_str.
/// **Go analogy:** golang.org/x/crypto/argon2.
///
/// # Errors
///
/// Returns `AuthError::InvalidCredentials` if hashing fails (shouldn't happen
/// with valid inputs, but Argon2 can fail on extreme parameters).
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            tracing::error!(error = %e, "Password hashing failed");
            AuthError::InvalidCredentials
        })?;

    Ok(password_hash.to_string())
}

/// Verifies a plaintext password against an Argon2 hash.
///
/// # Rust Concept: PasswordVerifier trait
///
/// The Argon2 instance + PasswordVerifier trait + the PHC format hash string
/// lets us verify passwords without knowing the original salt or parameters —
/// they're all encoded in the hash string. This is the standard PHC format
/// (Password Hashing Competition format).
///
/// # Security Note
///
/// We return a generic `AuthError::InvalidCredentials` whether the password
/// is wrong OR the user doesn't exist. This prevents user enumeration attacks
/// (timing attacks that reveal which emails are registered). The caller
/// should do the same — always return the same error message for both cases.
///
/// **C++ analogy:** `crypto_pwhash_str_verify` in libsodium.
/// **Go analogy:** `argon2.ComparePasswordAndHash` in some Go libraries.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    // Parse the hash string (extracts salt, params, hash from PHC format)
    let parsed_hash = PasswordHash::new(hash).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse password hash");
        AuthError::InvalidCredentials
    })?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// =============================================================================
// JWT Claims — the payload of our tokens
// =============================================================================

/// JWT Claims for access tokens.
///
/// # Rust Concept: Owned String vs &str with Lifetimes
///
/// Notice this struct uses `String` (owned) rather than `&'a str` (borrowed).
/// Why? Because Claims needs to be both serialized AND deserialized:
/// - For encoding: `sub` is created from a Uuid → String conversion
/// - For decoding: `sub` is deserialized from the JWT payload
///
/// If we used `&str`, we'd need lifetime annotations (`struct Claims<'a> { sub: &'a str }`)
/// and the deserialized Claims would borrow from the token byte buffer — but
/// that buffer gets dropped after decoding. Using owned String avoids this.
///
/// **C++ analogy:** `std::string` vs `std::string_view` — `string_view` is
/// only valid while the original string lives. Same issue here.
/// **Go analogy:** Go strings are always GC-managed, so this distinction
/// doesn't exist. Rust makes the ownership explicit.
///
/// **Rule of thumb:** Store `String` in structs, accept `&str` in function
/// parameters. This is the Rust equivalent of "use value semantics for
/// storage, reference semantics for parameters."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user ID (UUID as string)
    pub sub: String,
    /// User's role (for authorization checks)
    pub role: String,
    /// Issued at (UTC timestamp)
    pub iat: usize,
    /// Expiration time (UTC timestamp)
    pub exp: usize,
}

/// Creates an access token JWT for a user.
///
/// Access tokens are short-lived (default 15 minutes).
/// They're sent with every authenticated request in the Authorization header.
///
/// # Rust Concept: impl Into<String> for flexible parameters
///
/// `user_id: impl Into<String>` allows callers to pass either:
/// - `&str` — `create_access_token("some-id", "user", &settings)`
/// - `String` — `create_access_token(owned_string, "admin", &settings)`
///
/// This is more flexible than requiring a specific type. It's like C++
/// function overloading or Go's implicit interface satisfaction, but
/// Rust does it through traits at compile time with zero runtime overhead.
///
/// **C++ analogy:** Template function accepting anything convertible to string.
/// **Go analogy:** Accepting `interface{}` and type-switching (but that's runtime).
/// **Key difference:** The conversion is resolved at compile time, zero overhead.
pub fn create_access_token(
    user_id: impl Into<String>,
    role: impl Into<String>,
    settings: &Settings,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let expiry = now + Duration::minutes(settings.access_token_expiry_minutes);

    let claims = Claims {
        sub: user_id.into(),
        role: role.into(),
        iat: now.timestamp() as usize,
        exp: expiry.timestamp() as usize,
    };

    encode(
        &Header::default(), // HS256 (default algorithm)
        &claims,
        &EncodingKey::from_secret(settings.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create access token");
        AuthError::InvalidToken
    })
}

/// Creates a refresh token JWT for a user.
///
/// Refresh tokens are long-lived (default 7 days).
/// They're used ONLY to get new access tokens — never for API authorization.
///
/// The refresh token is sent to the client once (on login/refresh) and
/// stored securely (httpOnly cookie or secure storage).
pub fn create_refresh_token(
    user_id: impl Into<String>,
    settings: &Settings,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let expiry = now + Duration::days(settings.refresh_token_expiry_days);

    let claims = Claims {
        sub: user_id.into(),
        role: "refresh".to_string(), // Refresh tokens don't carry role
        iat: now.timestamp() as usize,
        exp: expiry.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create refresh token");
        AuthError::InvalidToken
    })
}

/// Decodes and validates a JWT token, returning the Claims.
///
/// This function:
/// 1. Verifies the signature using the JWT secret
/// 2. Checks the expiration time (exp claim)
/// 3. Returns parsed Claims if valid
///
/// Used by the auth middleware (Step 5) and the refresh handler (Step 6).
///
/// # Security Note
///
/// We use `Validation::default()` which:
/// - Requires `exp` claim
/// - Validates that `exp` is not in the past
/// - Does NOT validate `iat` or `nbf` by default
pub fn validate_token(token: &str, settings: &Settings) -> Result<Claims, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        // jsonwebtoken errors distinguish between expired and invalid
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                tracing::debug!("Token has expired");
            }
            _ => {
                tracing::warn!(error = %e, "Token validation failed");
            }
        }
        AuthError::InvalidToken
    })?;

    Ok(token_data.claims)
}

/// Computes the SHA-256 hash of a token string.
///
/// We store the HASH of refresh tokens in the database, not the raw token.
/// This is the same principle as password hashing: if the database is
/// compromised, the attacker cannot use the stored hashes as valid tokens.
///
/// # Rust Concept: sha2 crate with Digest trait
///
/// The `Digest` trait provides a uniform interface for hash functions.
/// `Sha256::new().chain_update(data).finalize()` produces a 32-byte hash.
/// We format it as hex for storage.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that hashing and verifying a password works end-to-end.
    ///
    /// # Rust Concept: Unit tests with #[cfg(test)]
    ///
    /// Rust unit tests live in the same file as the code they test,
    /// inside a `#[cfg(test)]` module. This keeps tests close to the
    /// implementation. `cargo test` compiles and runs them.
    ///
    /// **C++ analogy:** Google Test, Catch2 (separate files or inline).
    /// **Go analogy:** `_test.go` files in the same package.
    /// **Key difference:** Rust tests are compiled only in test mode,
    /// so they have zero impact on binary size/performance.
    #[test]
    fn test_password_hashing_roundtrip() {
        let password = "MySecureP@ssw0rd!";
        let hash = hash_password(password).expect("Hashing should succeed");

        // Hash should be different from password
        assert_ne!(hash, password);
        // Hash should contain the Argon2 identifier
        assert!(hash.starts_with("$argon2"));

        // Verify correct password
        let valid = verify_password(password, &hash).expect("Verification should succeed");
        assert!(valid);

        // Verify wrong password
        let invalid = verify_password("WrongPassword", &hash).expect("Verification should succeed");
        assert!(!invalid);
    }

    /// Test that hashing the same password twice produces different hashes
    /// (because of random salt).
    #[test]
    fn test_password_hashing_produces_different_hashes() {
        let password = "SamePassword";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password, different salts → different hashes
        assert_ne!(hash1, hash2);

        // Both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    /// Test JWT token creation and validation roundtrip.
    #[test]
    fn test_jwt_roundtrip() {
        let settings = Settings {
            database_url: String::new(),
            host: "127.0.0.1".to_string(),
            port: 3001,
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            jwt_secret: "test-secret-for-unit-tests".to_string(),
        };

        let user_id = Uuid::new_v4().to_string();
        let token = create_access_token(&user_id, "user", &settings)
            .expect("Token creation should succeed");

        // Validate the token
        let claims = validate_token(&token, &settings).expect("Token validation should succeed");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.role, "user");
    }

    /// Test that token validation fails with wrong secret.
    #[test]
    fn test_jwt_validation_fails_with_wrong_secret() {
        let settings1 = Settings {
            database_url: String::new(),
            host: "127.0.0.1".to_string(),
            port: 3001,
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            jwt_secret: "secret-one".to_string(),
        };

        let settings2 = Settings {
            database_url: String::new(),
            host: "127.0.0.1".to_string(),
            port: 3001,
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            jwt_secret: "secret-two".to_string(),
        };

        let token = create_access_token("user-id", "user", &settings1).unwrap();
        let result = validate_token(&token, &settings2);
        assert!(result.is_err());
    }

    /// Test SHA-256 token hashing.
    #[test]
    fn test_token_hashing() {
        let token = "my-refresh-token-12345";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        // Same input → same hash (deterministic)
        assert_eq!(hash1, hash2);

        // Hash should be 64 hex characters (32 bytes)
        assert_eq!(hash1.len(), 64);

        // Different input → different hash
        let hash3 = hash_token("different-token");
        assert_ne!(hash1, hash3);
    }
}
