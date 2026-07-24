---
name: Feedback: Confirm-dialog async handler ordering in Vue
description: Async handlers wired to confirm dialogs must close modal first, guard after, and clear id in finally — not the reverse.
type: feedback
originSessionId: 0378eb37-d27b-4000-9314-fca2c4039899
---
For async functions wired to a `@confirm` event on a `ConfirmModal`, always follow this order:

```ts
async function executeX() {
  modalOpen.value = false          // 1. close modal FIRST
  if (targetId.value === null) return  // 2. guard after close
  const id = targetId.value
  // ... optimistic update ...
  try {
    await api.delete(`/resource/${id}`)
  } catch {
    // rollback
  } finally {
    targetId.value = null           // 3. clear id in finally
  }
}
```

**Why:** Closing before the guard ensures the modal always dismisses even if the id was somehow null (avoids a stuck-open modal). Clearing in `finally` guarantees cleanup even if the catch branch itself throws (e.g. `loadCalendars` fails during rollback). The wrong pattern — guard before close, clear before await — was caught by code review in Task 2.4 (`executeLeave` vs `executeDelete`).

**How to apply:** Any new confirm-then-delete/leave handler in `MyCalendarsPage.vue` or similar should follow this exact shape. Reference `executeDelete` as the canonical example.
