---
name: Track 2 a11y authenticated sweep complete
description: 2026-05-01 closure of the live axe sweep across authenticated routes; 3 real findings fixed in commit 1be9d5b. Track 2 fully done.
type: project
originSessionId: 89a0962d-4c26-463e-85ed-12e689053aa0
---
Live axe-core sweep across all 7 authenticated routes (5 accents × 2 themes each) on 2026-05-01 with backend running locally. Closes the deferred sweep plan filed earlier same day after Track 2 audit/remediation merged.

**Why:** Track 2 a11y remediation (commit 504516a) was verified live only on `/login`. The authenticated routes were left for the next session that had the backend running.

**How to apply:** Track 2 is now fully closed. Next sequenced track per the master breakdown is **Track 4 — Upgrade flow** (Pro tier, Stripe, paywall screens, entitlement gating). Track 3 (mobile companion) runs after Track 4. The deferred sweep plan now has Done status.

**Findings shipped in commit 1be9d5b:**
- F1: missing `<h1>` on every authenticated route. Fixed by promoting account-tab `<h2>` to `<h1>` in all 4 tabs, adding sr-only `<h1>` in `CalendarPage.vue` tied to active tab, bumping `MediaItemCard` `<h4>` → `<h3>` to keep heading-order valid.
- F2: nested `<main>` on `/account/*`. Fixed by switching `AccountPage.vue` inner `<main>` to `<section :aria-label="t('account.tabs.label')">`.
- F3: `UiMenu` did not restore focus to invoker on close. Fixed via `watch(open)` mirroring the `UiModal` pattern; affects all menu consumers (topbar avatar, CalendarTile export/kebab).

Re-run after fixes: 0 axe violations on representative routes; 267 tests / 49 files passing; build clean.
