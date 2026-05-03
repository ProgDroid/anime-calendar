import { test, expect } from '@playwright/test'

// Auth surface (login + register) — public, no backend mocking required.

test.describe('login page on mobile', () => {
  test('renders the mobile auth shell at iPhone-14 viewport', async ({ page }) => {
    await page.goto('/login')
    // Both the email + password fields land below the fan/heading.
    await expect(page.locator('input[type="email"]')).toBeVisible()
    await expect(page.locator('input[type="password"]')).toBeVisible()
    // Primary submit CTA is full-width on mobile (the auth shell stretches it).
    const submit = page.locator('button[type="submit"]').first()
    await expect(submit).toBeVisible()
  })

  test('register page renders the mobile auth shell', async ({ page }) => {
    await page.goto('/register')
    await expect(page).toHaveURL(/\/register/)
    await expect(page.locator('input[type="email"]')).toBeVisible()
    await expect(page.locator('input[type="password"]').first()).toBeVisible()
  })
})
