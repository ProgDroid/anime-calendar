---
name: Playwright body locator can fail "hidden" during Vue mount
description: `expect(page.locator('body')).toBeVisible()` is brittle in SPAs because the body has zero area for one tick while Vue mounts the router-view. Use a stable post-mount marker (input, h1, data-testid) instead.
type: feedback
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Playwright's `toBeVisible()` checks include "non-zero size" — not just `display !== 'none'`. In a Vue/SPA, between page load and `<RouterView>` mounting its first component, `<body>` contains only `<div id="app"></div>` (empty) and registers as zero-area. `expect(body).toBeVisible()` fails with "received: hidden" even though there's nothing actually hidden.

**Why:** Track 3's existing `e2e/smoke.spec.ts` had `expect(page.locator('body')).toBeVisible()` from Task 5 scaffolding. It failed reliably on the first dev-server cold start in mobile-chrome. Replacing with `expect(page.locator('input[type="email"]')).toBeVisible()` made it stable across all 3 device projects.

**How to apply:**
1. Never assert visibility on `body` or `html` in a Playwright test against an SPA — they're meaningless markers.
2. Use a post-mount element: a form input, a heading, a `data-testid` on a known-rendered component. The first thing the router-view paints is the safest target.
3. For the dev-server cold-start case specifically: the playwright.config's `webServer.timeout: 120000` is generous, but the FIRST test still races vite's HMR boot. Sometimes the very first test of a fresh suite needs a re-run; if it consistently fails, the locator is the cause, not the timing.
