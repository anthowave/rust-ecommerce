// =============================================================================
// Phase 1, Step 7: HTTP Handlers — Axum Extractors in Action
// =============================================================================
// KEY LESSON: Axum handler functions
// ===================================
// An Axum handler is ANY async function whose parameters implement `FromRequest`
// and whose return type implements `IntoResponse`. There's no magic interface
// or base class — just duck typing at the type system level.
//
// COMPARISON:
//   Go Gin:      func(c *gin.Context) { ... }  — all params from c.Query(), c.Param()
//   Express:     (req, res, next) => { ... }   — all params from req.params, req.body
//   Spring Boot: @GetMapping("/{id}") method(@PathVariable Long id) — closest to Axum
//   Rust Axum:   async fn handler(Path(id): Path<Uuid>, State(state): State<...>) -> ...
//                Each parameter DECLARES what it needs. Axum resolves it at compile time.
//
// THIS IS THE KILLER FEATURE OF AXUM: You declare what you need in the function
// signature. If a parameter can't be extracted, Axum returns a 400/422/500
// automatically — you write ZERO error-handling boilerplate in your handlers.
//
// KEY LESSON: `Result<Json<T>, AppError>` return type
// =====================================================
// Because we implemented `IntoResponse` for `AppError` (in error.rs), Axum
// automatically converts both Ok and Err variants to HTTP responses:
//   Ok(Json(product)) → 200 with JSON body
//   Err(AppError::NotFound { .. }) → 404 with JSON error body
//   Err(AppError::ValidationError(..)) → 422 with JSON error body
//   Err(AppError::DatabaseError(..)) → 500 with JSON error body
//
// In Go, you'd write `c.JSON(200, product)` and `c.JSON(500, err)` manually
// in every handler. In Axum, the type system handles it.

use axum::Json;
use axum::extract::{Path, Query, State};
use std::sync::Arc;
use uuid::Uuid;

use common::error::AppError;

use crate::models::product;
use crate::models::product::{CreateProductRequest, ProductListResponse, UpdateProductRequest};

/// Application state shared across all handlers.
///
/// KEY LESSON: Arc<AppState> — Shared ownership across threads
/// ============================================================
/// `Arc` = Atomic Reference Counter. Like C++ `std::shared_ptr<T>`.
/// Every handler gets a clone of the Arc (cheap — just increments the counter).
/// The AppState holds our database pool — created once at startup, shared everywhere.
///
/// WHY ARC, NOT RC?
///   `Rc<T>` = single-threaded reference counting (fast, no atomics)
///   `Arc<T>` = atomic reference counting (thread-safe, slightly slower)
///   Tokio runs handlers on multiple threads → need Arc, not Rc.
///
/// In Go: you'd just share `*sql.DB` directly (GC handles cleanup).
/// In C++: you'd use `std::shared_ptr<AppState>`.
/// In Rust: `Arc<AppState>` is the idiomatic way to share state across handlers.
#[derive(Debug)]
pub struct AppState {
    pub db: crate::db::PgPool,
}

/// Query parameters for listing products.
///
/// KEY LESSON: Serde deserialization for query params
/// ====================================================
/// `#[serde(default)]` — if the query parameter is missing, use Default::default()
///   for u16: 0, for u64: 0, for String: empty string
/// `#[serde(default = "function")]` — use a custom default function
///
/// Axum's `Query<T>` extractor parses `?page=1&per_page=20&status=active&search=laptop`
/// into this struct automatically. If parsing fails (e.g., `?page=abc`), Axum
/// returns 422 with a helpful error message. No manual validation needed.
///
/// In Go: you'd parse each query param manually with `c.Query("page")` and `strconv.Atoi`.
/// In Express: `req.query.page` and manual type conversion.
#[derive(Debug, serde::Deserialize)]
pub struct ProductListQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub status: Option<product::ProductStatus>,
    pub search: Option<String>,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// POST /products — Create a new product.
///
/// KEY LESSON: Handler parameter decomposition
/// ============================================
/// Each parameter is extracted independently:
///   - `State(state): State<Arc<AppState>>` — get the shared application state
///   - `Json(body): Json<CreateProductRequest>` — parse the request body as JSON
///   - Returns `Result<impl IntoResponse, AppError>` — flexible return type
///
/// KEY LESSON: `impl IntoResponse` return type
/// ============================================
/// Using `impl IntoResponse` in the Ok position lets us return different types
/// for different handlers. For create, we return a 201 with the product.
/// For other handlers, we might return 200. The consistency comes from
/// `AppError` in the Err position — it's always the same error type.
///
/// Alternative: return `Result<Json<Product>, AppError>` for 200 status.
/// We use `impl IntoResponse` here to demonstrate the 201 status code pattern.
#[axum::debug_handler]
pub async fn create_product(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProductRequest>,
) -> Result<(axum::http::StatusCode, Json<product::Product>), AppError> {
    let product = product::create(&state.db, &body).await?;
    // KEY LESSON: Tuple response — (status, body) both implement IntoResponse
    // 201 Created is the standard for resource creation (200 is for reads, 201 for creates)
    Ok((axum::http::StatusCode::CREATED, Json(product)))
}

/// GET /products — List products with pagination and filtering.
///
/// KEY LESSON: Query<T> extractor
/// ===============================
/// `Query(query): Query<ProductListQuery>` — parses `?page=1&per_page=20` etc.
/// into a ProductListQuery struct. If parsing fails, Axum returns 422 automatically.
/// This is like Express's `req.query` but TYPE-SAFE — the struct defines the schema.
pub async fn list_products(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProductListQuery>,
) -> Result<Json<ProductListResponse>, AppError> {
    let result = product::list(
        &state.db,
        query.page,
        query.per_page,
        query.status.clone(),
        query.search.as_deref(), // KEY LESSON: Option<String> → Option<&str>
                                 // `.as_deref()` converts Option<String> to Option<&str>
                                 // This avoids cloning the String unnecessarily
    )
    .await?;

    Ok(Json(result))
}

/// GET /products/:id — Get a single product by ID.
///
/// KEY LESSON: Path<T> extractor
/// ==============================
/// `Path(id): Path<Uuid>` — extracts `:id` from the URL path `/products/:id`.
/// Axum validates that the path segment is a valid UUID. If it's not (e.g.,
/// `/products/not-a-uuid`), Axum returns 400 automatically. No manual parsing!
///
/// In Go: you'd write `uuid.Parse(c.Param("id"))` and check the error.
/// In Axum: the parameter IS a Uuid. If it can't be parsed, the handler
/// is never called — Axum handles it.
pub async fn get_product(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<product::Product>, AppError> {
    let product = product::find_by_id(&state.db, id).await?;

    match product {
        // KEY LESSON: Pattern matching on Option<T>
        // `match` is exhaustive — the compiler ensures we handle both Some and None.
        // In Go: `if product == nil { return nil, ErrNotFound }`
        // In Rust: pattern match — more concise, and the compiler checks exhaustiveness.
        Some(p) => Ok(Json(p)),
        None => Err(AppError::NotFound {
            entity: "Product",
            id: id.to_string(),
        }),
    }
}

/// PUT /products/:id — Update a product.
///
/// KEY LESSON: Combining Path + Json extractors
/// =============================================
/// `Path(id): Path<Uuid>` + `Json(body): Json<UpdateProductRequest>`
/// Both are extracted independently. The order doesn't matter — Axum figures
/// it out. This is the power of the extractor pattern: declare what you need,
/// Axum provides it. No manual parsing, no error handling boilerplate.
pub async fn update_product(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProductRequest>,
) -> Result<Json<product::Product>, AppError> {
    let updated = product::update(&state.db, id, &body).await?;

    match updated {
        Some(p) => Ok(Json(p)),
        None => Err(AppError::NotFound {
            entity: "Product",
            id: id.to_string(),
        }),
    }
}

/// DELETE /products/:id — Soft-delete a product.
///
/// KEY LESSON: Returning StatusCode directly
/// ==========================================
/// `StatusCode` implements `IntoResponse` — you can return it directly.
/// `(StatusCode, ())` is the idiomatic way to return an empty body with a status.
/// 204 No Content is the standard for successful DELETE operations.
pub async fn delete_product(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, ()), AppError> {
    let deleted = product::soft_delete(&state.db, id).await?;

    if deleted {
        // KEY LESSON: () is the "unit type" — like `void` in C/Go, but it's an
        // actual value (like an empty tuple). `(StatusCode, ())` returns just
        // the status code with an empty body.
        Ok((axum::http::StatusCode::NO_CONTENT, ()))
    } else {
        Err(AppError::NotFound {
            entity: "Product",
            id: id.to_string(),
        })
    }
}

/// GET /health — Health check endpoint.
///
/// KEY LESSON: Minimal handler — no state needed
/// ==============================================
/// This handler demonstrates that you can have handlers without State<T>.
/// It's used by load balancers and orchestrators to verify the service is alive.
/// Simple, stateless, always returns 200.
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "product-service",
    }))
}
