# Database rollback strategy

**Audit finding:** M-23 — the project ships up-only migrations with no
`down.sql` files. This document is the deliberate answer to that gap.

## Decision: backup / point-in-time recovery, not per-migration downs

The migrations in `migrations/` use sqlx's **simple** (up-only) style —
`<timestamp>_<name>.sql`, no paired `.down.sql`. We **keep** it that way and
recover via PostgreSQL backups / point-in-time recovery (PITR), rather than
maintaining reversible migrations. Rationale:

1. **Several migrations cannot be mechanically reversed without data loss** (see
   the table below). A naïve generated `down.sql` for them would silently drop
   rows or fail on duplicate keys — worse than having no down at all.
2. **Migrations are applied by the CLI only** (`sqlx migrate run` in CI, manual
   on dev). The server does **not** call `sqlx::migrate!()` at startup, so
   reversible migrations would buy nothing operationally.
3. **Converting to reversible style renames every file**, which would churn the
   committed `.sqlx/` cache and aggravate the known dev-DB **checksum drift**
   (see below) for zero runtime benefit.
4. PITR/backup is the industry-standard recovery path for a production Postgres
   anyway — you need backups regardless of migration style.

**Operational requirement:** the production database MUST have automated backups
with PITR (WAL archiving) enabled. That is the rollback mechanism. A bad deploy
is recovered by restoring to a timestamp just before the migration ran, not by
running a down migration.

## Migration inventory (21, in order)

Reverse-safety legend: ✅ additive/trivially reversible · ⚠️ **destructive or
lossy to reverse** — never reverse without a verified backup.

| # | Migration | What it does | Reverse |
|---|-----------|--------------|---------|
| 1 | `20260401000000_create_tables` | Baseline schema: users, calendars, calendar_items, user_settings; pgcrypto; enums | ✅ |
| 2 | `20260408000000_add_subscription_token` | Add + backfill `calendars.subscription_token`, UNIQUE | ✅ |
| 3 | `20260410000000_add_indexes` | Add 3 hot-path indexes | ✅ |
| 4 | `20260412000000_add_soft_deletes` | Add `deleted_at`; swap full UNIQUEs → partial-unique (email, subscription_token) | ⚠️ recreating full UNIQUE fails if soft-deleted dupes exist |
| 5 | `20260415000000_add_password_reset_tokens` | Create `password_reset_tokens` | ⚠️ drop loses live reset tokens |
| 6 | `20260415000001_remove_redundant_prt_index` | Drop redundant index | ✅ |
| 7 | `20260416000000_add_email_verification` | Add `email_verified_at` (backfilled), create `email_verification_tokens` | ✅ / ⚠️ token table drop |
| 8 | `20260416000001_add_refresh_tokens` | Create `refresh_tokens` | ⚠️ drop logs everyone out |
| 9 | `20260430000000_add_accent_preference` | Add `accent` enum + `accent_preference` | ✅ |
| 10 | `20260501000000_add_added_at_to_calendar_items` | Add `added_at` + composite index | ✅ |
| 11 | `20260501000001_add_subscriptions` | Create `subscriptions` (Stripe mirror) | ⚠️ drop loses entitlement state |
| 12 | `20260501000002_add_stripe_events` | Create `stripe_events` idempotency table | ⚠️ drop reopens webhook replay |
| 13 | `20260504000000_add_calendar_event_style` | Add `event_style` (CHECK) | ✅ |
| 14 | `20260504000001_add_calendar_frozen_subscribe_ics` | Add `frozen_subscribe_ics` | ✅ |
| 15 | `20260504000002_add_user_settings_reminder_offsets` | Add `reminder_offsets_minutes[]` | ✅ |
| 16 | `20260505000000_add_calendars_meta_version` | Add `meta_version` | ✅ |
| 17 | `20260505000001_create_calendar_editors` | Create `calendar_editors` bridge | ⚠️ drop loses sharing grants |
| 18 | `20260505000002_create_calendar_invitations` | Create citext ext + `calendar_invitations` | ⚠️ drop loses invites; ext drop cascades |
| 19 | `20260505000003_fix_editor_invitation_fk_widths` | ALTER FK cols BIGINT→INTEGER via cast | ⚠️ reverse cast risks precision; FK churn |
| 20 | `20260611000000_add_hot_path_indexes` | Add 2 indexes (F2-14) | ✅ |
| 21 | `20260612000000_username_partial_unique_drop_redundant_indexes` | username full UNIQUE → partial; drop 2 redundant indexes (F2-24/F2-25) | ⚠️ restoring full UNIQUE fails if soft-deleted username dupes exist |

If a future migration is genuinely additive and you want a fast forward-only
"undo" for a specific deploy, you may hand-author a one-off reversal SQL script
in `docs/` and apply it manually — but that is an exception, not a per-migration
contract, and it must be reviewed for the ⚠️ hazards above.

## Dev-DB checksum-drift constraint (read before touching migrations)

`sqlx migrate run` is **blocked on the dev database** by a pre-existing
`_sqlx_migrations` checksum drift. New migrations are applied to dev manually via
a psycopg2 bypass that executes the SQL and writes the bookkeeping row (SHA-384
checksum) directly. See the project memory `project_dev_db_migration_drift` for
the exact recipe. This is why:

- New migrations may land in a commit "not yet applied to dev" — they apply on
  the next fresh DB / CI run, or via the manual bypass.
- We do **not** rename existing migration files (e.g. to `.up.sql`/`.down.sql`),
  which would invalidate every recorded checksum and deepen the drift.

## TL;DR for an operator rolling back a bad deploy

1. Stop the deploy / take the app offline if the migration corrupted data.
2. Restore the production database from backup / PITR to a timestamp **just
   before** the migration executed.
3. Redeploy the previous application version.
4. Do **not** attempt to "reverse" a ⚠️ migration by hand against live data.
