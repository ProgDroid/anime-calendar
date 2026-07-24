---
name: Reconcile loop — conditional UPDATE guard against webhook race
description: Conditional UPDATE keyed on `current_period_end <= stripe_period_end` lets a slow safety-net loop coexist with a fast webhook writer. Pattern applies to any "background reconciler races real-time event source" architecture.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
When a slow background loop and a fast event-driven writer both update the same row, the loop must NOT regress newer event-driven data. Use a conditional UPDATE guarded on a monotonic field:

```sql
UPDATE subscriptions SET
    status = $2,
    current_period_end = $3,
    cancel_at_period_end = $4,
    trial_end = $5,
    updated_at = NOW()
WHERE id = $1
  AND current_period_end <= $3   -- guard: only apply if Stripe ≥ local
```

`rows_affected = 0` means a fresher webhook write moved local past the snapshot the loop fetched — the safe no-op outcome. Treat 0 as success, not error.

**Why:** without the guard, a reconcile pass that fetched Stripe at T0 and wrote at T1 could overwrite a webhook that arrived at T0.5 with newer data. The webhook version is by definition fresher (Stripe-pushed, lower latency than the cron). Race window scales with pass duration × subscription count.

**How to apply:**
- Pick a monotonic field that's part of the truth surface (Stripe's `current_period_end` is perfect — it only moves forward on renewal).
- The guard column should be both indexed and naturally part of the WHERE clause — no extra read required.
- Test the race explicitly: seed local with a NEWER period_end than the mock returns, run the pass, assert the row is unchanged.
- Pair with a counter on rows_affected = 1 (drift_corrected) so you can graph reconcile activity over time. A spike means webhook delivery is degraded.
- For the inverse (events that *should* always win), put the same guard on the event handler instead — same pattern, opposite direction.

**Anti-pattern:** "first read, compare, then write" inside the loop. That's a TOCTOU race; only the conditional UPDATE atomically pairs the check with the write.
