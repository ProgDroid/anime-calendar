-- Per-user reminder offsets, expressed in minutes before episode air time.
-- Default ARRAY[30] preserves a single 30-minute heads-up — matches the
-- Free-tier always-emit baseline. Pro users can store up to 5 entries;
-- Free user values are stored but ignored at .ics emission time.
ALTER TABLE user_settings
    ADD COLUMN reminder_offsets_minutes INTEGER[] NOT NULL DEFAULT ARRAY[30];
