-- Add soft-delete support to users and calendars.
--
-- Strategy:
--   - Add `deleted_at TIMESTAMP NULL` to both tables.
--   - Replace the UNIQUE constraints on users.email and
--     calendars.subscription_token with partial unique indexes
--     (WHERE deleted_at IS NULL) so the same email / token can be
--     reused after an account or calendar is deleted.
--   - Add sparse indexes on deleted_at for efficient admin queries.
--
-- Application-layer behaviour:
--   - DELETE becomes UPDATE ... SET deleted_at = NOW()
--   - All SELECT queries add WHERE deleted_at IS NULL
--   - Deleting a user cascades a soft-delete to all their calendars
--   - calendar_items rows are kept intact as an audit trail

-- -------------------------------------------------------------------------
-- users
-- -------------------------------------------------------------------------

ALTER TABLE users ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL;

-- Drop the implicit unique index created by `email TEXT NOT NULL UNIQUE`
ALTER TABLE users DROP CONSTRAINT users_email_key;

-- Enforce uniqueness only among non-deleted rows
CREATE UNIQUE INDEX idx_users_email_active
    ON users (email)
    WHERE deleted_at IS NULL;

-- Sparse index for admin / audit queries on deleted users
CREATE INDEX idx_users_deleted_at
    ON users (deleted_at)
    WHERE deleted_at IS NOT NULL;

-- -------------------------------------------------------------------------
-- calendars
-- -------------------------------------------------------------------------

ALTER TABLE calendars ADD COLUMN deleted_at TIMESTAMP NULL DEFAULT NULL;

-- Drop the implicit unique index created by `subscription_token TEXT NOT NULL UNIQUE`
ALTER TABLE calendars DROP CONSTRAINT IF EXISTS calendars_subscription_token_key;

-- Enforce uniqueness only among non-deleted rows
CREATE UNIQUE INDEX idx_calendars_subscription_token_active
    ON calendars (subscription_token)
    WHERE deleted_at IS NULL;

-- Sparse index for admin / audit queries on deleted calendars
CREATE INDEX idx_calendars_deleted_at
    ON calendars (deleted_at)
    WHERE deleted_at IS NOT NULL;
