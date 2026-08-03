-- SQLx Migration: Create products table
-- Run with: sqlx migrate run (or automatically via sqlx::migrate!())
-- This is like golang-migrate's .up.sql files, or Flyway's V1__create_products.sql

-- EXTENSION: Enable UUID generation
-- pgcrypto provides gen_random_uuid() — we'll use this for default IDs
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ENUM: product_status
-- KEY LESSON: Postgres enums vs Rust enums
-- We create a Postgres enum type to mirror our Rust ProductStatus enum.
-- sqlx can map between them automatically with #[sqlx(type_name = "product_status")]
CREATE TYPE product_status AS ENUM ('active', 'draft', 'discontinued', 'out_of_stock');

-- TABLE: products
CREATE TABLE products (
    -- UUID primary key, auto-generated
    -- In Rust: uuid::Uuid
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Product name — NOT NULL, max 255 chars
    -- In Rust: String (heap-allocated, owned)
    name VARCHAR(255) NOT NULL,

    -- Product description — can be empty, but NOT NULL (use empty string, not null)
    -- KEY LESSON: Rust uses Option<String> for nullable. In SQL: NULL = None, NOT NULL = String.
    -- This is a conscious design choice: avoid null where empty string suffices.
    description TEXT NOT NULL DEFAULT '',

    -- Price — stored as NUMERIC(10,2) for exact decimal
    -- NEVER use FLOAT/DOUBLE for money! Floating point rounding causes bugs.
    -- In Rust: rust_decimal::Decimal (not f64!)
    -- NUMERIC(10,2) means: 10 total digits, 2 after decimal point
    -- Max value: 99,999,999.99 — sufficient for most products
    price NUMERIC(10, 2) NOT NULL CHECK (price > 0),

    -- Stock quantity — INTEGER is fine for inventory
    -- In Rust: i32 (32-bit signed integer, like C++ int)
    stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),

    -- Product status — uses our custom enum type
    -- In Rust: ProductStatus enum
    status product_status NOT NULL DEFAULT 'active',

    -- Timestamps — WITH TIME ZONE stores UTC
    -- In Rust: chrono::DateTime<chrono::Utc>
    -- KEY LESSON: Always store UTC in databases, convert to local timezone at display layer
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Soft delete: instead of actually deleting rows, we mark them as deleted
    -- Benefits: data recovery, audit trail, referential integrity
    -- Trade-off: queries must always filter WHERE deleted_at IS NULL
    deleted_at TIMESTAMPTZ
);

-- INDEXES for common query patterns
-- Index on name for search
CREATE INDEX idx_products_name ON products (name) WHERE deleted_at IS NULL;
-- Index on status for filtering by status
CREATE INDEX idx_products_status ON products (status) WHERE deleted_at IS NULL;
-- Index on created_at for ordering by newest
CREATE INDEX idx_products_created_at ON products (created_at DESC) WHERE deleted_at IS NULL;
-- Composite index for filtered listing (status + created_at)
CREATE INDEX idx_products_status_created ON products (status, created_at DESC) WHERE deleted_at IS NULL;

-- TRIGGER: auto-update updated_at timestamp
-- Every time a row is updated, set updated_at to NOW()
-- This is like ActiveRecord's updated_at in Rails, or GORM's auto-timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- DOWN migration (reverse operations)
-- SQLx doesn't automatically run down migrations — you create a separate file
-- or run the down SQL manually. We'll add the down here as a comment for reference:
--
-- DROP TRIGGER IF EXISTS update_products_updated_at ON products;
-- DROP FUNCTION IF EXISTS update_updated_at_column();
-- DROP TABLE IF EXISTS products;
-- DROP TYPE IF EXISTS product_status;