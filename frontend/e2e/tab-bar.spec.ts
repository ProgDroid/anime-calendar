import { test, expect } from '@playwright/test'

// The bottom tab bar is gated by the `meta.bottomTabBar: true` route flag.
// Public routes (login/register) must NOT show it. Authenticated routes do —
// since this suite has no backend, we can only reliably assert the negative.

test('bottom tab bar is hidden on /login (public route)', async ({ page }) => {
  await page.goto('/login')
  await expect(page.locator('input[type="email"]')).toBeVisible()
  await expect(page.getByTestId('bottom-tab-bar')).toHaveCount(0)
})

test('bottom tab bar is hidden on /register (public route)', async ({ page }) => {
  await page.goto('/register')
  await expect(page.locator('input[type="email"]')).toBeVisible()
  await expect(page.getByTestId('bottom-tab-bar')).toHaveCount(0)
})

test('bottom tab bar is hidden on /forgot-password (public route)', async ({ page }) => {
  await page.goto('/forgot-password')
  await expect(page.locator('input[type="email"]')).toBeVisible()
  await expect(page.getByTestId('bottom-tab-bar')).toHaveCount(0)
})
