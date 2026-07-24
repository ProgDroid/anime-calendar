---
name: sqlx timestamp type mismatch — TIMESTAMP vs TIMESTAMPTZ
description: TIMESTAMPTZ maps to DateTime<Utc>, TIMESTAMP maps to NaiveDateTime; mismatching them causes E0308 at cargo sqlx prepare time
type: feedback
originSessionId: 3ec09e38-54a3-4383-9c58-30ad8b46dfa5
---
Use `TIMESTAMP` (without time zone) for new timestamp columns on tables where existing columns use `NaiveDateTime` in Rust (e.g. `users.created_at`, `users.updated_at`).

**Why:** sqlx maps `TIMESTAMPTZ` → `DateTime<Utc>` and `TIMESTAMP` → `NaiveDateTime`. If you add a `TIMESTAMPTZ` column but the entity struct has `Option<NaiveDateTime>`, the project compiles fine in `SQLX_OFFLINE=true` mode (cache hides the mismatch) but fails with `E0308: mismatched types` the first time `cargo sqlx prepare` runs against the live DB.

**How to apply:** Check existing timestamp columns before writing a migration. If the table uses `NaiveDateTime` in Rust, use `TIMESTAMP WITHOUT TIME ZONE` (or just `TIMESTAMP`) in the migration. Don't use `TIMESTAMPTZ` unless you're also updating the entity struct to `DateTime<Utc>`.
