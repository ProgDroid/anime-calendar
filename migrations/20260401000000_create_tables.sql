-- Baseline schema — creates all tables in the state they were in
-- before the first incremental migration (20260408_add_subscription_token).
--
-- Used by sqlx::test to spin up isolated test databases from scratch.
-- Production databases were set up manually before migrations began,
-- so this migration is a no-op on those (all tables already exist).
--
-- NOTE: sqlx refuses to apply a migration whose version is lower than the
-- highest already-applied version, so this file never runs on production.

-- ---------------------------------------------------------------------------
-- Extensions
-- ---------------------------------------------------------------------------

-- pgcrypto provides gen_random_bytes(), used by 20260408 for token defaults.
-- It is a trusted extension in PostgreSQL 14+, so the database owner can
-- install it without superuser privileges.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ---------------------------------------------------------------------------
-- Custom types
-- ---------------------------------------------------------------------------

CREATE TYPE language AS ENUM ('english', 'native', 'romaji');

CREATE TYPE theme AS ENUM ('light', 'dark');

CREATE TYPE site_language AS ENUM ('en', 'pt');

-- ---------------------------------------------------------------------------
-- users
-- email is UNIQUE here; migration 20260412 replaces this constraint with
-- a partial unique index (WHERE deleted_at IS NULL).
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    id             SERIAL      PRIMARY KEY,
    username       TEXT        NOT NULL UNIQUE,
    email          TEXT        NOT NULL UNIQUE,
    password_hash  TEXT,
    created_at     TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------------------------
-- calendars
-- subscription_token is added by migration 20260408.
-- ---------------------------------------------------------------------------

CREATE TABLE calendars (
    id         SERIAL    PRIMARY KEY,
    name       TEXT      NOT NULL,
    language   language  NOT NULL DEFAULT 'english',
    user_id    INTEGER   NOT NULL REFERENCES users (id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------------------------
-- calendar_items
-- ---------------------------------------------------------------------------

CREATE TABLE calendar_items (
    calendar_id  INTEGER  NOT NULL REFERENCES calendars (id),
    item_id      INTEGER  NOT NULL,
    PRIMARY KEY (calendar_id, item_id)
);

-- ---------------------------------------------------------------------------
-- user_settings
-- ---------------------------------------------------------------------------

CREATE TABLE user_settings (
    user_id                    INTEGER        PRIMARY KEY REFERENCES users (id),
    theme_preference           theme          NOT NULL DEFAULT 'dark',
    language_preference        site_language  NOT NULL DEFAULT 'en',
    title_language_preference  language       NOT NULL DEFAULT 'english',
    timezone                   TEXT           NOT NULL DEFAULT 'UTC',
    created_at                 TIMESTAMP      NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at                 TIMESTAMP      NOT NULL DEFAULT CURRENT_TIMESTAMP
);
