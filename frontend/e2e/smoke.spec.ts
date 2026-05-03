import { test, expect } from '@playwright/test'

test('app shell loads on mobile viewport', async ({ page }) => {
  await page.goto('/login')
  await expect(page).toHaveURL(/\/login/)
  // The login form's email field is the most stable post-mount marker.
  await expect(page.locator('input[type="email"]')).toBeVisible()
})
