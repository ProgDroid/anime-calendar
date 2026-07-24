---
name: When a plan introduces transactional locking, audit which connection each guarded operation runs on
description: Plan stubs that use pg_advisory_xact_lock often re-call existing service methods inside the lock arm, but those methods take the pool — they don't see the locked transaction. The lock is a no-op for race protection unless the guarded reads/writes use the locked connection.
type: feedback
originSessionId: 75332392-b9dc-4e06-b274-05f71e4842d6
---
When an implementer brief or plan stub introduces `pg_advisory_xact_lock` (or any other transactional lock), audit every operation inside the lock arm: each one MUST run on the same connection that holds the lock.

The codebase's existing service methods (`EntitlementService::assert_can_create_calendar`, `ShowCountService::count_distinct_for_user`, etc.) are pool-bound — they implicitly acquire a fresh connection per call. Calling them inside a `tx.begin() + pg_advisory_xact_lock + ...` arm does NOT serialize them on the lock. They observe the snapshot from a different connection, which means concurrent transactions can race the cap check.

**Why:** During Phase 1.3 of monetisation refinement (2026-05-04), the plan stub literally re-called `entitlement.assert_can_create_calendar(user_id).await?` inside the locked arm of the PUT /calendar handler — exactly the wrong shape. Two concurrent requests at cap-1 would both pass the pool-bound check, then both serialize on the lock and INSERT, ending at cap+1.

The fix was to add `_in_tx(conn, ...)` static helpers to `ShowCountService` and `EntitlementService` that take `&mut sqlx::PgConnection`, mirroring the existing `_in_tx` precedent in `SubscriptionMapper`. Inside the lock arm, the cap re-check goes through the `_in_tx` variants on `&mut *tx`, so it observes the locked transaction's pending state.

**How to apply:**
- Whenever a plan stub or brief introduces a transaction + lock, audit every single `await` inside the lock arm. Ask: does this call use `&mut *tx` (or `&mut **conn`)? If it uses `&self.pool` or any pool-bound API, the lock is a fiction for that operation.
- If the codebase doesn't already have `_in_tx` variants for what you need, add them. Pattern: take `conn: &mut sqlx::PgConnection`, run the same query against `conn`. The `feedback_in_tx_static_helper_pattern` memory documents the static-helper convention.
- This applies symmetrically to other locking mechanisms: `pg_advisory_lock`, `SELECT ... FOR UPDATE`, etc. Any read or write that observes "the locked state" must run on the locked connection.
- Briefs that introduce locking should call this out explicitly so the implementer doesn't just copy the plan stub verbatim.
