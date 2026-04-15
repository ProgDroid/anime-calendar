-- Add password_reset_tokens table for password reset flow.
--
-- Strategy:
--   - token_hash: SHA-256 hash of the random token (256-bit, base64-encoded).
--     The actual token is sent to the user via email; only the hash is stored.
--   - expires_at: timestamp when the token becomes invalid.
--   - used_at: timestamp when the token was used to reset the password (NULL if not used).
--   - Each reset generates a new token; old tokens are invalidated by expiry or soft deletion.
--
-- Constraints:
--   - ON DELETE CASCADE ensures tokens are cleaned up when a user is deleted.
--   - token_hash is UNIQUE to prevent token reuse across different reset flows.
--   - Indexes on token_hash (for lookups) and user_id (for cleanup/audit).

CREATE TABLE password_reset_tokens (
    id          SERIAL       PRIMARY KEY,
    user_id     INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT         NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prt_token_hash ON password_reset_tokens(token_hash);
CREATE INDEX idx_prt_user_id    ON password_reset_tokens(user_id);
