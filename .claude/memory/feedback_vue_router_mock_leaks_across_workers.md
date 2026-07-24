---
name: vi.mock('vue-router', ...) leaks across vitest workers
description: Module-level vue-router mocks in one spec file break sibling specs that use createRouter; use createMemoryHistory + router.push override instead
type: feedback
originSessionId: be1596ff-ca01-4f16-80bf-1a49c8d8e222
---
`vi.mock('vue-router', () => ({ useRouter: () => ({ push: spy }) }))` in one spec file leaks into sibling spec files that share the same vitest worker. The sibling specs that import `createRouter` from real vue-router crash with `TypeError: Cannot read properties of undefined (reading 'push')` because the cached module no longer exposes `createRouter`/`createMemoryHistory`.

Even `vi.mock('vue-router', async (importOriginal) => ({ ...await importOriginal(), useRouter: ... }))` did **not** fix it in this codebase — the leaked `useRouter` override propagates to the sibling spec's component instances.

**The fix:** never `vi.mock('vue-router', ...)`. Instead:

```ts
import { createRouter, createMemoryHistory } from 'vue-router'

const routerPushSpy = vi.fn()

function mountTab() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div />' } }],
  })
  router.push = routerPushSpy as unknown as typeof router.push
  return mount(Component, { global: { plugins: [router] } })
}
```

**Why:** A real router with memory history pollutes nothing across workers (memory is per-instance), and overriding `router.push` post-construction gives you the spy without touching the module cache.

**How to apply:** Anytime you need to assert `useRouter().push(...)` was called, use the memory-history pattern above. Do not `vi.mock('vue-router', ...)`. This is a hard rule — there is no situation in this codebase where the mock approach is safer than memory history.

Sibling memory: `feedback_test_history_pollution` already established `createMemoryHistory` as the rule, but only for `createWebHistory` pollution; this generalises to **any** vue-router mocking strategy.

First hit: 2026-05-05 during PreferencesTab.spec.ts authoring; broke MyCalendarsPage.spec.ts (4 tests / `editCalendar`, mobile create CTA, etc.) until the mock was replaced with memory history.
