-- anime-calendar baseline schema
-- PostgreSQL 13+
--
-- This file reflects the current database state after all migrations have
-- been applied. It is kept in sync manually whenever a migration changes the
-- schema. Use it to spin up a fresh database without replaying every migration.
--
-- To apply:
--   psql -U <user> -d <database> -f schema.sql
--
-- To run migrations against an existing database instead:
--   cargo sqlx migrate run

-- ---------------------------------------------------------------------------
-- Custom types
-- ---------------------------------------------------------------------------

CREATE TYPE language AS ENUM ('english', 'native', 'romaji');

CREATE TYPE theme AS ENUM ('light', 'dark');

CREATE TYPE site_language AS ENUM ('en', 'pt');

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    id             SERIAL      PRIMARY KEY,
    username       TEXT        NOT NULL,
    email          TEXT        NOT NULL,
    -- NULL for OAuth-only accounts (no password set)
    password_hash  TEXT,
    created_at     TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- NULL = active; set to NOW() on deletion (soft delete)
    deleted_at     TIMESTAMP   NULL     DEFAULT NULL
);

-- Uniqueness enforced only among active (non-deleted) users
-- so an email can be reused after account deletion
CREATE UNIQUE INDEX idx_users_email_active
    ON users (email)
    WHERE deleted_at IS NULL;

-- Sparse index for admin / audit queries
CREATE INDEX idx_users_deleted_at
    ON users (deleted_at)
    WHERE deleted_at IS NOT NULL;

-- ---------------------------------------------------------------------------
-- calendars
-- ---------------------------------------------------------------------------

CREATE TABLE calendars (
    id                 SERIAL      PRIMARY KEY,
    name               TEXT        NOT NULL,
    language           language    NOT NULL DEFAULT 'english',
    user_id            INTEGER     NOT NULL REFERENCES users (id),
    -- Persistent iCal feed token — 43-char alphanumeric, 256-bit entropy.
    -- Stable across edits so existing subscriptions keep working.
    subscription_token TEXT        NOT NULL
                                   DEFAULT encode(gen_random_bytes(32), 'hex'),
    created_at         TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- NULL = active; set to NOW() on deletion (soft delete)
    deleted_at         TIMESTAMP   NULL     DEFAULT NULL
);

CREATE INDEX idx_calendars_user_id
    ON calendars (user_id);

-- Uniqueness enforced only among active (non-deleted) calendars
-- so a subscription token can be reused after calendar deletion
CREATE UNIQUE INDEX idx_calendars_subscription_token_active
    ON calendars (subscription_token)
    WHERE deleted_at IS NULL;

-- Sparse index for admin / audit queries
CREATE INDEX idx_calendars_deleted_at
    ON calendars (deleted_at)
    WHERE deleted_at IS NOT NULL;

-- ---------------------------------------------------------------------------
-- calendar_items
-- Stores the many-to-many relationship between calendars and AniList item IDs.
-- item_id is an external AniList ID; no local items table exists.
-- ---------------------------------------------------------------------------

CREATE TABLE calendar_items (
    calendar_id  INTEGER  NOT NULL REFERENCES calendars (id),
    item_id      INTEGER  NOT NULL,
    PRIMARY KEY (calendar_id, item_id)
);

CREATE INDEX idx_calendar_items_calendar_id
    ON calendar_items (calendar_id);

-- ---------------------------------------------------------------------------
-- user_settings
-- One row per user, upserted on every settings save.
-- ---------------------------------------------------------------------------

CREATE TABLE user_settings (
    user_id                    INTEGER        PRIMARY KEY REFERENCES users (id),
    theme_preference           theme          NOT NULL DEFAULT 'dark',
    language_preference        site_language  NOT NULL DEFAULT 'en',
    title_language_preference  language       NOT NULL DEFAULT 'english',
    -- IANA timezone name, e.g. "America/Sao_Paulo"
    timezone                   TEXT           NOT NULL DEFAULT 'UTC',
    created_at                 TIMESTAMP      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                 TIMESTAMP      NOT NULL DEFAULT CURRENT_TIMESTAMP
);
