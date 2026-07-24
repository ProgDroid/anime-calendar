---
name: Playwright SSE stub + GET/POST disambiguation on same endpoint
description: How to stub SSE streams and disambiguate GET vs POST on the same URL in Playwright E2E tests without a real backend.
type: feedback
originSessionId: 0631cb0d-5d1f-44d7-a675-0b839192a599
---
Two related patterns discovered writing the co-editor sharing E2E specs (2026-05-07).

**Why:** The app's E2E infrastructure has no real backend — all `/api/**` is aborted by default. SSE streams and endpoints that serve both GET (preview) and POST (accept) need special stub handling.

**How to apply:**

### SSE stream stub

Use `route.fulfill()` with `Content-Type: text/event-stream`. The browser's `EventSource` client receives the body as a stream and dispatches `onmessage` synchronously:

```ts
await page.route('**/api/calendars/1/events', async (route) => {
  await route.fulfill({
    status: 200,
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
    },
    body: 'data: {"type":"kick","reason":"owner_downgrade"}\n\n',
  })
})
```

The `\n\n` double-newline terminates the SSE frame. The EventSource fires immediately — no polling needed. Assertions can use `page.waitForURL()` or `expect(...).toBeVisible()` directly after.

### GET vs POST on the same URL

When an endpoint handles both GET (e.g. preview) and POST (e.g. accept), use a single `page.route()` handler that branches on `route.request().method()`:

```ts
await page.route(`**/api/invitations/${TOKEN}`, (route) => {
  if (route.request().method() === 'POST') {
    return route.fulfill({ status: 403, body: '{"error":"email mismatch"}' })
  }
  return route.fulfill({ status: 200, body: JSON.stringify(PREVIEW) })
})
```

This avoids double-registration conflicts (registering two handlers for the same pattern). Playwright runs the most-recently-registered handler — a second registration would silently shadow the first for ALL methods.
