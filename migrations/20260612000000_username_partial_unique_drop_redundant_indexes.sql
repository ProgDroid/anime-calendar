-- Step 6 (audit follow-up): align users.username with the soft-delete model
-- and drop two redundant indexes. DDL-only — no sqlx::query! text changes, so
-- no .sqlx regeneration is required.
--
-- F2-24: users.username was left as a full UNIQUE when email and
--   subscription_token were converted to partial uniques (WHERE deleted_at
--   IS NULL) back in 20260412_add_soft_deletes. A soft-deleted account
--   therefore reserved its username forever. Convert it to the same
--   partial-unique shape so the handle is freed on soft delete. Safe: no
--   code looks a user up by username (all lookups are by id / email, both
--   already filtered by deleted_at IS NULL), so there is no row-ambiguity;
--   an *active* duplicate username is still rejected by the partial unique.
--
-- F2-25: drop two redundant indexes:
--   - idx_calendar_items_calendar_id: (calendar_id) is a left-prefix of the
--     PRIMARY KEY (calendar_id, item_id) AND of idx_calendar_items_calendar_added
--     (calendar_id, added_at DESC) — doubly redundant.
--   - idx_calendars_subscription_token: shadowed by the partial unique
--     idx_calendars_subscription_token_active; token lookups only ever target
--     active (non-deleted) calendars.

-- F2-24 ---------------------------------------------------------------------
-- Drop the implicit unique index created by `username TEXT NOT NULL UNIQUE`
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_username_key;

-- Enforce uniqueness only among non-deleted rows
CREATE UNIQUE INDEX idx_users_username_active
    ON users (username)
    WHERE deleted_at IS NULL;

-- F2-25 ---------------------------------------------------------------------
DROP INDEX IF EXISTS idx_calendar_items_calendar_id;
DROP INDEX IF EXISTS idx_calendars_subscription_token;
