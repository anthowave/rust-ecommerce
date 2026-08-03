// Re-export all handlers for convenient imports
pub mod products;

// Note: Explicit imports preferred over wildcard re-exports.
// Each file that needs handlers imports them directly from `crate::handlers::products`.
