import { test as base, expect } from '@playwright/test'

/**
 * Shared e2e fixtures.
 *
 * The frontend boots an axios call to /user (auth rehydration) before the
 * router decides whether the route is public. With no backend running during
 * Playwright runs, Vite's dev proxy logs ECONNREFUSED for every page load.
 *
 * We `route.abort()` all `/api/**` requests so the proxy is never reached.
 * `abort` (not `fulfill(401)`) preserves the behavior the app already
 * tolerates — a network failure on /user → user stays unauth. Returning a
 * fast 401 instead trips the axios refresh interceptor synchronously and
 * disrupts the public-route navigation the e2e tests rely on.
 *
 * Implemented as an auto-fixture that depends on `page` (rather than
 * overriding `page`) so it composes cleanly with project-level
 * `contextOptions` (viewport, userAgent, isMobile, etc.).
 */
export const test = base.extend<{ apiMock: void }>({
  apiMock: [
    async ({ page }, use) => {
      // Playwright runs matching route handlers in REVERSE registration
      // order — the most recently registered handler wins. Register the
      // catch-all abort first, then the specific public-config stub, so
      // the stub runs first for `/api/public-config` and the abort still
      // covers every other `/api/**` request.
      //
      // The SPA hard-fails its bootstrap if `/api/public-config` is
      // rejected, so an explicit success stub is required for every spec.
      await page.route('**/api/**', (route) => route.abort())
      await page.route('**/api/public-config', (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            google_client_id: 'e2e-test-cid',
            limits: { free_calendar_limit: 3, free_show_cap: 25, pro_max_reminders: 5 },
            presence_heartbeat_seconds: 30,
          }),
        }),
      )
      await use()
    },
    { auto: true },
  ],
})

export { expect }
