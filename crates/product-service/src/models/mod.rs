// Re-export all models for convenient imports
pub mod product;
// Note: We don't use `pub use product::*` because Rust prefers explicit imports
// in each file over wildcard re-exports. This avoids name conflicts and makes
// dependencies clearer.
