---
name: sqlx offline cache maintenance
description: Reminder to regenerate and commit .sqlx/ whenever sqlx query macros change
type: feedback
originSessionId: 6affb378-d634-41e7-873e-fb9eed26e218
---
After any change to the **SQL string** in a `sqlx::query!`, `sqlx::query_as!`, or `sqlx::query_scalar!` call in the server crate, regenerate the offline cache and commit it.

**Only the SQL text matters — pure Rust refactors around a query need NO regen.** `.sqlx/` is keyed on the query string, not the surrounding Rust. Renaming the enclosing fn, dropping a `.map_err`, extracting a helper, moving a query between files, or **deleting a duplicate query** (whose identical twin still exists elsewhere) does not change any cache entry. Don't reflexively run `cargo sqlx prepare` after such refactors — it's wasted, and on this machine it would also drag you into the dev-DB migration-checksum drift (`project_dev_db_migration_drift`) for nothing. Concrete case: audit Step 5 (commit `cc64ad6`, 2026-06-11) renamed 37 mapper helpers, extracted an email helper, dropped `format!` wrappers, and deleted a duplicated calendar-fetch query — zero `.sqlx/` changes, CI green without regen.

**Standard command** (production + non-test code):
```bash
DATABASE_URL=postgresql://anime-calendar:anime@aegyptvault.local:9647/anime-calendar \
  AWS_LC_SYS_PREBUILT_NASM=1 cargo sqlx prepare --workspace
```

**When test-only queries are added** (inside `#[cfg(test)]` blocks), the standard command misses them because it runs `cargo check` which skips test targets. Use:
```bash
DATABASE_URL=postgresql://anime-calendar:anime@aegyptvault.local:9647/anime-calendar \
  AWS_LC_SYS_PREBUILT_NASM=1 cargo sqlx prepare --workspace -- --all-targets
```

The `-- --all-targets` flag was needed after adding `sqlx::query!` inside `#[cfg(test)]` controller test helpers. Without it, `SQLX_OFFLINE=true cargo test` fails with "no cached data for this query".

**Why:** CI runs with `SQLX_OFFLINE=true` and uses the committed `.sqlx/` directory for compile-time SQL type-checking. If `.sqlx/` is out of sync with the actual queries, CI will fail.

**How to apply:** Always commit `.sqlx/` alongside the query change. If the new query lives in a `#[cfg(test)]` block, use the `--all-targets` variant.

**Pitfall — dynamic queries bypass the cache silently:** Using `sqlx::query_scalar("SELECT ...")` (no `!`) instead of `sqlx::query_scalar!("SELECT ...")` compiles fine in offline mode but skips compile-time type checking entirely. The query will never appear in `.sqlx/` and schema drift goes undetected. Always use the macro forms (`query!`, `query_as!`, `query_scalar!`). Flag any dynamic query usage in code review.

---

## Worktree migration conflict workaround

When working in a git worktree branched before some migrations were applied (or before migration files were modified in main), `cargo sqlx migrate run` will fail with:
```
error: migration XXXXXXXXXX was previously applied but has been modified
```
This blocks the entire migrate run — even pending new migrations won't be applied.

**Root cause:** sqlx stores a checksum of each migration when applied. If the migration *file* was later edited in main (e.g., the `create_tables.sql` was patched to fix a column type), the worktree's copy has a different checksum from what the DB recorded.

**Workaround (2026-04-17):** Since `psql` was not available, applied the new migration's DDL directly via Python + psycopg2:
```python
import os, psycopg2, re
url = os.environ['DATABASE_URL']
# parse url, connect, check table existence, execute DDL if not present
conn = psycopg2.connect(...)
conn.autocommit = True
cur = conn.cursor()
cur.execute("CREATE TABLE refresh_tokens (...)")
```
Then ran `cargo sqlx prepare --workspace` normally (it only needs the table to exist, not a clean migrate history).

**Alternative with psql:**
```bash
psql "$DATABASE_URL" -f migrations/XXXXXXXXXX_your_migration.sql
```

**Note:** `cargo sqlx prepare` does NOT check migration checksums — it just needs the tables to exist in the live DB to validate query types. Only `cargo sqlx migrate run` enforces checksums.
