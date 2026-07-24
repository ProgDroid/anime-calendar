---
name: Smoke tests must not let onMounted timers escape into teardown
description: Mobile smoke tests that resolve API calls in onMounted can schedule real-timer setTimeout(router.push, ...) callbacks that survive teardown and leak as flakes across the suite. Reject the API in the smoke test, OR use vi.useFakeTimers().
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Vue components frequently fire-and-forget a `setTimeout(() => router.push('/somewhere'), 1500)` inside an `onMounted` success branch. In the success path, the timer is harmless because the user navigates anyway. In a vitest smoke test, however, the timer callback runs *after* the test has finished asserting and unmounted, often after the entire test file has finished, and can fire `router.push` against a now-undefined router instance — surfacing as a cross-spec flake.

**Why:** Track 3 Task 14's `VerifyEmailConfirmPage.spec.ts` resolved `verifyEmail()` in `beforeEach`. The component's success branch scheduled a 1500ms `setTimeout(router.push)` that real-timed past test cleanup. This is the same shape as the pre-existing `SubscriptionTab.spec.ts` flake.

**How to apply:**
For mobile smoke tests where you only want to assert "page renders without throwing":

1. **Reject the API in `beforeEach`** so the component takes the error branch (which usually has no setTimeout):
   ```ts
   verifyEmailMock.mockRejectedValue(new Error('skip-redirect'))
   ```
   The heading/eyebrow text is usually rendered by a wrapping shell regardless of branch, so the assertion still works.

2. **OR use `vi.useFakeTimers()`** so any scheduled setTimeout is held captive and cleared on `vi.useRealTimers()` in `afterEach`.

3. The deeper test in the existing `src/__tests__/VerifyEmailConfirmPage.spec.ts` correctly uses fake timers — pattern was just dropped in the new mobile-smoke variant.

4. When code-reviewing a smoke test, check `onMounted` of the component for `setTimeout`/`Promise.race` and require one of these guards.
