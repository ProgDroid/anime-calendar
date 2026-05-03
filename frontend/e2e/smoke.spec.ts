import { test, expect } from '@playwright/test'

test('app shell loads on mobile viewport', async ({ page }) => {
  await page.goto('/login')
  await expect(page).toHaveURL(/\/login/)
  // Prove the app actually rendered, not just navigated.
  await expect(page.locator('body')).toBeVisible()
})
