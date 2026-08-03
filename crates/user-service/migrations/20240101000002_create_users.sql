-- Migration: Create users and refresh_tokens tables
-- Phase 2: User Service — Authentication & Authorization
--
-- Key Design Decisions:
-- 1. password_hash stores Argon2 hashes (NOT plaintext, NOT bcrypt)
--    Argon2 won the Password Hashing Competition (2015) — resistant to
--    GPU/ASIC attacks via memory-hardness parameter.
-- 2. role uses a Postgres enum for type safety at the database level.
--    This prevents invalid role values from ever being stored.
--    Equivalent to Rust enums + sqlx::Type derive.
-- 3. refresh_tokens stores a HASH of the token, not the raw token.
--    Same principle as password hashing: if the DB is compromised,
--    attackers can't steal valid refresh tokens.
-- 4. ON DELETE CASCADE on refresh_tokens → users: if a user is deleted,
--    all their refresh tokens are automatically cleaned up.

-- Create user_role enum (Postgres native enum type)
-- In Rust, this maps to an enum with sqlx::Type derive + #[sqlx(type_name = "user_role")]
DO $$ BEGIN
    CREATE TYPE user_role AS ENUM ('user', 'admin');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       VARCHAR(255) NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,                -- Argon2 hash (includes salt)
    name        VARCHAR(255) NOT NULL,
    role        user_role NOT NULL DEFAULT 'user',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ                     -- Soft delete (NULL = active)
);

-- Index for email lookups (login, uniqueness check)
CREATE INDEX IF NOT EXISTS idx_users_email ON users (email) WHERE deleted_at IS NULL;

-- Index for role-based queries (e.g., "list all admins")
CREATE INDEX IF NOT EXISTS idx_users_role ON users (role) WHERE deleted_at IS NULL;

-- Refresh tokens table (one user can have multiple sessions)
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL,                  -- SHA-256 hash of the refresh token JWT
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for token lookup during refresh
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash
    ON refresh_tokens (token_hash)
    WHERE revoked = FALSE;

-- Index for cleaning up expired tokens (batch job)
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires
    ON refresh_tokens (expires_at)
    WHERE revoked = FALSE;

-- Index for finding all active sessions for a user
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user
    ON refresh_tokens (user_id)
    WHERE revoked = FALSE;