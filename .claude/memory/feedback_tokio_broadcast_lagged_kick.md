---
name: Feedback: tokio broadcast RecvError::Lagged must break on must-deliver channels
description: Lagged on a kick/interrupt broadcast channel means the client missed its eviction signal — treat it as a break, not a continue, or the SSE stream lives forever
type: feedback
originSessionId: 7da6ffde-691a-4b8c-a32d-d10222ee08c5
---
When using a `tokio::broadcast` channel for "must-deliver-once" signals (kick, session eviction), `RecvError::Lagged(n)` means the receiver fell behind and missed one or more messages. If those messages included the kick signal, silently continuing leaves the SSE stream open forever — the client is never disconnected.

**Wrong:**
```rust
msg = kick_rx.recv() => {
    match msg {
        Ok(payload) => { yield Ok(...); break; }
        Err(RecvError::Lagged(_)) => { /* silently continue */ }
        Err(RecvError::Closed) => break,
    }
}
```

**Correct:**
```rust
msg = kick_rx.recv() => {
    match msg {
        Ok(payload) => { yield Ok(...); break; }
        Err(RecvError::Lagged(n)) => {
            log::warn!("kick channel lagged {n} — closing SSE stream defensively");
            break;
        }
        Err(RecvError::Closed) => break,
    }
}
```

**Why:** A lagged kick receiver may have missed the eviction message entirely. Breaking defensively is safe — the worst case is a false disconnect for a well-behaved client who will immediately reconnect. Continuing is unsafe — a kicked user stays connected indefinitely.

**How to apply:** Any `tokio::broadcast` channel used for "interrupt / evict / close" semantics must treat `Lagged` as a break. Channels used for data streaming (e.g. `cal:{id}`, `presence:{id}`) can safely skip lagged events with a warn log.
