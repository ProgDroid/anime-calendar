-- Frozen .ics blob for Free-tier subscribe URLs. NULL means "no frozen
-- copy" — could be Free user who never paid, or a previously-paid user
-- whose webhook hasn't fired yet. Subscribe endpoint disambiguates via
-- subscription history.
ALTER TABLE calendars
    ADD COLUMN frozen_subscribe_ics TEXT;
