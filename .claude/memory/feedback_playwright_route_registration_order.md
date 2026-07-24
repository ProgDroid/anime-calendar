---
name: Playwright page.route runs handlers in REVERSE registration order
description: Last-registered matching handler wins; register catch-all FIRST and specific stubs LAST
type: feedback
originSessionId: 68ef861d-4b45-4a3c-8efa-28e7a87de409
---
Playwright's official rule (confirmed against `/microsoft/playwright` docs): "When several routes match the given pattern, they run in the order opposite to their registration. The last registered route can always override all the previous ones."

So in `frontend/e2e/fixtures.ts`, the catch-all `page.route('**/api/**', abort)` MUST be registered **before** any specific stub like `page.route('**/api/public-config', fulfill)`. The specific stub registered later wins for its URL; the abort still applies to everything else.

**Why:** Got bitten twice. First time (2026-05-03 morning, public-config commit), wrote the fixture in the wrong order with a comment claiming "Playwright matches in registration order" — all 24 mobile e2e tests broke because the catch-all aborted `/api/public-config` and the SPA's hard-failing `loadPublicConfig()` rendered the static error shell instead of mounting. The previous version of THIS memory had the rule inverted and reinforced the mistake — it's been corrected.

**How to apply:**
1. When extending `frontend/e2e/fixtures.ts` with a new specific stub: catch-all `route.abort()` goes first, specific `fulfill()` last.
2. After adding, run ONE mobile spec (e.g. `npx playwright test e2e/smoke.spec.ts --project=mobile-chrome`) to confirm the SPA mounts (login form visible). If the catch-all is still winning, the test fails at `expect(input[type=email]).toBeVisible()`.
3. Never trust an inline comment about Playwright matching order — verify against current docs (`/microsoft/playwright` on Context7) before rewriting the fixture.
4. If you change the catch-all from `abort()` to `fulfill(401)`, the axios refresh interceptor trips synchronously and breaks public-route navigation — see the comment block in `fixtures.ts`.
