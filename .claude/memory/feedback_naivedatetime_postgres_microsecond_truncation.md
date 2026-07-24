---
name: NaiveDateTime → Postgres TIMESTAMP truncates nanoseconds, breaks round-trip equality
description: Tests that round-trip Utc::now().naive_utc() through a TIMESTAMP column and assert_eq! the result are flaky — chrono carries nanos (9 digits), Postgres TIMESTAMP keeps only microseconds (6 digits).
type: feedback
originSessionId: c52b64cd-a699-4ba2-b082-946c535a03c2
---
When a test inserts a `NaiveDateTime` into a Postgres `TIMESTAMP` column and reads it back, do **not** assert equality against the original value if you built it from `Utc::now().naive_utc()` (or any constructor that retains sub-microsecond precision). Use a date literal — `chrono::NaiveDate::from_ymd_opt(YYYY, M, D).unwrap().and_hms_opt(H, M, S).unwrap()` — or explicitly truncate nanos before inserting.

**Why:** Postgres `TIMESTAMP` (without an explicit `(p)` precision) stores microseconds (6 digits). chrono's `NaiveDateTime` carries nanoseconds (9 digits). The round-trip drops the bottom 3 digits. A test that runs once and happens to land on a microsecond boundary (last 3 nanos = 0) passes, but the same test on the next run fails: `left: ...760992  right: ...760992800`. Hit this on `bump_expiry_only_affects_pending_rows` in commit `cd90e08`; fixed in `bb9adab`.

**How to apply:**
- Test fixtures that need a "future deadline" or any timestamp the test will compare via `assert_eq!`: build it from a date literal, not `Utc::now() + Duration::days(N)`.
- If the test does NOT compare the round-tripped value (just uses it for ordering / WHERE clauses), `Utc::now().naive_utc()` is fine.
- This is a different concern from `feedback_sqlx_timestamp_types` — that memory is about which Rust type maps to which Postgres column type. This memory is about precision *within* the `TIMESTAMP` ↔ `NaiveDateTime` mapping.
- The same flake will appear with TIMESTAMPTZ ↔ DateTime<Utc> for the same reason. Same fix: build from a date literal in tests.
