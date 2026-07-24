---
name: RAII guards for stream-lifetime resources must be moved INTO async_stream::stream! { ... }
description: A guard variable acquired in the handler body and then captured into the stream block must be re-bound inside the block (let _g = guard;) or it gets dropped before the stream starts
type: feedback
originSessionId: d141fb2f-b707-4ff0-8ba3-228ad77d407f
---
When acquiring an RAII guard (counter decrementer, file lock, connection slot, etc.) for the lifetime of an SSE stream or any long-lived `async_stream::stream!`, move the guard variable INSIDE the `stream! { ... }` block as the first statement:

```rust
let connection_guard = match tracker.try_acquire(actor_id) {
    Some(g) => g,
    None => return Err(Error::TooManyRequests),
};
// ... initial frame setup ...
let stream = async_stream::stream! {
    // CRITICAL: re-bind so the guard's lifetime extends through every yield.
    let _connection_guard = connection_guard;
    yield ...;
    loop { ... }
};
```

**Why:** `async_stream::stream!` returns an opaque `Stream` whose first poll runs from the top of the block. If the guard is captured by reference or only kept in the outer scope, it gets dropped when the handler returns the stream — *before* the first poll. The counter decrements immediately while the connection is still being established, defeating the cap.

The compiler does not warn about this because the guard IS captured by the closure (it has a `Drop` impl, so move-capture works) — but the closure's value is the stream itself, dropped after the call to `Sse::from_stream(stream)` returns the response. The re-bind inside the block hangs the guard's drop on the stream's drop.

**How to apply:**
- Any `async_stream::stream!` block that depends on resource quotas, semaphore permits, or per-connection counters needs an explicit `let _name = guard;` line at the top.
- Tests should verify: (a) cap reached → 429, AND (b) connection closed → counter decremented (so a re-acquire succeeds). Phase 3 H-6 covered (a) via cap-rejected test and (b) via re_acquire_after_drop_succeeds.
- This pattern came up live in Phase 3 T5 (`SseConnectionTracker` for `controllers/sse.rs::calendar_events`).
