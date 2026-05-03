import { test, expect } from './fixtures'

// Viewport flip on a public surface: shell branches but URL state and
// form values persist across the resize. Real-world equivalent of
// rotating an iPad into landscape mid-session.

test('viewport flip mid-session preserves URL and form state', async ({ page }) => {
  await page.goto('/login')
  await expect(page.locator('input[type="email"]')).toBeVisible()
  await page.locator('input[type="email"]').fill('user@example.test')

  // Flip to desktop.
  await page.setViewportSize({ width: 1280, height: 800 })

  // URL stays.
  await expect(page).toHaveURL(/\/login/)
  // Email value survives the layout swap.
  await expect(page.locator('input[type="email"]')).toHaveValue('user@example.test')

  // Flip back to mobile.
  await page.setViewportSize({ width: 390, height: 844 })
  await expect(page).toHaveURL(/\/login/)
  await expect(page.locator('input[type="email"]')).toHaveValue('user@example.test')
})
