---
name: Feedback: Vue 3.5 KeepAlive + jsdom navigation crash in tests
description: Tests that trigger router.push inside a KeepAlive-wrapped component cause a Vue 3.5 runtime crash in jsdom — use spy instead of route assertion
type: feedback
originSessionId: 7da6ffde-691a-4b8c-a32d-d10222ee08c5
---
When a component wrapped in `<KeepAlive>` responds to an event by calling `router.push(...)`, actually completing the navigation during a Vitest/jsdom test causes Vue 3.5 to crash. Vue tries to unmount the KeepAlive-cached vnode tree during the route change, finds a null internal instance, and throws.

**Symptom:** Test passes if the component doesn't navigate; crashes when it does — with a Vue internal "Cannot read properties of null" error during route change teardown.

**Fix:** Spy on `router.push` and mock it to a no-op. Assert the spy was called with the expected path instead of asserting `router.currentRoute.value.path`.

```ts
const pushSpy = vi.spyOn(router, 'push').mockImplementation(() => Promise.resolve())
// … trigger the event …
expect(pushSpy).toHaveBeenCalledWith('/my-calendars')
```

**Why:** Vue 3.5 `KeepAlive` caches component vnodes; when navigation unmounts the parent, the cached vnode's internal component instance is null in jsdom, which crashes the Vue unmount cycle. This is a jsdom-only bug (real browsers handle it fine).

**How to apply:** Any test asserting on navigation away from a `<KeepAlive>`-wrapped route must use a `router.push` spy. This applies to the mobile editor (uses `<KeepAlive>` for the tab panels) and any future route that caches children. Desktop editor (no KeepAlive) can assert `router.currentRoute.value.path` directly.
