// =============================================================================
// Phase 1, Step 2: Configuration Management
// =============================================================================
// KEY LESSON: The `config` crate
// ================================
// Rust has no standard configuration format. The `config` crate is the community
// standard — it's like Go's `viper` library:
//   - Merge settings from multiple sources (env vars, files, defaults)
//   - Hierarchical keys with dot-separated names (e.g., "server.port")
//   - Auto-converted to types via serde (no manual type conversion!)
//
// PATTERN: Define a struct, derive Deserialize, use Config builder to populate it.
// This is the "parse, don't validate" pattern applied to configuration.
//
// COMPARISON:
//   Go:     viper.GetString("database.url") — stringly-typed, runtime errors
//   C++:    std::getenv + manual parsing — same issues
//   Node:   process.env + manual parsing — same issues
//   Rust:   Config::build() + deserialize into a struct — TYPE-SAFE, compile-time schema
//
// The `envy` crate is an alternative: it maps env vars directly to struct fields
// (like `envconfig` in Go). We use `config` here because it supports files too.

use serde::Deserialize;

/// Application configuration, loaded from environment variables.
///
/// KEY LESSON: `#[serde(deserialize_with)]` and `#[serde(default)]` annotations
/// =============================================================================
/// `#[serde(default)]` — if the field is missing from the source, use `Default::default()`
///                        (for String: empty string, for u16: 0)
/// `#[serde(default = "function_name")]` — use a custom default function
///
/// Naming convention:
/// - Struct fields: `snake_case` (Rust standard)
/// - Environment variables: `APP__*` (double underscore for nested, single for flat)
///   Example: `APP_SERVER_PORT=8080` maps to `Settings.server.port`
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Database configuration.
    /// KEY LESSON: Nested structs in config.
    /// `config` maps `APP_DATABASE_URL` to `database.url` via the `APP` prefix + prefix `DATABASE`.
    #[serde(default)]
    pub database: DatabaseSettings,

    /// Server configuration.
    #[serde(default)]
    pub server: ServerSettings,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DatabaseSettings {
    /// PostgreSQL connection URL.
    /// Example: postgres://user:password@localhost:5432/ecommerce
    /// KEY LESSON: Rust naming: `url` not `URL`. Rust style is snake_case always,
    /// even for acronyms. So `http_client`, not `HTTPClient`. Unlike Go's `URL`.
    #[serde(default = "default_database_url")]
    pub url: String,

    /// Maximum number of connections in the pool.
    /// SQLx default is 10. 25 is reasonable for a microservice.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerSettings {
    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Host to bind to.
    #[serde(default = "default_host")]
    pub host: String,
}

// ─── Default value functions ─────────────────────────────────────────────────
// KEY LESSON: Custom default functions for serde
// ==============================================
// These are called when the field is missing from the environment.
// They provide reasonable defaults for development.

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5432/ecommerce".into()
}

fn default_max_connections() -> u32 {
    25
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "0.0.0.0".into() // Listen on all interfaces (like Go's ":3000", Express's app.listen(3000))
}

// Implement Default for the whole Settings struct
// This lets us use Settings::default() in tests without setting env vars.
impl Default for Settings {
    fn default() -> Self {
        Self {
            database: DatabaseSettings {
                url: default_database_url(),
                max_connections: default_max_connections(),
            },
            server: ServerSettings {
                port: default_port(),
                host: default_host(),
            },
        }
    }
}

/// Load configuration from environment variables.
///
/// KEY LESSON: The `?` operator in practice
/// =========================================
/// Every fallible operation uses `?` — if it fails, the error propagates up.
/// This is Rust's equivalent of Go's `if err != nil { return err }` pattern,
/// but it's a single character.
///
/// `config::Config::builder()`
///   .add_source(config::Environment::with_prefix("APP").separator("__"))
///   .build()?;
/// This reads all env vars starting with `APP_` and maps them:
///   APP_DATABASE_URL → database.url
///   APP_SERVER__PORT → server.port  (double underscore for nested)
///   APP_SERVER_PORT  → server_port   (single underscore in flat naming)
///
/// KEY INSIGHT: The `config` crate does the parsing — we don't need to manually
/// map env var names to struct fields. Compare to Go where you'd write:
///   cfg.Server.Port = os.Getenv("SERVER_PORT") // manual mapping for every field!
pub fn load_config() -> Result<Settings, config::ConfigError> {
    // Initialize with defaults, then override with environment
    let settings = config::Config::builder()
        // Layer 1: Hard-coded defaults (lowest priority)
        .set_default("database.url", default_database_url())?
        .set_default(
            "database.max_connections",
            default_max_connections().to_string(),
        )?
        .set_default("server.port", default_port().to_string())?
        .set_default("server.host", default_host())?
        // Layer 2: Environment variables (higher priority)
        // APP_DATABASE_URL — overrides database.url
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    // Deserialize the config into our Settings struct
    // This is where type conversion happens — the config crate ensures
    // SERVER_PORT = "3000" (string) becomes server.port: u16 = 3000
    settings.try_deserialize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.port, 3000);
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.database.max_connections, 25);
        assert!(settings.database.url.contains("postgres"));
    }
}
