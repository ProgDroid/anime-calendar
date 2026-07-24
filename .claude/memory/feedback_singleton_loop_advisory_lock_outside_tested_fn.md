---
name: feedback_singleton_loop_advisory_lock_outside_tested_fn
description: "For a singleton background loop, take the pg_try_advisory_lock in a wrapper OUTSIDE the unit-tested pass fn, or parallel tests contend on the global key"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ab83a158-ffc2-4fb3-a1dd-2857b860d545
---

To make a singleton background loop multi-replica safe (M-9, reconcile loop), wrap each tick in a session-level `pg_try_advisory_lock(SENTINEL_KEY)` taken on a **dedicated held connection** (`mapper.pool().acquire()`), run the pass, then `pg_advisory_unlock`. Losers skip the tick; the lock auto-releases if the connection/process dies.

**Critical placement:** put the lock in a NEW wrapper (`run_guarded_pass`) that calls the existing pass fn (`run_pass`) — do **not** put it inside `run_pass` itself. `run_pass` is exercised directly by parallel `#[tokio::test]` cases on a shared pool; a global advisory key inside it would make concurrent tests contend and spuriously skip, breaking assertions. The CLI one-shot path also calls the unlocked fn.

**Why:** session-scoped advisory locks are global to the DB, so any test that runs the locked fn in parallel competes for the one key. Keeping the lock in the production-only wrapper isolates it. **How to apply:** use the non-macro `sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")` form (no `.sqlx` regen needed); test the skip-while-locked behaviour by holding the lock on a separate connection. Distinct from [[feedback_audit_lock_in_tx_consistency]] (that's per-user `pg_advisory_xact_lock` inside a tx; this is a session-scoped singleton-loop guard).
