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
      await page.route('**/api/**', (route) => route.abort())
      await use()
    },
    { auto: true },
  ],
})

export { expect }
