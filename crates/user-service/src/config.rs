use config::Config as ConfigLoader;
use serde::Deserialize;
use tracing::warn;

/// Application configuration loaded from environment variables.
///
/// # Rust Concept: Clone vs Copy
///
/// This struct derives `Clone` (explicit `.clone()`) but NOT `Copy`.
/// `Copy` means bitwise copy is always safe — it's only implemented for
/// types that live on the stack (integers, bools, small fixed-size types).
/// Our config contains `String` which owns heap data — a bitwise copy
/// would duplicate the pointer but not the data, leading to a double-free.
/// Rust prevents this: `String` is `Clone` but not `Copy`.
///
/// **C++ analogy:** `Copy` ≈ trivially copyable types (int, bool). `Clone` ≈
/// copy constructor for types with heap data (std::string, std::vector).
/// **Go analogy:** Go copies everything implicitly, but slices/maps are
/// reference types internally (the underlying array is shared). This is
/// confusing — Rust makes the cost explicit with `.clone()`.
///
/// # Rust Concept: String vs &str
///
/// Why `String` fields rather than `&'a str`? Because our config must own
/// its data — it lives for the entire program lifetime. `&str` would borrow
/// from the config loader, which gets dropped after startup. This is the
/// same reason you use `std::string` instead of `std::string_view` for
/// persistent data in C++.
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    /// Database connection URL (e.g., postgres://user:pass@localhost/userdb)
    pub database_url: String,

    /// Server host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// JWT access token expiration in minutes
    #[serde(default = "default_access_token_expiry_minutes")]
    pub access_token_expiry_minutes: i64,

    /// JWT refresh token expiration in days
    #[serde(default = "default_refresh_token_expiry_days")]
    pub refresh_token_expiry_days: i64,

    /// JWT signing secret — MUST be set in production!
    /// In dev, falls back to a default (with a warning).
    /// Note: The Argon2 salt is embedded in the password hash itself,
    /// so we don't need a separate salt config.
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3001 // Different from product-service (3000)
}

fn default_access_token_expiry_minutes() -> i64 {
    15 // Short-lived access tokens (standard security practice)
}

fn default_refresh_token_expiry_days() -> i64 {
    7 // Refresh tokens last longer for convenience
}

fn default_jwt_secret() -> String {
    "CHANGE_ME_IN_PRODUCTION".to_string()
}

impl Settings {
    /// Load settings from environment variables.
    ///
    /// Uses the `config` crate which reads from:
    /// 1. Default values (in this struct)
    /// 2. Environment variables (prefix USER_SERVICE__)
    ///
    /// Environment variables use __ as separator for nested keys.
    /// Example: USER_SERVICE__DATABASE_URL=postgres://...
    /// Example: USER_SERVICE__JWT_SECRET=supersecret
    pub fn from_env() -> anyhow::Result<Self> {
        let config = ConfigLoader::builder()
            // Add defaults first (lowest priority)
            .add_source(config::Config::try_from(&Settings::defaults_map())?)
            // Override with environment variables
            .add_source(
                config::Environment::with_prefix("USER_SERVICE")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let settings: Settings = config.try_deserialize()?;

        // Security check: warn if using default JWT secret
        if settings.jwt_secret == "CHANGE_ME_IN_PRODUCTION" {
            warn!("WARNING: Using default JWT secret! Set USER_SERVICE__JWT_SECRET in production!");
        }

        // Warn about short access token expiry in dev
        if settings.access_token_expiry_minutes < 5 {
            warn!(
                "Access token expiry is very short: {} minutes",
                settings.access_token_expiry_minutes
            );
        }

        Ok(settings)
    }

    /// Build a map of default values for the config loader.
    fn defaults_map() -> serde_json::Value {
        serde_json::json!({
            "host": default_host(),
            "port": default_port(),
            "access_token_expiry_minutes": default_access_token_expiry_minutes(),
            "refresh_token_expiry_days": default_refresh_token_expiry_days(),
            "jwt_secret": default_jwt_secret(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        // We can't easily test from_env() without setting env vars,
        // but we can verify the defaults make sense.
        let defaults = Settings::defaults_map();
        assert_eq!(defaults["host"], "127.0.0.1");
        assert_eq!(defaults["port"], 3001);
        assert_eq!(defaults["access_token_expiry_minutes"], 15);
        assert_eq!(defaults["refresh_token_expiry_days"], 7);
    }
}
