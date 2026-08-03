// =============================================================================
// PHASE 3: Cart Service Configuration
// =============================================================================
// KEY LESSON: config crate — layered configuration
// ==================================================
// The `config` crate merges settings from multiple sources in order:
//   1. Default values (set in code via `Default` trait)
//   2. Environment variables (overrides defaults)
//   3. Config files (optional, .env files)
//
// The `#[serde(default)]` attribute means: if a field is missing from the
// environment, use the Default::default() value instead of erroring.
//
// COMPARISON:
//   Go: viper library merges config from files, env, flags
//   C++: boost::program_options or manual env parsing
//   Rust: config crate is the standard

use serde::Deserialize;

/// Cart service configuration, loaded from environment variables.
///
/// KEY LESSON: `Clone` derive
/// ============================
/// We need Clone because `AppState` will hold a `Settings` and `State<T>` requires
/// `Clone` for the inner type. Since `Settings` contains heap-allocated `String`s,
/// it can be `Clone` but NOT `Copy`. Clone requires an explicit `.clone()` call,
/// which is a visible cost — this is intentional in Rust.
///
/// Also: `Settings` stores `String` (owned), not `&str` (borrowed).
/// This is because the Settings struct needs to OWN its data — it can't borrow
/// from something that might be dropped. Config data typically lives for the
/// entire program lifetime, but Rust's borrow checker doesn't know that.
/// Using owned `String` makes this explicit and safe.
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    /// Server host
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// PostgreSQL connection string
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// JWT secret key for token validation
    #[serde(default)]
    pub jwt_secret: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3002
}

fn default_database_url() -> String {
    "postgres://postgres:postgres@localhost:5432/ecommerce".to_string()
}

// KEY LESSON: Default trait
// =========================
// The `Default` trait provides `Settings::default()` which creates a Settings
// with all default values. The `#[serde(default)]` on each field uses these.
// `..Default::default()` is the idiomatic way to fill in remaining fields.
impl Default for Settings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            database_url: default_database_url(),
            jwt_secret: String::new(),
        }
    }
}

impl Settings {
    /// Load settings from environment variables.
    ///
    /// KEY LESSON: config crate builder pattern
    /// ==========================================
    /// `Config::builder()` creates a builder. Each `.add_source()` adds a layer.
    /// `.build()?` merges all layers — later sources override earlier ones.
    /// `.try_deserialize()` converts the merged config into our Settings struct.
    ///
    /// The `?` operator works because `config::ConfigError` implements `std::error::Error`,
    /// and our function returns `anyhow::Result<Self>` which can hold any error type.
    pub fn from_env() -> anyhow::Result<Self> {
        let settings = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .prefix("CART_")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let mut settings: Self = settings.try_deserialize()?;

        // Apply defaults for values that must be non-empty
        if settings.host.is_empty() {
            settings.host = default_host();
        }
        if settings.port == 0 {
            settings.port = default_port();
        }
        if settings.database_url.is_empty() {
            settings.database_url = default_database_url();
        }

        Ok(settings)
    }
}
