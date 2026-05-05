CREATE TABLE calendar_editors (
    calendar_id   BIGINT      NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
    user_id       BIGINT      NOT NULL REFERENCES users(id)     ON DELETE CASCADE,
    active        BOOLEAN     NOT NULL DEFAULT TRUE,
    suspended_at  TIMESTAMP   NULL,
    joined_at     TIMESTAMP   NOT NULL DEFAULT NOW(),
    PRIMARY KEY (calendar_id, user_id)
);

CREATE INDEX idx_calendar_editors_active
    ON calendar_editors (calendar_id)
    WHERE active = true;

CREATE INDEX idx_calendar_editors_user_active
    ON calendar_editors (user_id)
    WHERE active = true;
