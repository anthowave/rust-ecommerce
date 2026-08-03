// =============================================================================
// Phase 1, Step 5: Product Model & Database Queries
// =============================================================================
// KEY LESSON: sqlx::FromRow — Compile-time row mapping
// =====================================================
// `#[derive(sqlx::FromRow)]` auto-generates code to map Postgres rows to
// Rust structs. This is like:
//   - Go: sqlx's `db.Select(&products, "SELECT * FROM products")` + struct tags
//   - C++: ODB or other ORMs that auto-generate mapping code
//   - JS: any ORM that maps rows to objects (Prisma, TypeORM, Sequelize)
//
// The KEY DIFFERENCE: SQLx checks at COMPILE TIME that:
//   1. The SQL query is syntactically correct
//   2. The columns returned by the query match the struct fields
//   3. The types are compatible (e.g., Postgres INTEGER → Rust i32)
//
// This means: if you rename a column in the DB, you get a COMPILE ERROR,
// not a runtime crash. No other language's SQL library does this.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use common::error::AppError;

/// Product model — mirrors the `products` table in PostgreSQL.
///
/// KEY LESSON: Derive macros in action
/// ====================================
/// `Debug`       — enables `{:?}` formatting (like Go's `%+v`, C++ operator<<)
/// `Clone`       — enables `.clone()` (like C++ copy constructor, Go's copy)
/// `Serialize`   — enables serialization TO JSON (like Go's json.Marshal)
/// `Deserialize` — enables deserialization FROM JSON (like Go's json.Unmarshal)
/// `FromRow`     — enables mapping FROM database rows (specific to sqlx)
///
/// IMPORTANT: `#[sqlx(type_name = "...")]` tells SQLx the Postgres type name.
/// This is needed for custom types (enums) because Rust's type system doesn't
/// know the Postgres type name by default.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub price: Decimal,
    pub stock: i32,
    // KEY LESSON: Redundant type_name on fields
    // The type_name is already on ProductStatus (from its derive).
    // Adding it again on the field confuses the FromRow derive macro.
    // Let the type-level attribute handle the mapping.
    pub status: ProductStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Product status enum — matches the `product_status` Postgres enum.
///
/// KEY LESSON: Deriving sqlx::Type for custom enums
/// =================================================
/// `#[derive(sqlx::Type)]` tells SQLx how to convert between this Rust enum
/// and a Postgres type. The `#[sqlx(type_name = "product_status")]` annotation
/// is ESSENTIAL — it links the Rust enum to the Postgres enum type.
///
/// Without this, SQLx wouldn't know how to read/write this type.
///
/// COMPARISON:
///   Go:   manual Scan/Value interface implementation for custom types
///   C++:  custom codec registration in your ORM
///   JS:   Prisma enum mapping in schema.prisma, TypeORM @Column decorator
///
/// The #[repr(i32)] is NOT needed here — SQLx uses the type_name, not numeric
/// representation. This is safer than C-style enums where you'd store integers
/// and hope they don't get out of sync.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[sqlx(type_name = "product_status", rename_all = "snake_case")]
pub enum ProductStatus {
    Active,
    Draft,
    Discontinued,
    OutOfStock,
}

/// Request body for creating a product.
///
/// KEY LESSON: Separate request/response types from model types
/// =============================================================
/// This is a DTO (Data Transfer Object) pattern. The API contract and the
/// database schema should evolve independently. Using separate types prevents:
///   - Exposing database internals in the API (e.g., deleted_at, timestamps)
///   - Accidentally allowing clients to set server-managed fields (id, created_at)
///   - Breaking the API when database schema changes
///
/// In Go: you'd define separate request/response structs (same pattern)
/// In JS/TS: same — request DTOs, response DTOs, entity models
/// In Rust: the derive macros make it zero-boilerplate
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub price: Decimal,
    pub stock: i32,
}

/// Request body for updating a product.
///
/// KEY LESSON: Option<T> for partial updates (PATCH semantics)
/// =============================================================
/// Every field is `Option<T>`. If a field is `Some(value)`, update it.
/// If a field is `None`, leave it unchanged. This is standard PATCH behavior.
///
/// This is like Go's `*string` pointers for optional JSON fields, or
/// JavaScript's `undefined` checks: `if (req.name !== undefined) { ... }`.
///
/// In Rust, `Option<T>` is the standard way to represent "maybe present."
/// Serde automatically handles: missing JSON field → None, present → Some(value).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<Decimal>,
    pub stock: Option<i32>,
    pub status: Option<ProductStatus>,
}

/// Response body for product list (with pagination metadata).
///
/// KEY LESSON: Generic wrapper types
/// ==================================
/// `ProductListResponse` is a generic container for any list endpoint response.
/// This prevents repeating `total`, `page`, `per_page` in every list response type.
///
/// This is like Go generics (Go 1.18+): `type ListResponse[T any] struct { ... }`
/// or C++ templates: `template<typename T> struct ListResponse { ... }`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProductListResponse {
    pub products: Vec<Product>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ─── Database Query Functions ────────────────────────────────────────────────

/// Create a new product in the database.
///
/// KEY LESSON: sqlx::query_as! — compile-time checked SQL
/// ========================================================
/// `sqlx::query_as!(Product, "INSERT INTO products ...", ...)`
/// This macro:
///   1. Connects to your DATABASE_URL at compile time
///   2. Checks the SQL syntax
///   3. Verifies column names match the Product struct
///   4. Verifies parameter types match Postgres column types
///
/// This requires a running database during compilation! Without one, you'll
/// get a compile error. For CI/CD, use `sqlx prepare` to cache the schema.
///
/// If you don't want compile-time checking (e.g., no DB in CI), use
/// `sqlx::query_as::<_, Product>("...")` instead — same API but runtime checks.
///
/// We're using `query_as` (runtime-checked) initially because we don't have
/// Postgres running. Once the DB is set up, switch to `query_as!` for compile-time safety.
pub async fn create(pool: &PgPool, req: &CreateProductRequest) -> Result<Product, AppError> {
    let product = sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (name, description, price, stock)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.price)
    .bind(req.stock)
    .fetch_one(pool)
    .await?;

    Ok(product)
}

/// Get a product by ID (excluding soft-deleted products).
///
/// KEY LESSON: `r#"..."#` raw string literals
/// ===========================================
/// Rust's raw strings: `r#"..."#` — like JS template literals but without
/// interpolation. `r"text"` is the basic form. `r#"text with "quotes""#` is
/// the extended form (you can add more `#` for strings containing `#"`).
/// This prevents needing to escape SQL quotes: `'it''s'` → `r#"it's"#`.
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Product>, AppError> {
    let product = sqlx::query_as::<_, Product>(
        r#"
        SELECT * FROM products
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(pool) // KEY LESSON: fetch_optional returns Option<Product>
    .await?; // Returns None if no rows, Some(product) if found
    // Unlike Go's sql.ErrNoRows (which returns an error!), SQLx returns Option.
    // This is more ergonomic: you pattern-match on Some/None instead of checking
    // for a specific error type.

    Ok(product)
}

/// List products with pagination, optional status filter, and optional search.
///
/// KEY LESSON: Dynamic query building with SQLx
/// =============================================
/// SQLx supports building queries at runtime with `QueryBuilder`.
/// This avoids complex `WHERE 1=1 AND ...` patterns or string concatenation.
///
/// In Go: you'd use `strings.Builder` or a query builder library
/// In JS: Knex.js or other query builders
/// In Rust: `sqlx::QueryBuilder` provides safe, parameterized query building
pub async fn list(
    pool: &PgPool,
    page: i64,
    per_page: i64,
    status: Option<ProductStatus>,
    search: Option<&str>,
) -> Result<ProductListResponse, AppError> {
    // KEY LESSON: QueryBuilder pattern
    // =================================
    // sqlx::QueryBuilder::new("SELECT ... WHERE deleted_at IS NULL")
    //   .push(" AND status = ") .push_bind(status)
    //   .push(" ORDER BY created_at DESC LIMIT ") .push_bind(per_page)
    //   .build_query_as::<Product>()
    //   .fetch_all(&pool)
    //   .await?

    let offset = (page - 1) * per_page;

    // Build the count query (for total records)
    let mut count_builder =
        sqlx::QueryBuilder::new("SELECT COUNT(*) FROM products WHERE deleted_at IS NULL");

    if let Some(ref s) = status {
        count_builder.push(" AND status = ");
        count_builder.push_bind(s);
    }
    if let Some(ref q) = search {
        count_builder.push(" AND (name ILIKE ");
        count_builder.push_bind(format!("%{q}%"));
        count_builder.push(" OR description ILIKE ");
        count_builder.push_bind(format!("%{q}%"));
        count_builder.push(")");
    }

    let total: i64 = count_builder.build_query_scalar().fetch_one(pool).await?;

    // Build the data query
    let mut query_builder =
        sqlx::QueryBuilder::new("SELECT * FROM products WHERE deleted_at IS NULL");

    if let Some(ref s) = status {
        query_builder.push(" AND status = ");
        query_builder.push_bind(s);
    }
    if let Some(ref q) = search {
        query_builder.push(" AND (name ILIKE ");
        query_builder.push_bind(format!("%{q}%"));
        query_builder.push(" OR description ILIKE ");
        query_builder.push_bind(format!("%{q}%"));
        query_builder.push(")");
    }

    query_builder.push(" ORDER BY created_at DESC LIMIT ");
    query_builder.push_bind(per_page);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let products: Vec<Product> = query_builder.build_query_as().fetch_all(pool).await?;

    Ok(ProductListResponse {
        products,
        total,
        page,
        per_page,
    })
}

/// Update a product by ID. Only updates fields that are Some(value).
///
/// KEY LESSON: Dynamic UPDATE with SET clauses
/// ============================================
/// Since we don't know which fields the client wants to update, we build
/// the SET clause dynamically. Each `Some(value)` field gets a `column = $N`
/// clause added to the query.
///
/// This pattern is common in REST APIs with PATCH semantics.
/// In Go: same pattern with strings.Builder or an UPDATE builder
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateProductRequest,
) -> Result<Option<Product>, AppError> {
    // First, check if the product exists (and is not deleted)
    let existing = find_by_id(pool, id).await?;
    if existing.is_none() {
        return Ok(None);
    }

    // Build dynamic UPDATE query
    // KEY LESSON: The pattern for dynamic SQL: build SET clauses dynamically
    let mut builder = sqlx::QueryBuilder::new("UPDATE products SET ");

    // Track whether we've added any SET clauses yet
    let mut has_set = false;

    if let Some(ref name) = req.name {
        if has_set {
            builder.push(", ");
        } // KEY LESSON: `ref` pattern — borrow the value inside Option
        builder.push("name = ");
        builder.push_bind(name);
        has_set = true;
    }
    if let Some(ref description) = req.description {
        if has_set {
            builder.push(", ");
        }
        builder.push("description = ");
        builder.push_bind(description);
        has_set = true;
    }
    if let Some(price) = req.price {
        // KEY LESSON: Decimal is Copy! No `ref` needed for Copy types.
        // `Copy` types are copied by value, so `if let Some(price) = req.price`
        // copies the Decimal. For non-Copy types (String), use `ref`.
        if has_set {
            builder.push(", ");
        }
        builder.push("price = ");
        builder.push_bind(price);
        has_set = true;
    }
    if let Some(stock) = req.stock {
        if has_set {
            builder.push(", ");
        }
        builder.push("stock = ");
        builder.push_bind(stock);
        has_set = true;
    }
    if let Some(ref status) = req.status {
        if has_set {
            builder.push(", ");
        }
        builder.push("status = ");
        builder.push_bind(status);
        has_set = true;
    }

    // If no fields were provided for update, return the existing product unchanged
    if !has_set {
        return Ok(existing);
    }

    builder.push(" WHERE id = ");
    builder.push_bind(id);
    builder.push(" AND deleted_at IS NULL RETURNING *");

    let product = builder
        .build_query_as::<Product>()
        .fetch_optional(pool)
        .await?;

    Ok(product)
}

/// Soft-delete a product (set deleted_at = NOW()).
///
/// KEY LESSON: Soft delete vs hard delete
/// =======================================
/// Soft delete preserves data for:
///   - Audit trails (who deleted what, when)
///   - Data recovery (mistakes happen)
///   - Referential integrity (orders reference products)
///
/// Hard delete should be a separate admin-only endpoint (if needed).
pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE products
        SET deleted_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;

    // KEY LESSON: rows_affected() tells us if anything was actually deleted
    // If the product was already deleted (or didn't exist), rows_affected() returns 0.
    Ok(result.rows_affected() > 0)
}
