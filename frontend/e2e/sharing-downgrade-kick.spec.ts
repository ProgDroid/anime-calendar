import { test, expect } from './fixtures'

const CALENDAR_ID = 1

// Shape must match the real GET /api/user response: auth.ts reads
// `username` and `user_id` (not `id`).
const MOCK_USER = { user_id: 99, username: 'editor', email: 'editor@test.com' }

const MOCK_SETTINGS = {
  theme_preference: 'dark',
  language_preference: 'en',
  title_language_preference: 'English',
  accent_preference: 'coral',
  timezone: 'UTC',
  reminder_offsets_minutes: [30],
  user_id: 99,
}

const MOCK_CALENDAR = {
  id: CALENDAR_ID,
  name: 'Spring 2026',
  language: 'english',
  event_style: 'timed',
  items: [],
  user_id: 2, // owned by alice (id=2), not this editor (id=99)
  meta_version: 0,
  created_at: '2026-01-01T00:00:00',
  updated_at: '2026-01-01T00:00:00',
}

const MOCK_CALENDARS = {
  owned: [],
  shared_with_me: [
    { id: CALENDAR_ID, name: 'Spring 2026', owner: { id: 2, display: 'alice', avatar: null } },
  ],
}

// SSE body that immediately emits an owner_downgrade kick frame.
const KICK_SSE_BODY = 'data: {"type":"kick","reason":"owner_downgrade"}\n\n'

test('editor is kicked and redirected to /my-calendars when owner downgrades', async ({
  page,
}) => {
  // Stubs are registered in REVERSE win order — register catch-all (fixtures.ts)
  // first (done by auto-fixture), then specific stubs here (they win over abort).

  // Auth: stub /user so the router guard sees an authenticated session.
  await page.route('**/api/user', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_USER),
    }),
  )

  // Settings: required by router guard + CalendarEditorViewDesktop onBeforeMount.
  await page.route('**/api/user/settings', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_SETTINGS),
    }),
  )

  // Subscription: editor is on free tier (tier check in onBeforeMount).
  await page.route('**/api/subscription/me', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        tier: 'free',
        status: null,
        current_period_end: null,
        cancel_at_period_end: false,
        trial_end: null,
      }),
    }),
  )

  // Calendar data: the page loads the calendar on mount.
  await page.route(`**/api/calendars/${CALENDAR_ID}`, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_CALENDAR),
    }),
  )

  // My-calendars list: needed when the kicked editor lands on /my-calendars.
  await page.route('**/api/calendars', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(MOCK_CALENDARS),
    }),
  )

  // Heartbeat: stubbed 204 so axios doesn't throw (timer fires at 30 s, but
  // stub is present just in case test runs slowly under load).
  await page.route(`**/api/calendars/${CALENDAR_ID}/presence/heartbeat`, (route) =>
    route.fulfill({ status: 204 }),
  )

  // SSE stream: immediately emits a kick frame with reason=owner_downgrade.
  // Registered LAST so it wins over the catch-all abort in fixtures.ts.
  await page.route(`**/api/calendars/${CALENDAR_ID}/events`, (route) =>
    route.fulfill({
      status: 200,
      headers: {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      },
      body: KICK_SSE_BODY,
    }),
  )

  // Navigate to the calendar editor as the logged-in editor.
  await page.goto(`/calendar/${CALENDAR_ID}`)

  // Do NOT assert an intermediate editor marker here: the stubbed SSE body
  // delivers the kick frame instantly on mount, so the component can redirect
  // before the editor finishes rendering — asserting the editor UI races the
  // very redirect under test. The redirect IS the contract.
  await page.waitForURL('**/my-calendars', { timeout: 10_000 })

  // Verify we landed on the My Calendars page (shared-with-me section visible).
  await expect(page).toHaveURL(/\/my-calendars/)
})
