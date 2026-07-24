---
name: Feedback: SSE events echo back to the actor — always filter by actor
description: All SSE calendar events are broadcast to every subscriber including the originator; UI handlers must guard frame.actor !== authStore.user or the user sees their own actions as remote changes
type: feedback
originSessionId: 7da6ffde-691a-4b8c-a32d-d10222ee08c5
---
The SSE stream at `GET /calendars/{id}/events` broadcasts every published event to **all** active subscribers on the channel, including the user who triggered the mutation. If a Vue component watches `lastEvent` and shows UI for remote changes (collision banner, toast notifications), it must filter out self-originated events or the user will see their own saves reflected back as if a collaborator made them.

**Pattern:**
```ts
watch(lastEvent, (frame) => {
  if (!frame) return
  switch (frame.type) {
    case 'item_added':
    case 'item_removed':
    case 'meta_updated':
      if (frame.actor !== authStore.user) {  // ← always guard
        // show UI feedback
      }
      break
    case 'kick':
      // kick has no actor field — applies to the recipient unconditionally
      break
  }
})
```

**Why:** Redis Pub/Sub delivers to all subscribers. The server does not exclude the publisher. Missing the actor guard means: user saves → SSE bounce-back → their own collision banner fires → confusing UX.

**How to apply:** Every `watch(lastEvent, ...)` branch that shows UI feedback to the "other editor" (toasts, banners, list changes) must check `frame.actor !== authStore.user`. Exceptions: `kick` (no actor field, applies unconditionally), `presence` (viewer list is always authoritative), `meta_snapshot` (initial state, no actor).
