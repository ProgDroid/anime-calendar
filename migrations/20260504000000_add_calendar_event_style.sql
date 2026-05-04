-- Calendars get a per-calendar "event style" knob: timed (DTSTART:datetime)
-- vs all_day (DTSTART;VALUE=DATE). Default 'timed' preserves existing
-- rendering. Phase 1 ships the column; Phase 2 wires the toggle and
-- consumer logic into ics_export.
ALTER TABLE calendars
    ADD COLUMN event_style TEXT NOT NULL DEFAULT 'timed'
    CHECK (event_style IN ('timed', 'all_day'));
