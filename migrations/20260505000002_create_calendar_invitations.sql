CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE calendar_invitations (
    id             BIGSERIAL PRIMARY KEY,
    calendar_id    BIGINT       NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    inviter_id     BIGINT       NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
    invitee_email  CITEXT       NOT NULL,
    token_hash     TEXT         NOT NULL,
    status         TEXT         NOT NULL DEFAULT 'pending',
    expires_at     TIMESTAMP    NOT NULL,
    sent_at        TIMESTAMP    NOT NULL DEFAULT NOW(),
    resolved_at    TIMESTAMP    NULL,
    CONSTRAINT calendar_invitations_status_chk
        CHECK (status IN ('pending','accepted','declined','revoked','expired','suspended'))
);

CREATE UNIQUE INDEX idx_calendar_invitations_pending_unique
    ON calendar_invitations (calendar_id, lower(invitee_email))
    WHERE status = 'pending';

CREATE INDEX idx_calendar_invitations_calendar_status
    ON calendar_invitations (calendar_id, status);

CREATE INDEX idx_calendar_invitations_token_hash
    ON calendar_invitations (token_hash)
    WHERE status = 'pending';
