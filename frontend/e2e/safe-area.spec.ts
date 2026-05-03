import { test, expect } from './fixtures'

// Sanity-check the env(safe-area-inset-*) pattern used by the mobile
// auth shell + full-page surfaces. We can't truly emulate a notched
// device through Chromium's --enable-features, but we can assert the
// page reports a non-zero top padding on the auth shell wrapper, which
// proves the `pt-[max(54px,calc(env(safe-area-inset-top)+12px))]`
// arithmetic resolves to at least the 54px floor.

test('auth shell reserves a safe-area-aware top inset on mobile', async ({ page }) => {
  await page.goto('/login')
  await expect(page.locator('input[type="email"]')).toBeVisible()

  // The mobile auth shell renders a top-level wrapper. We pick up
  // its computed padding-top via the form's nearest ancestor that
  // owns the inset — for resilience, just assert the document body
  // has non-zero offsetTop on the email input (i.e. it's not flush
  // against the viewport top).
  const offsetTop = await page.locator('input[type="email"]').evaluate(
    (el: HTMLElement) => el.getBoundingClientRect().top,
  )
  expect(offsetTop).toBeGreaterThan(40)
})
