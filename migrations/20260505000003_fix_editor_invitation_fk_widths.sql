-- Align FK column widths to their referenced PKs.
--
-- 20260505000001 / 20260505000002 declared calendar_id, user_id, inviter_id
-- as BIGINT, but the referenced PKs (calendars.id, users.id) are SERIAL
-- (= INTEGER). PostgreSQL accepts the cross-width FK via implicit coercion,
-- but sqlx::query! reads pg_attribute and types BIGINT columns as i64 — which
-- won't construct entities (CalendarEditor, CalendarInvitation) whose fields
-- are i32 to match the rest of the codebase.
--
-- Forward-only fix: drop FKs, ALTER TYPE INTEGER USING column::int, recreate
-- FKs. Tables are empty in dev (Phase 0 has not exposed any handler yet), so
-- the cast is lossless.

-- calendar_editors ---------------------------------------------------------

ALTER TABLE calendar_editors
    DROP CONSTRAINT calendar_editors_calendar_id_fkey,
    DROP CONSTRAINT calendar_editors_user_id_fkey;

ALTER TABLE calendar_editors
    ALTER COLUMN calendar_id TYPE INTEGER USING calendar_id::int,
    ALTER COLUMN user_id     TYPE INTEGER USING user_id::int;

ALTER TABLE calendar_editors
    ADD CONSTRAINT calendar_editors_calendar_id_fkey
        FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    ADD CONSTRAINT calendar_editors_user_id_fkey
        FOREIGN KEY (user_id)     REFERENCES users(id)     ON DELETE CASCADE;

-- calendar_invitations -----------------------------------------------------
-- id stays BIGSERIAL (i64) — it's the only column where BIGINT is intentional.

ALTER TABLE calendar_invitations
    DROP CONSTRAINT calendar_invitations_calendar_id_fkey,
    DROP CONSTRAINT calendar_invitations_inviter_id_fkey;

ALTER TABLE calendar_invitations
    ALTER COLUMN calendar_id TYPE INTEGER USING calendar_id::int,
    ALTER COLUMN inviter_id  TYPE INTEGER USING inviter_id::int;

ALTER TABLE calendar_invitations
    ADD CONSTRAINT calendar_invitations_calendar_id_fkey
        FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    ADD CONSTRAINT calendar_invitations_inviter_id_fkey
        FOREIGN KEY (inviter_id)  REFERENCES users(id)     ON DELETE CASCADE;
