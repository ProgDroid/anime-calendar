import { test as base, expect } from '@playwright/test'

/**
 * Shared e2e fixtures.
 *
 * The frontend boots an axios call to /user (auth rehydration) before the
 * router decides whether the route is public. With no backend running during
 * Playwright runs, Vite's dev proxy logs ECONNREFUSED for every page load.
 * We intercept all `/api/**` traffic with a default 401 response so the
 * proxy is never reached. Individual tests can override specific routes
 * by registering a more specific `page.route` after this one.
 */
export const test = base.extend({
  page: async ({ page }, use) => {
    await page.route('**/api/**', (route) =>
      route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: '{}',
      }),
    )
    await use(page)
  },
})

export { expect }
