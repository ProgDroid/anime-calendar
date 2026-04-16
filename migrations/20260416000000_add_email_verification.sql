-- Add email_verified_at to users.
-- NULL = unverified; NOT NULL = verified at that timestamp.
--
-- Backfill: existing users are treated as already verified so they are not
-- locked out after deploy. Only accounts created after this migration must
-- go through the verification flow.
--
-- email_verification_tokens stores short-lived one-time tokens:
--   - token_hash: SHA-256 hex of the raw token sent in the email link.
--   - expires_at: 24-hour window; expired tokens are rejected.
--   - ON DELETE CASCADE removes tokens when the user is deleted.
--   - UNIQUE on token_hash prevents two concurrent tokens resolving the same hash.

ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMP;

UPDATE users SET email_verified_at = created_at WHERE email_verified_at IS NULL;

CREATE TABLE email_verification_tokens (
    id          SERIAL       PRIMARY KEY,
    user_id     INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT         NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_evtk_user_id    ON email_verification_tokens(user_id);
