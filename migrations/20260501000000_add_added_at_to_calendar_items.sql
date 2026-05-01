ALTER TABLE calendar_items
  ADD COLUMN added_at TIMESTAMP NOT NULL DEFAULT NOW();

CREATE INDEX idx_calendar_items_calendar_added
  ON calendar_items (calendar_id, added_at DESC);
