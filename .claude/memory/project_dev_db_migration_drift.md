---
name: Dev DB migration checksum drift (recurred — migrate run treated as blocked again 2026-06-12)
description: Drift was resolved 2026-04-30 but recent audit work (2026-06-12) treats `sqlx migrate run` as blocked again and applies new migrations via the psycopg2 bypass with SHA-384 _sqlx_migrations bookkeeping
type: project
originSessionId: 7a8d378a-90cb-41a0-b043-addec4ba2a44
---
**Status:** RESOLVED on 2026-04-30 (Track 2 Pre-Task).

The dev DB previously had unresolved checksum drift on three migrations: `20260401000000`, `20260415000000`, `20260415000001`. Plain `sqlx migrate run` blocked on any new migration.

**Resolution (2026-04-30):** Updated the stored `checksum` BYTEA in `_sqlx_migrations` for each drifted version to the current sha384 of the file on disk via psycopg2:

```sql
UPDATE _sqlx_migrations SET checksum = decode('<hex>', 'hex') WHERE version = <version>;
```

Updated checksums:
- `20260401000000` → `0c7d088d697155be249472e84e7d99fa58d27dce33161961f746433c311176fa1ee6a0861f38cc6152de17bd70f5ed5f`
- `20260415000000` → `f5564bb8b8f489a6fdb2844ef2633bdf00cb186cb6842e7dd24f562b2a6ecd69bca908907c724b32743a4254fbd49fd0`
- `20260415000001` → `0c4b06c4fd933dbb4fb83ef9c262df9895d502a9c41f60d23136526ab823116c7d581247406b9d174f36ab86425f2219`

Verified with `cargo sqlx migrate info --source migrations` from repo root: all installed migrations show `/installed` with no checksum mismatch warnings.

**Original cause:** During Track 1 Task A1 the new `20260430000000_add_accent_preference.sql` migration was applied via the documented psycopg2 workaround (see `feedback_sqlx_offline_cache.md`). Earlier migration files had been edited in place at some point without re-syncing the stored checksums.

**Going forward:** Plain `cargo sqlx migrate run` now works for new migrations. The psycopg2 workaround is no longer required for normal flow — only if a migration needs to bypass sqlx for some reason.

**Note:** `cargo sqlx prepare` never checked migration checksums (only that tables/types exist), so query-macro work was unaffected throughout.

**Follow-up backfill (2026-04-30, Track 2 Pre-Task):** Two more migrations showed as `pending` even though their tables/columns already existed in the dev DB (features merged on main): `20260416000000_add_email_verification` and `20260416000001_add_refresh_tokens`. Schema verified to match the migration DDL exactly — `users.email_verified_at TIMESTAMP`, `email_verification_tokens` (id/user_id/token_hash UNIQUE/expires_at TIMESTAMPTZ/created_at TIMESTAMPTZ) with `idx_evtk_user_id`, and `refresh_tokens` (id/user_id/token_hash/expires_at TIMESTAMP/created_at TIMESTAMP/used_at TIMESTAMP) with both indexes. One non-blocking superset noted: dev DB also has an extra `idx_evtk_token_hash` index that's not in the migration file (redundant with the UNIQUE constraint). Backfilled `_sqlx_migrations` with sha384 checksums:
- `20260416000000` → `db3d0481f7cc061aceb352879f8b5aaa1834336e8ff796592571b5c94ae8bd4f503854ed198029da22a8dc8e110f7905`
- `20260416000001` → `5f9cf4dc349f6aedd6a2f0adbf332a46084b0c689521e25562d384e31b51dd3af86af1563e5e85db619e03ca273c7fc3`

`cargo sqlx migrate info --source ../migrations` from `server/` now shows all 9 versions as `/installed`.

**Regression / current practice (2026-06-12, Step 6 audit):** The "RESOLVED" status above is contradicted by recent work. F2-14 (Step 5) and Step 6 both treat `sqlx migrate run` as **blocked** again and apply new migrations via the psycopg2 bypass (likely because migrations added after 2026-04-30 — `20260501*`, `20260505*`, `20260611*`, `20260612*` — drifted or weren't all bookkept). Couldn't reconcile definitively this session: the installed `sqlx` CLI (`~/.cargo/bin/sqlx`) errors with `no driver found for URL scheme "postgres"` on `sqlx migrate info`, so a future session should resolve the true state first.

**Proven bypass recipe for a NEW migration (used for `20260612000000`):** connect with psycopg2 (creds in `database.toml`: host `aegyptvault.local`, port `9647`, db/user `anime-calendar`, pass `anime`), then in one transaction: (1) skip if the version row already exists, (2) `cur.execute(migration_sql)` (psycopg2 runs multi-statement DDL in one call), (3) insert bookkeeping: `INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) VALUES (%s, %s, NOW(), TRUE, %s, %s)` where `checksum = psycopg2.Binary(hashlib.sha384(migration_file_bytes).digest())` and `execution_time` is elapsed ns. Version = numeric filename prefix; description = filename remainder with `_`→spaces, `.sql` stripped. See also [[feedback_sqlx_offline_cache]].
