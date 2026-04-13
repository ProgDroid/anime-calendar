-- Add subscription_token to calendars for live iCal feed URLs.
-- Each token is a 64-char hex string (256-bit entropy), unique per calendar.
-- Owners paste the resulting URL into Google Calendar / any iCal-compatible app.

ALTER TABLE calendars
    ADD COLUMN IF NOT EXISTS subscription_token TEXT NOT NULL DEFAULT '';

-- Backfill existing rows. gen_random_bytes is available in PostgreSQL 13+.
-- For older versions: CREATE EXTENSION IF NOT EXISTS pgcrypto;
UPDATE calendars
SET subscription_token = encode(gen_random_bytes(32), 'hex')
WHERE subscription_token = '';

ALTER TABLE calendars
    DROP CONSTRAINT IF EXISTS calendars_subscription_token_unique;

ALTER TABLE calendars
    ALTER COLUMN subscription_token SET DEFAULT encode(gen_random_bytes(32), 'hex'),
    ADD CONSTRAINT calendars_subscription_token_unique UNIQUE (subscription_token);
