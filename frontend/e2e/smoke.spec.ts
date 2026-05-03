import { test, expect } from '@playwright/test'

test('app shell loads on mobile viewport', async ({ page }) => {
  await page.goto('/login')
  await expect(page).toHaveURL(/\/login/)
  // All three projects use a mobile viewport (< 1024px wide).
  const viewport = page.viewportSize()
  expect(viewport?.width).toBeLessThan(1024)
})
