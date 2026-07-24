---
name: Don't use `#[sqlx::test]` — use the shared `test_pool()` helper
description: The codebase deliberately avoids `#[sqlx::test]` because of its 20-connection setup pool limit. All DB tests use `test_pool()` (shared) or `test_tx()` (rollback) from `server/src/test_helpers.rs`.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
When writing DB-touching tests in this codebase, use `crate::test_helpers::test_pool()` (shared pool) or `crate::test_helpers::test_tx()` (transaction rollback). **Do NOT use `#[sqlx::test]`** — it creates a per-test database which (a) blows past sqlx's 20-connection setup pool limit on parallel runs and (b) appears to mis-configure migrations on this codebase, causing tests to fail with `relation "users" does not exist`.

**Why:** Documented at the top of `server/src/test_helpers.rs`:

> Rather than using `#[sqlx::test]` (which creates a new Postgres database per test and is limited by a 20-connection setup pool), mapper tests use a shared DATABASE_URL database and wrap each test in a transaction that is always rolled back.

During Phase 1.2 of the monetisation refinement (2026-05-04), the implementer for `services::ics_export::tests` and `services::frozen_ics::tests` used `#[sqlx::test]` against this convention. All 9 new tests failed with `relation "users" does not exist`. Phase 1.1's tests had used `test_pool()` correctly. The brief had cited P1.1's pattern but the implementer didn't translate it.

**How to apply:**
- For mapper unit tests with no concurrent state: use `test_tx()` (rollback after).
- For service tests where the service uses its own internal connection / transaction (so an outer transaction wouldn't be visible): use `test_pool()` with explicit cleanup. Pattern:
  ```rust
  #[tokio::test]
  async fn my_test() {
      let pool = crate::test_helpers::test_pool().await;
      // seed with a unique-per-run username/email/id to avoid parallel collisions
      let user_id = seed_user(&pool).await;
      // ... assertions ...
      // explicit cleanup (DELETE FROM ...) so the next run starts clean
  }
  ```
- For controller integration tests: use `test_pool()` and the existing `App::new()` test helper pattern in `controllers/calendar.rs`.
- Dispatching implementer briefs: ALWAYS include the line "use `crate::test_helpers::test_pool()`, NOT `#[sqlx::test]`" in the test-pattern guidance.
