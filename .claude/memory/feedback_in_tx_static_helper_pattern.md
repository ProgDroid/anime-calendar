---
name: Mapper _in_tx static-helper pattern for cross-mapper transactions
description: For multi-mapper writes that must commit atomically, expose static `_in_tx(conn, ...)` helpers on each mapper and have the caller open one outer transaction
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** When a controller needs to write across multiple mappers atomically, add `_in_tx(conn: &mut PgConnection, ...)` static helpers on each mapper and have the controller open the outer transaction. Don't try to share `&self` mappers across nested transactions.

**Why:** Mappers in this codebase own a `Database` (which owns a `PgPool`). The natural shape `mapper.do_thing(...)` does `pool.acquire()` internally — fine for single-mapper work, but useless when two mappers must share a transaction. The `_in_tx` shape inverts ownership: the connection lives on the caller's stack, both mappers borrow it, the caller commits or rolls back. Existing examples: `find_active_for_user_with(conn, ...)`, `find_by_stripe_id_with(conn, ...)` (these use `_with` suffix; newer Phase-3 helpers use `_in_tx` for clarity).

This is what made Track 4 Phase 3's idempotency atomic: the Stripe webhook handler calls `pool.begin()` once, then `StripeEventMapper::record_first_time_in_tx` and `SubscriptionMapper::upsert_from_stripe_in_tx` both borrow that same `&mut PgConnection`. A handler-side panic rolls both back together so a Stripe retry replays cleanly.

**How to apply:**
- Static methods (no `&self`) — the mapper instance isn't needed; the connection is.
- Take `&mut sqlx::PgConnection` (not `&mut Transaction<'_, _>`) so the helper composes with both `Transaction::begin()`/`begin_nested()` and bare connections from `pool.acquire()`.
- Public visibility (or `pub(crate)`) — the caller is typically a controller or another mapper, not the same mapper.
- Match the field signature of the non-tx version exactly so callers can swap with one search-replace.
- Pair with the non-tx `&self` version if there are still single-mapper callers — they delegate to the static helper:
  ```rust
  pub async fn do_thing(&self, x: i32) -> ServerResult<()> {
      Self::do_thing_in_tx(&mut *self.db.pool.acquire().await?, x).await
  }
  ```
- Document on the `_in_tx` helper *why* it exists ("participates in an existing transaction so X commits atomically with Y").

**When NOT to add it:** If the mapper's writes are already self-contained (one query, no cross-mapper dependency), a `_with` shim on top of the `&self` version is just clutter. Wait until a real cross-mapper caller appears.
