// =============================================================================
// PHASE 3: Cart Handlers — Axum async handlers using shared mutable state
// =============================================================================
// These handlers demonstrate the Interior Mutability pattern in practice.
// Each handler receives `State<Arc<CartStore>>` — an Arc-wrapped, RwLock-protected
// in-memory store that's shared across ALL concurrent requests.
//
// KEY LESSON: Handler extractors as dependency injection
// =======================================================
// Axum handlers declare what they need as function parameters:
//   - `State<Arc<CartStore>>` — shared application state
//   - `Extension<AuthUser>` — authenticated user (from JWT middleware)
//   - `Json<T>` — JSON request body (deserialized at compile time)
//   - `Path<T>` — URL path parameters
//
// Each parameter type implements the `FromRequest` trait. Axum calls
// `FromRequest::from_request()` for each parameter in order. If any fails,
// Axum returns 400/500 automatically. This eliminates boilerplate.
//
// **C++ analogy:** Dependency injection containers (but compile-time).
// **Go analogy:** Manual request parsing in each handler.
// **Express.js analogy:** `req.body`, `req.params` — but untyped.
// **Axum advantage:** Everything is type-safe and compile-time checked.

use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::error::CartError;
use crate::middleware::auth::AuthUser;

/// GET /cart — Get the current user's cart.
///
/// KEY LESSON: Read-only handler
/// ==============================
/// This handler only needs `.read().await` — multiple users can read
/// their carts concurrently. The RwLock allows this, unlike a Mutex
/// which would serialize all reads.
pub async fn get_cart(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<CartResponse>, CartError> {
    let cart = state
        .cart_store
        .get_or_create_cart(&auth_user.user_id)
        .await?;
    Ok(Json(CartResponse::from_cart(&cart)))
}

/// POST /cart/items — Add an item to the cart.
///
/// KEY LESSON: Mutable handler
/// ============================
/// This handler calls `.write().await` — it needs exclusive access.
/// If another request is reading the cart, this will wait (async-yield)
/// until all readers release their locks.
///
/// The `add_item` method auto-creates a cart if none exists, and
/// auto-increments quantity if the product already exists.
pub async fn add_item(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AddItemRequest>,
) -> Result<(StatusCode, Json<CartResponse>), CartError> {
    let cart = state
        .cart_store
        .add_item(
            &auth_user.user_id,
            req.product_id,
            req.name,
            req.unit_price,
            req.quantity,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(CartResponse::from_cart(&cart))))
}

/// PUT /cart/items/:product_id — Update item quantity.
///
/// KEY LESSON: Path extractor
/// ==========================
/// `Path(product_id): Path<Uuid>` — Axum parses the path parameter and
/// converts it to a Uuid. If parsing fails, Axum returns 400 automatically.
/// No manual string parsing or error handling needed.
///
/// **Go comparison:** You'd manually parse with `uuid.Parse(r.URL.Query().Get("id"))`.
/// **Axum advantage:** Type-safe path parameters at compile time.
pub async fn update_quantity(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(product_id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateQuantityRequest>,
) -> Result<Json<CartResponse>, CartError> {
    let cart = state
        .cart_store
        .update_quantity(&auth_user.user_id, &product_id, req.quantity)
        .await?;

    Ok(Json(CartResponse::from_cart(&cart)))
}

/// DELETE /cart/items/:product_id — Remove an item from the cart.
pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Path(product_id): axum::extract::Path<Uuid>,
) -> Result<Json<CartResponse>, CartError> {
    let cart = state
        .cart_store
        .remove_item(&auth_user.user_id, &product_id)
        .await?;
    Ok(Json(CartResponse::from_cart(&cart)))
}

/// DELETE /cart — Clear the entire cart.
pub async fn clear_cart(
    State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<CartResponse>, CartError> {
    let cart = state.cart_store.clear_cart(&auth_user.user_id).await?;
    Ok(Json(CartResponse::from_cart(&cart)))
}

// ─── DTOs (Data Transfer Objects) ────────────────────────────────────────────

/// Request body for adding an item to the cart.
///
/// KEY LESSON: Serde derive for compile-time JSON parsing
/// ========================================================
/// `#[derive(Deserialize)]` auto-generates JSON parsing code at compile time.
/// No reflection, no runtime type inspection. The generated code is just as
/// fast as hand-written parsing — but with zero chance of manual bugs.
///
/// **Go:** `json.Unmarshal` with runtime reflection.
/// **C++:** nlohmann/json uses compile-time templates.
/// **Rust:** Serde's derive macro generates compile-time code (fastest option).
#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    /// UUID of the product to add.
    pub product_id: Uuid,

    /// Product name (denormalized from product-service).
    pub name: String,

    /// Unit price in decimal (from product-service at time of add).
    pub unit_price: rust_decimal::Decimal,

    /// Quantity to add.
    pub quantity: i32,
}

/// Request body for updating item quantity.
#[derive(Debug, Deserialize)]
pub struct UpdateQuantityRequest {
    /// New quantity. If 0 or negative, the item is removed.
    pub quantity: i32,
}

/// Response body for cart endpoints.
///
/// KEY LESSON: Response DTO (Data Transfer Object)
/// ================================================
/// We don't expose the internal Cart struct directly via the API.
/// Instead, we create a CartResponse DTO. This decouples the API
/// contract from the internal data model. We can change the internal
/// Cart struct without breaking the API.
///
/// **Go analogy:** Response structs in HTTP handlers.
/// **C++ analogy:** DTOs in layered architecture.
/// **Rust benefit:** #[serde(rename_all = "camelCase")] ensures
/// the JSON uses camelCase (frontend convention) while Rust uses
/// snake_case (Rust convention). Best of both worlds.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartResponse {
    pub user_id: Uuid,
    pub items: Vec<CartItemResponse>,
    /// Cart total calculated server-side
    pub total: String, // String to avoid floating-point JSON representation
    pub item_count: usize,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CartItemResponse {
    pub product_id: Uuid,
    pub name: String,
    pub unit_price: String,
    pub quantity: i32,
    pub line_total: String,
    pub added_at: String,
}

impl CartResponse {
    /// Convert from internal Cart model to API response DTO.
    pub fn from_cart(cart: &crate::models::cart::Cart) -> Self {
        Self {
            user_id: cart.user_id,
            items: cart
                .items
                .iter()
                .map(|item| CartItemResponse {
                    product_id: item.product_id,
                    name: item.name.clone(),
                    unit_price: item.unit_price.to_string(),
                    quantity: item.quantity,
                    line_total: item.line_total().to_string(),
                    added_at: item.added_at.to_rfc3339(),
                })
                .collect(),
            total: cart.total().to_string(),
            item_count: cart.item_count(),
            updated_at: cart.updated_at.to_rfc3339(),
        }
    }
}
