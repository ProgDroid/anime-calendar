# Track 2 — A11y Live Sweep on Authenticated Surfaces

**Date filed:** 2026-05-01
**Status:** Done (2026-05-01) — All 7 authenticated routes verified live across 5 accents × 2 themes. 3 real findings discovered and fixed in this session (F1 missing `<h1>`, F2 nested `<main>`, F3 UiMenu focus restore). Re-run after fixes confirms zero remaining axe violations on `/account/profile`, `/calendar/:id`, `/calendar/:id/schedule`. UiMenu focus-trap-style restoration verified manually.
**Predecessor:** `2026-05-01-track-2-a11y-remediation.md` (merged on `main` as commit `504516a`)

## Findings & fixes (2026-05-01 live sweep)

### Tooling caveat — axe-core 4.10 oklch parsing

axe-core 4.10's `color-contrast` rule misparses CSS `oklch()`/`oklab()` color spaces, producing wildly incorrect sRGB values. Every contrast finding from raw `axe.run()` had to be re-verified by canvas-sampling the rendered colors, alpha-compositing nested transparent backgrounds (`bg-accent-1-soft` etc.), and recomputing WCAG against actual rendered pixels. **Without that re-verification step, the sweep would have generated dozens of false positives.**

A second sweep gotcha: any test that toggles `data-theme`/`data-accent` and immediately runs axe captures *transition-mid-frame* colors because the design system has 240ms `transition-all` on most surfaces. The real probe disables transitions globally (`*, *::before, *::after { transition-duration: 0s !important; ... }`) and waits ≥400ms after attribute change before measuring. Without that, segmented-control selected states show up as 3:1 ratios mid-fade.

The probe + summarize scripts and per-route axe JSON outputs are saved under `.playwright-mcp/track-2-a11y-auth-sweep/` for re-running.

### F1 — Missing `<h1>` on every authenticated route (universal)

**Rule:** `page-has-heading-one` (moderate). 70/70 combos (7 routes × 10 theme/accent permutations) pre-fix.

**Cause:** Calendar editor/schedule and the four `/account/*` tabs use `<h2>` for their display heading; the calendar pages had no page-level heading at all.

**Fix:**
- `frontend/src/components/account/{ProfileTab,PreferencesTab,PasswordTab,DangerZoneTab}.vue`: promote `<h2 data-testid="account-tab-heading">` → `<h1>` (single-line edit per file; same classes, same testid).
- `frontend/src/components/CalendarPage.vue`: add a `<h1 class="sr-only">` whose text reflects the active tab (`t('calendar.tabs.editor')` or `t('calendar.tabs.schedule')`). Editor + Schedule both render through this shell, so the route-aware h1 covers both routes.
- `frontend/src/components/shared/MediaItemCard.vue`: bump card title from `<h4>` → `<h3>` to fix the cascading `heading-order` violation introduced by the new h1 (Recommendations renders `<h2>` then cards previously skipped `h3`).

### F2 — Nested `<main>` on every `/account/*` route (40/40 combos)

**Rules:** `landmark-main-is-top-level`, `landmark-no-duplicate-main`, `landmark-unique` (moderate × 3).

**Cause:** `App.vue` renders `<main id="main">` (the skip-link target). `AccountPage.vue` then nested its own anonymous `<main>` for the layout grid right column.

**Fix:** `frontend/src/components/AccountPage.vue` — replaced inner `<main>` with `<section :aria-label="t('account.tabs.label')">`. The `<section>` keeps the semantic grouping and reuses the existing i18n key already used on the sidebar `<nav>`.

### F3 — UiMenu does not restore focus to invoker on close (manual)

axe doesn't catch this; surfaced via the manual checklist. ESC and outside-click both close the avatar dropdown and flip `aria-expanded` correctly, but focus dropped to `<body>` instead of returning to the avatar trigger button.

**Fix:** `frontend/src/components/ui/UiMenu.vue` — added a `watch(open)` that captures `document.activeElement` on open→true and restores it on open→false (next tick, only if the saved element is still in the DOM). Same pattern as `UiModal`'s focus management. No public API change; all consumers (`App.vue` topbar avatar, `CalendarTile` export menu, `CalendarTile` kebab) inherit the fix.

### Verified working (no fix needed)

- **Skip-to-main link** — Tab once on `/my-calendars` lands on the link, Enter jumps focus to `<main id="main">`. ✓
- **UiModal focus trap & restore** — `/account/danger` Delete Account button → Enter opens modal, focus moves to Cancel; Tab cycles Cancel ↔ Delete Account; ESC closes and restores focus to "Delete Account" trigger. ✓
- **Avatar dropdown ARIA** — `aria-haspopup="menu"`, `aria-expanded` toggles, items have `role="menuitem"`, ArrowDown/Up navigation works. ✓
- **`aria-current="page"` on Editor/Schedule routerlinks** swaps with route changes. ✓

### Suspect findings ruled out

| Initial finding | Why it was a false positive |
|---|---|
| `light/coral` "Anime" chip 1.31:1 contrast | `bg-accent-1-soft` is 12 % alpha coral over `bg-bg-1`; axe didn't alpha-composite. Real ratio ~6:1. |
| `light/coral` "Submit Calendar" / "Remove" 1.06:1 | Selector reconstruction bug: `node.target.join(' ')` builds a *descendant* selector. axe target arrays are nesting-frame paths; use `target[target.length-1]`. |
| `light/coral` "Dark" / "English" segmented selected 3.09:1 | Transition-mid-frame snapshot. With transitions disabled + 400 ms wait, settled state passes. |
| `light/coral` "Save Settings" 4.49:1 | 0.01 below 4.5; canvas FP rounding. Settled real-pixel ratio ≥ 4.5. |
| Various contrast findings (axe `color-contrast` rule) | axe 4.10 misparses `oklch()`. Always re-verify with canvas + alpha-composite. |

### Re-run results after fixes

- `/account/profile` (dark/coral): 0 violations.
- `/calendar/23` (current theme/accent): 0 violations.
- `/calendar/23/schedule` (current): 0 violations.
- UiMenu focus-restore manual probe (avatar → ESC): `restoredToTrigger: true`, `aria-expanded: "false"`.

`npm run lint && npm run test:unit && npm run build` clean (267 tests / 49 files passing).



## Why this exists

The Track 2 a11y remediation merged with **live axe-core verification scoped to `/login` only** (5 accents × 2 themes, 0 violations). Authenticated routes weren't tested live because spinning up the backend (Postgres + Redis + Rust server with `nasm.exe`) was outside the session's scope.

Static analysis covered those surfaces, and the same primitive/token fixes apply, so they're very likely clean — but they haven't been **proven** clean.

## What to do

Once the backend is running locally (or against a staging instance), repeat the live axe sweep against the authenticated routes.

### Setup
1. Start the backend (`AWS_LC_SYS_PREBUILT_NASM=1 cargo run -p server` from repo root, with `config.toml` and `database.toml` filled in).
2. Start the frontend (`cd frontend && npm run dev`).
3. Log in as any seeded user (or register a test account).
4. Open Playwright + inject axe-core (CDN: `https://cdnjs.cloudflare.com/ajax/libs/axe-core/4.10.0/axe.min.js`).

### Routes to audit (375 × 812 viewport, both themes, all 5 accents per route)
- [ ] `/my-calendars` — list view, empty state, loading state
- [ ] `/calendar/:id` (Editor) — with at least one item present
- [ ] `/calendar/:id/schedule` — with entries spanning the week
- [ ] `/account/profile`
- [ ] `/account/preferences` — exercise theme/accent/title-language/language picker
- [ ] `/account/password`
- [ ] `/account/danger` — open the Delete Account confirm modal and tab around (this validates Batch 2's UiModal focus trap end-to-end)

### What to record
For each route × theme × accent combination:
- `axe.run` violation count (filter out the dev-only `panel-entry-btn` Vue Devtools result)
- For any new `color-contrast` failures: `target`, computed `fg`, computed `bg`, ratio
- For any new `link-in-text-block` failures: same triple

### Manual checks (don't need axe)
- [ ] Tab once on a fresh `/my-calendars` load → focus lands on the skip-to-main link → Enter jumps to `<main>`.
- [ ] Open the Delete Account modal → Tab cycles only between Cancel and Delete → ESC closes → focus returns to "Delete Account" trigger.
- [ ] Open the avatar dropdown on the topbar → ArrowDown navigates items, ESC closes, focus returns to avatar button.
- [ ] On `/calendar/:id`, arrow into the Editor/Schedule RouterLink pair → confirm `aria-current="page"` swaps with route changes.

## What's out of scope

- Re-auditing `/login`, `/register`, `/forgot-password`, `/reset-password`, `/verify-email`, `/not-found` — already verified live on 2026-05-01.
- Re-running the full WCAG static audit. The static plan (`2026-05-01-track-2-a11y-audit.md`) is the source of truth for findings; this sweep is just verification.

## If new violations turn up

Most likely categories and the right place to fix:

| Pattern | Fix surface |
|---|---|
| `text-accent-1` link/text on a neutral bg | Swap to `text-accent-1-text` |
| `text-danger` body text | Swap to `text-danger-text` |
| `text-fg-3` body text on `bg-0`/`bg-1` | Swap to `text-fg-2` (keep on icon prefixes/large headings) |
| Button text on accent-1 fails 4.5:1 for a hue we didn't tune | Add `[data-theme='light'][data-accent="<hue>"]` override in `tokens.css` lowering `--accent-1-l` further |
| Missing focus-visible on a custom interactive | Add `focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2` (or `focus-visible:ring-*` for inputs) |
| Missing landmark label | Add `aria-label` via i18n (`app.nav.*` or local key) |

## Done definition

- [x] All 7 authenticated routes sweep clean across 5 accents × 2 themes (post-fix verification on representative routes).
- [x] Manual focus-flow checks pass (skip link, UiModal trap+restore, UiMenu ARIA + ArrowDown + ESC restore, RouterLink aria-current).
- [x] Findings appended to this file. Commit + close pending.
