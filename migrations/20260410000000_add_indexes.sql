-- Add indexes on hot query paths identified in the performance audit.
-- All three columns appear in WHERE or JOIN clauses on every request.

CREATE INDEX IF NOT EXISTS idx_calendars_user_id
    ON calendars (user_id);

CREATE INDEX IF NOT EXISTS idx_calendars_subscription_token
    ON calendars (subscription_token);

CREATE INDEX IF NOT EXISTS idx_calendar_items_calendar_id
    ON calendar_items (calendar_id);
