import { test, expect } from './fixtures'

const TOKEN = 'abc123'
const PREVIEW = {
  calendar_name: 'Spring 2026',
  owner_display: 'alice',
  owner_avatar: null,
  item_count: 12,
  masked_email: 'e***@t***.com',
}
// Shape must match the real GET /api/user response: auth.ts reads
// `username` and `user_id` (not `id`).
const MOCK_USER = { user_id: 1, username: 'editor', email: 'editor@test.com' }

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

    // H-10: sign-in/sign-up are buttons that stash the token in
    // sessionStorage and navigate to a plain /login or /register URL —
    // the token must never appear in the query string (Referer/history leak).
    const signIn = page.locator('[data-testid="invite-sign-in"]')
    const signUp = page.locator('[data-testid="invite-sign-up"]')
    await expect(signIn).toBeVisible()
    await expect(signUp).toBeVisible()

    await signIn.click()
    await expect(page).toHaveURL('/login')
    expect(new URL(page.url()).search).toBe('')
    expect(
      await page.evaluate(() => sessionStorage.getItem('pendingInviteToken')),
    ).toBe(TOKEN)
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
    // GET preview → success. NB: this glob does NOT match the /accept
    // sub-path, which needs its own stub below.
    await page.route(`**/api/invitations/${TOKEN}`, (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(PREVIEW),
      }),
    )
    // accept() POSTs to /invitations/{token}/accept → 403 email mismatch.
    await page.route(`**/api/invitations/${TOKEN}/accept`, (route) =>
      route.fulfill({
        status: 403,
        contentType: 'application/json',
        body: '{"error":"email mismatch"}',
      }),
    )

    await page.goto(`/invite/${TOKEN}`)
    await expect(page.locator('[data-testid="invite-ready"]')).toBeVisible()

    await page.locator('[data-testid="invite-accept"]').click()
    await expect(page.locator('[data-testid="invite-mismatch"]')).toBeVisible()
  })
})
