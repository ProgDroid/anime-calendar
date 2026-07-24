---
name: AccountPage tab insertion requires updating the spec's router stub
description: Adding a sidebar tab to AccountPage breaks all 11 AccountPage.spec.ts tests if the test router stub doesn't also gain the route — RouterLink :to fails to resolve and tears down render
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
**Rule:** When adding a new tab to `frontend/src/components/AccountPage.vue`, also update `frontend/src/__tests__/AccountPage.spec.ts`:
1. Add the new route to the `makeRouter()` stub.
2. Update the "renders all N sidebar tabs" assertion (test name + new `expect`).
3. Add the new tab to both `it.each` parameter rows ("clicking…pushes to…" and "highlight tracks the active route at…").

**Why:** AccountPage uses `<RouterLink :to="{ name: tab.name }">` for every tab. If the test's stub router has no route with that name, `RouterLink`'s `useLink()` setup throws during component setup — and the throw cascades through all 11 tests in the spec, not just the tab-count assertion. The failure surface looks like "vue-router internal error" rather than "missing route", which obscures the cause.

This came up in Track 4 Phase 4 (adding the Subscription tab). The full vitest run flipped from 287 passed → 11 failed in `AccountPage.spec.ts`, all 11 because the new tab pointed at a `name` the test router didn't know.

**How to apply:**
- Whenever adding/removing a tab in `AccountPage.vue`, the spec change isn't optional or follow-up work — it's part of the same commit.
- The same coupling exists for any component that renders `<RouterLink :to="{ name }">` based on a config list. The cleanest defensive pattern is to register the tab routes in one shared module (`router/account-tabs.ts`) and import it from both production and test code, so they can't drift. We don't do this yet; if a third tab insertion lands, that's the trigger.

**Adjacent gotcha:** an in-progress vitest run will show this failure mode the same way as the [`createWebHistory` cross-spec pollution](feedback_test_history_pollution.md) — vue-router internal stack traces, no clear pointer to the actual issue. When you see `useLink` / `setupStatefulComponent` in the stack, suspect missing routes before suspecting state pollution.
