import { test, expect } from './fixtures'

const TOKEN = 'abc123'
const PREVIEW = {
  calendar_name: 'Spring 2026',
  owner_display: 'alice',
  owner_avatar: null,
  item_count: 12,
  masked_email: 'e***@t***.com',
}
const MOCK_USER = { id: 1, username: 'editor', email: 'editor@test.com' }

test.describe('Invite landing page', () => {
  test('shows invalid state when token is not found', async ({ page }) => {
    // Fixture catch-all abort is registered first; this specific stub wins.
    await page.route(`**/api/invitations/${TOKEN}`, (route) =>
      route.fulfill({ status: 400, contentType: 'application/json', body: '{"error":"not found"}' }),
    )

    await page.goto(`/invite/${TOKEN}`)
    await expect(page.locator('[data-testid="invite-invalid"]')).toBeVisible()
  })

  test('shows unauth state when user is not logged in', async ({ page }) => {
    // /api/user stays aborted (fixture default) → user is unauthenticated.
    await page.route(`**/api/invitations/${TOKEN}`, (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(PREVIEW),
      }),
    )

    await page.goto(`/invite/${TOKEN}`)
    await expect(page.locator('[data-testid="invite-unauth"]')).toBeVisible()

    // Sign-in and sign-up links must carry the redirect query param.
    const signIn = page.locator('[data-testid="invite-sign-in"]')
    const signUp = page.locator('[data-testid="invite-sign-up"]')
    await expect(signIn).toBeVisible()
    await expect(signUp).toBeVisible()
    await expect(signIn).toHaveAttribute('href', `/login?redirect=/invite/${TOKEN}`)
    await expect(signUp).toHaveAttribute('href', `/register?redirect=/invite/${TOKEN}`)
  })

  test('shows ready state when user is logged in', async ({ page }) => {
    // Stub the user endpoint before /api/** catch-all so auth rehydration succeeds.
    await page.route('**/api/user', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(MOCK_USER),
      }),
    )
    await page.route(`**/api/invitations/${TOKEN}`, (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(PREVIEW),
      }),
    )

    await page.goto(`/invite/${TOKEN}`)
    await expect(page.locator('[data-testid="invite-ready"]')).toBeVisible()
    await expect(page.locator('[data-testid="invite-accept"]')).toBeVisible()
    await expect(page.locator('[data-testid="invite-decline"]')).toBeVisible()
  })

  test('shows mismatch state when accept returns 403', async ({ page }) => {
    await page.route('**/api/user', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(MOCK_USER),
      }),
    )
    await page.route(`**/api/invitations/${TOKEN}`, (route) => {
      if (route.request().method() === 'POST') {
        // accept() POST → 403
        return route.fulfill({
          status: 403,
          contentType: 'application/json',
          body: '{"error":"email mismatch"}',
        })
      }
      // GET preview → success
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(PREVIEW),
      })
    })

    await page.goto(`/invite/${TOKEN}`)
    await expect(page.locator('[data-testid="invite-ready"]')).toBeVisible()

    await page.locator('[data-testid="invite-accept"]').click()
    await expect(page.locator('[data-testid="invite-mismatch"]')).toBeVisible()
  })
})
