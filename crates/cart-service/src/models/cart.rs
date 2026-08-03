// =============================================================================
// PHASE 3: Cart Model — Interior Mutability & Concurrency
// =============================================================================
// This is THE file where Phase 3's core concepts are taught.
//
// KEY LESSON: Interior Mutability Pattern
// ========================================
// Rust's fundamental rule: ONE mutable reference OR many immutable references.
// This is enforced at compile time. But sometimes you NEED shared mutable state:
//   - Multiple HTTP handlers need to read/write the same cart data
//   - The state is shared across all requests (via Axum State)
//
// "Interior Mutability" lets you mutate data through a shared (&T) reference
// by moving the borrow-check to RUNTIME instead of compile time.
//
// The pattern: `Arc<RwLock<T>>`
//   - `Arc<T>`: Shared ownership (Atomic Reference Counting) — like C++ shared_ptr
//   - `RwLock<T>`: Interior mutability with reader/writer locking
//
// CRITICAL DISTINCTION: tokio::sync::RwLock vs std::sync::RwLock
// =================================================================
// std::sync::RwLock::read() BLOCKS THE OS THREAD until the lock is acquired.
// In async code, blocking an OS thread starves OTHER tasks on that thread.
// tokio::sync::RwLock::read() RETURNS A FUTURE — it YIELDS to the runtime
// instead of blocking the thread. Other tasks can run while waiting.
//
// RULE: Always use tokio::sync::RwLock in async code, std::sync::RwLock in sync code.
// Go doesn't have this distinction — all mutex ops block goroutines, but
// goroutines are cheap so it's fine. Rust's async model is cooperative.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::CartError;

// ─── DATA TYPES ──────────────────────────────────────────────────────────────

/// A single item in a shopping cart.
///
/// KEY LESSON: Store owned Strings, not &str references
/// ======================================================
/// Struct fields should own their data (String, not &str) unless the struct
/// is specifically designed to be a temporary view. The CartItem needs to
/// outlive any particular request, so it must own its data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    /// The product's UUID.
    pub product_id: Uuid,

    /// Product name (denormalized from product-service for display).
    pub name: String,

    /// Unit price at the time the item was added.
    pub unit_price: Decimal,

    /// Quantity in cart.
    pub quantity: i32,

    /// When the item was added to the cart.
    pub added_at: DateTime<Utc>,
}

impl CartItem {
    /// Calculate the total price for this line item.
    /// `Decimal * i32` is supported by rust_decimal.
    pub fn line_total(&self) -> Decimal {
        self.unit_price * Decimal::from(self.quantity)
    }
}

/// The complete shopping cart for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cart {
    /// The user who owns this cart.
    pub user_id: Uuid,

    /// Items in the cart.
    pub items: Vec<CartItem>,

    /// When the cart was last modified.
    pub updated_at: DateTime<Utc>,
}

impl Cart {
    /// Create a new, empty cart for a user.
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            items: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Calculate the cart total (sum of all line totals).
    pub fn total(&self) -> Decimal {
        self.items.iter().map(|item| item.line_total()).sum()
    }

    /// Check if the cart is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Count distinct items in the cart (not total quantity).
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

// ─── CART STORE — THE IN-MEMORY STORAGE ──────────────────────────────────────

/// In-memory cart store using `Arc<RwLock<HashMap<...>>>`.
///
/// KEY LESSON: `Mutex<T>` OWNS the data
/// ======================================
/// In Go, a mutex and the data it protects are separate:
/// ```go
/// type CartStore struct {
///     mu   sync.RWMutex
///     data map[string]*Cart
/// }
/// // You can FORGET to lock! Compiler won't stop you.
/// ```
///
/// In Rust, `RwLock<T>` wraps the data. You CANNOT access T without locking:
/// ```rust
/// let guard = store.carts.read().await;  // MUST lock to access
/// let cart = guard.get(&user_id);        // NOW you can read
/// // guard auto-unlocks when dropped (RAII — like C++ lock_guard)
/// ```
///
/// The compiler ENFORCES that you lock before accessing. You cannot forget.
/// This eliminates an entire class of concurrency bugs.
///
/// WHY RwLock (not Mutex)?
/// =========================
/// - `RwLock::read()` allows MULTIPLE concurrent readers (GET /cart)
/// - `RwLock::write()` allows ONE exclusive writer (POST/PUT/DELETE)
/// - For a cart service, reads are more common than writes
/// - `Mutex` would serialize ALL access (slower under read-heavy load)
#[derive(Clone)]
pub struct CartStore {
    /// The inner data: a map from user_id to Cart.
    /// `Arc` allows cloning the store (cheap — just bumps an atomic counter).
    /// `RwLock` provides async-aware interior mutability.
    ///
    /// KEY LESSON: Arc::clone() is cheap
    /// ===================================
    /// `Arc::clone(&store.carts)` does NOT copy the HashMap. It only increments
    /// an atomic reference count (like C++ shared_ptr). The data is shared.
    /// This is why we can pass `CartStore` to Axum's `State` — it clones cheaply.
    ///
    /// Compare:
    ///   - C++: shared_ptr<T> — same semantics
    ///   - Go: all maps are reference types by default (implicit sharing)
    ///   - Rust: explicit Arc<T> — costs are visible
    pub carts: Arc<RwLock<HashMap<Uuid, Cart>>>,
}

impl CartStore {
    /// Create a new, empty cart store.
    pub fn new() -> Self {
        Self {
            carts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a user's cart (returns a clone, not a reference to locked data).
    ///
    /// KEY LESSON: Lock scope is controlled by RAII
    /// ==============================================
    /// The `.read().await` returns a `RwLockReadGuard<'_, HashMap<...>>`.
    /// This guard acts like a smart pointer — you can dereference it to access
    /// the HashMap. When the guard goes out of scope (end of this function),
    /// the lock is automatically released. No `defer mutex.RUnlock()` needed.
    ///
    /// Compare to Go:
    /// ```go
    /// func (s *CartStore) GetCart(userID string) (*Cart, error) {
    ///     s.mu.RLock()
    ///     defer s.mu.RUnlock()  // Easy to forget!
    ///     cart, ok := s.data[userID]
    ///     ...
    /// }
    /// ```
    /// Rust's RAII makes this automatic and compile-time safe.
    pub async fn get_cart(&self, user_id: &Uuid) -> Result<Option<Cart>, CartError> {
        let carts = self.carts.read().await;
        Ok(carts.get(user_id).cloned())
    }

    /// Get or create a cart for a user. Always returns a cart — creates empty
    /// one if none exists.
    ///
    /// KEY LESSON: Read-then-write pattern
    /// =====================================
    /// First we check with a read lock (cheap, allows concurrency).
    /// If the cart exists, we return it without ever taking a write lock.
    /// Only if the cart doesn't exist do we upgrade to a write lock.
    /// This minimizes contention — most requests are for existing carts.
    pub async fn get_or_create_cart(&self, user_id: &Uuid) -> Result<Cart, CartError> {
        // Fast path: read lock
        {
            let carts = self.carts.read().await;
            if let Some(cart) = carts.get(user_id) {
                return Ok(cart.clone());
            }
        } // <-- read lock DROPPED here (guard goes out of scope)

        // Slow path: write lock to create
        let mut carts = self.carts.write().await;
        // Double-check: another request might have created the cart between
        // our read unlock and write lock
        if let Some(cart) = carts.get(user_id) {
            return Ok(cart.clone());
        }

        let cart = Cart::new(*user_id);
        carts.insert(*user_id, cart.clone());
        Ok(cart)
    }

    /// Add an item to a user's cart.
    ///
    /// If the product already exists in the cart, increment its quantity.
    /// Otherwise, add a new line item.
    pub async fn add_item(
        &self,
        user_id: &Uuid,
        product_id: Uuid,
        name: String,
        unit_price: Decimal,
        quantity: i32,
    ) -> Result<Cart, CartError> {
        if quantity <= 0 {
            return Err(CartError::ValidationError(
                "Quantity must be positive".to_string(),
            ));
        }

        let mut carts = self.carts.write().await;
        let cart = carts
            .entry(*user_id)
            .or_insert_with(|| Cart::new(*user_id));

        // Check if the product is already in the cart
        if let Some(existing) = cart
            .items
            .iter_mut()
            .find(|item| item.product_id == product_id)
        {
            existing.quantity += quantity;
        } else {
            cart.items.push(CartItem {
                product_id,
                name,
                unit_price,
                quantity,
                added_at: Utc::now(),
            });
        }

        cart.updated_at = Utc::now();
        Ok(cart.clone())
    }

    /// Update the quantity of an item in the cart.
    ///
    /// If quantity is 0 or negative, the item is removed.
    /// If the product is not in the cart, returns ItemNotFound error.
    pub async fn update_quantity(
        &self,
        user_id: &Uuid,
        product_id: &Uuid,
        quantity: i32,
    ) -> Result<Cart, CartError> {
        let mut carts = self.carts.write().await;

        let cart = carts
            .get_mut(user_id)
            .ok_or_else(|| CartError::CartNotFound {
                user_id: *user_id,
            })?;

        if quantity <= 0 {
            // Remove the item if quantity is 0 or negative
            let original_len = cart.items.len();
            cart.items
                .retain(|item| item.product_id != *product_id);
            if cart.items.len() == original_len {
                return Err(CartError::ItemNotFound {
                    product_id: product_id.to_string(),
                });
            }
        } else {
            let item = cart
                .items
                .iter_mut()
                .find(|item| item.product_id == *product_id)
                .ok_or_else(|| CartError::ItemNotFound {
                    product_id: product_id.to_string(),
                })?;
            item.quantity = quantity;
        }

        cart.updated_at = Utc::now();
        Ok(cart.clone())
    }

    /// Remove an item from the cart.
    pub async fn remove_item(
        &self,
        user_id: &Uuid,
        product_id: &Uuid,
    ) -> Result<Cart, CartError> {
        let mut carts = self.carts.write().await;

        let cart = carts
            .get_mut(user_id)
            .ok_or_else(|| CartError::CartNotFound {
                user_id: *user_id,
            })?;

        let original_len = cart.items.len();
        cart.items
            .retain(|item| item.product_id != *product_id);

        if cart.items.len() == original_len {
            return Err(CartError::ItemNotFound {
                product_id: product_id.to_string(),
            });
        }

        cart.updated_at = Utc::now();
        Ok(cart.clone())
    }

    /// Clear all items from a user's cart.
    pub async fn clear_cart(&self, user_id: &Uuid) -> Result<Cart, CartError> {
        let mut carts = self.carts.write().await;

        let cart = carts
            .get_mut(user_id)
            .ok_or_else(|| CartError::CartNotFound {
                user_id: *user_id,
            })?;

        cart.items.clear();
        cart.updated_at = Utc::now();
        Ok(cart.clone())
    }
}

impl Default for CartStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── TESTS ───────────────────────────────────────────────────────────────────
// KEY LESSON: Testing concurrent code
// ====================================
// We use `tokio::test` (not `#[test]`) because CartStore methods are async
// and use tokio::sync::RwLock. The test runtime provides the Tokio scheduler.
//
// Compare: Go tests can use goroutines natively. Rust async tests need
// the `#[tokio::test]` attribute to set up the async runtime.

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test UUID.
    fn test_user_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn test_product_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-100000000001").unwrap()
    }

    #[tokio::test]
    async fn test_new_store_is_empty() {
        let store = CartStore::new();
        let cart = store.get_cart(&test_user_id()).await.unwrap();
        assert!(cart.is_none());
    }

    #[tokio::test]
    async fn test_add_item_creates_cart() {
        let store = CartStore::new();
        let result = store
            .add_item(
                &test_user_id(),
                test_product_id(),
                "Test Product".into(),
                Decimal::new(1999, 2), // 19.99
                2,
            )
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].quantity, 2);
        assert_eq!(result.items[0].name, "Test Product");
    }

    #[tokio::test]
    async fn test_add_item_increments_existing() {
        let store = CartStore::new();
        let user_id = test_user_id();
        let product_id = test_product_id();

        // Add once
        store
            .add_item(&user_id, product_id, "Widget".into(), Decimal::new(500, 2), 1)
            .await
            .unwrap();

        // Add same product again — should increment quantity
        let cart = store
            .add_item(&user_id, product_id, "Widget".into(), Decimal::new(500, 2), 3)
            .await
            .unwrap();

        assert_eq!(cart.items.len(), 1, "Should still be one line item");
        assert_eq!(cart.items[0].quantity, 4, "Quantity should be 1 + 3 = 4");
    }

    #[tokio::test]
    async fn test_add_item_rejects_zero_quantity() {
        let store = CartStore::new();
        let result = store
            .add_item(
                &test_user_id(),
                test_product_id(),
                "Test".into(),
                Decimal::new(100, 0),
                0,
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CartError::ValidationError(msg) => {
                assert!(msg.contains("positive"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_update_quantity() {
        let store = CartStore::new();
        let user_id = test_user_id();
        let product_id = test_product_id();

        store
            .add_item(&user_id, product_id, "Item".into(), Decimal::new(100, 0), 2)
            .await
            .unwrap();

        let cart = store
            .update_quantity(&user_id, &product_id, 5)
            .await
            .unwrap();

        assert_eq!(cart.items[0].quantity, 5);
    }

    #[tokio::test]
    async fn test_update_quantity_removes_on_zero() {
        let store = CartStore::new();
        let user_id = test_user_id();
        let product_id = test_product_id();

        store
            .add_item(&user_id, product_id, "Item".into(), Decimal::new(100, 0), 2)
            .await
            .unwrap();

        let cart = store
            .update_quantity(&user_id, &product_id, 0)
            .await
            .unwrap();

        assert!(cart.items.is_empty());
    }

    #[tokio::test]
    async fn test_remove_item() {
        let store = CartStore::new();
        let user_id = test_user_id();
        let product_id = test_product_id();

        store
            .add_item(&user_id, product_id, "Item".into(), Decimal::new(100, 0), 1)
            .await
            .unwrap();

        let cart = store.remove_item(&user_id, &product_id).await.unwrap();
        assert!(cart.items.is_empty());
    }

    #[tokio::test]
    async fn test_remove_item_not_found() {
        let store = CartStore::new();
        let result = store
            .remove_item(&test_user_id(), &test_product_id())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_clear_cart() {
        let store = CartStore::new();
        let user_id = test_user_id();

        store
            .add_item(
                &user_id,
                Uuid::new_v4(),
                "Item 1".into(),
                Decimal::new(100, 0),
                1,
            )
            .await
            .unwrap();
        store
            .add_item(
                &user_id,
                Uuid::new_v4(),
                "Item 2".into(),
                Decimal::new(200, 0),
                2,
            )
            .await
            .unwrap();

        let cart = store.clear_cart(&user_id).await.unwrap();
        assert!(cart.items.is_empty());
    }

    #[tokio::test]
    async fn test_cart_total() {
        let store = CartStore::new();
        let user_id = test_user_id();

        store
            .add_item(
                &user_id,
                Uuid::new_v4(),
                "A".into(),
                Decimal::new(1000, 2), // 10.00
                2,
            )
            .await
            .unwrap();
        store
            .add_item(
                &user_id,
                Uuid::new_v4(),
                "B".into(),
                Decimal::new(500, 2), // 5.00
                3,
            )
            .await
            .unwrap();

        let cart = store.get_cart(&user_id).await.unwrap().unwrap();
        // 10.00 * 2 + 5.00 * 3 = 20.00 + 15.00 = 35.00
        assert_eq!(cart.total(), Decimal::new(3500, 2));
    }

    #[tokio::test]
    async fn test_get_or_create_returns_existing() {
        let store = CartStore::new();
        let user_id = test_user_id();

        // First call creates
        let cart1 = store.get_or_create_cart(&user_id).await.unwrap();
        assert!(cart1.is_empty());

        // Add an item via the store
        store
            .add_item(&user_id, test_product_id(), "Item".into(), Decimal::new(100, 0), 1)
            .await
            .unwrap();

        // Second call returns existing cart WITH the item
        let cart2 = store.get_or_create_cart(&user_id).await.unwrap();
        assert_eq!(cart2.items.len(), 1);
    }
}