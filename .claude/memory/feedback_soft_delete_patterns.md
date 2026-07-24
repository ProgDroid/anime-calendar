---
name: Soft delete patterns for this codebase
description: How soft deletes are implemented here — partial indexes, cascade behaviour, audit trail, transaction scope changes
type: feedback
originSessionId: 6affb378-d634-41e7-873e-fb9eed26e218
---
Soft deletes in this project follow a specific pattern. Use this as the template for any future schema additions.

**Why:** Implemented 2026-04-12 for `users` and `calendars` to support audit trails and allow email/token reuse after account deletion.

## Schema pattern
- Add `deleted_at TIMESTAMP NULL DEFAULT NULL` to the table
- Drop the simple `UNIQUE` constraint on any reuse-sensitive column (e.g. `email`, `subscription_token`)
- Replace with a partial unique index: `CREATE UNIQUE INDEX ... WHERE deleted_at IS NULL`
- Add a sparse audit index: `CREATE INDEX ... ON t (deleted_at) WHERE deleted_at IS NOT NULL`

## Query pattern
- Every SELECT must add `AND deleted_at IS NULL` — there is no global filter; each query is explicit
- UPDATE queries also add `AND deleted_at IS NULL` (prevents updating already-deleted rows)
- Soft-delete is `UPDATE ... SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL`

## Cascade behaviour
- `delete_user` cascades: soft-deletes all user's calendars first, then the user, in a single transaction
- `delete_calendar` does NOT cascade to `calendar_items` — those rows are kept as an audit trail

## Transaction scope changes when moving hard → soft delete
- `delete_calendar` (hard): needed a transaction because two tables were touched (calendar_items + calendars)
- `delete_calendar` (soft): no transaction needed — only calendars is touched; calendar_items untouched
- `delete_user` (hard): single table, no transaction
- `delete_user` (soft): needs a new transaction because it must also cascade to calendars

## How to apply
Follow this exact pattern for any new soft-deleteable entity. After adding queries, regenerate `.sqlx/` against the live DB after applying the migration.
