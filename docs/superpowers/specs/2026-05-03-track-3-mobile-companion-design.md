# Track 3 — Mobile Companion (Design Spec)

**Date:** 2026-05-03
**Author:** brainstormed with Claude
**Status:** approved, ready for implementation plan
**Source design:** `design_handoff_anime_calendar/screens-mobile.jsx`
**Master breakdown:** [`2026-04-30-design-redesign-master-breakdown.md`](./2026-04-30-design-redesign-master-breakdown.md)
**Predecessors:** Track 1 (Foundations, complete), Track 2 (Existing surfaces, complete), Track 4 (Upgrade flow, complete)
**Sequencing:** runs last per master breakdown.

## Goal

Make every Track 2 + Track 4 surface work well on phone viewports, plus introduce the mobile-specific patterns the design handoff specifies (4-tab → simplified 2-tab bottom bar, FAB, bottom-sheet modals, batch-add CTA, banner fade-in preserved). Pure frontend track — no server changes.

## Decisions (locked in brainstorm)

1. **Scope:** full sweep over all 11 specified surfaces (Login · Register · MyCalendars · Editor · Schedule · Account · Upgrade Interrupt / Paywall / Success / Cancelled · 404 + verify states).
2. **Route topology:** hybrid responsive. Single route tree; components branch on viewport. **Editor** is the one exception — `CalendarEditorView.vue` becomes a thin shell that picks `CalendarEditorViewDesktop` (existing layout, renamed) or `CalendarEditorViewMobile` (new). All other surfaces keep one component file with internal Tailwind responsive prefixes.
3. **Editor variant:** **A — Tabbed** (segmented Items / Search + FAB on Items tab, jumps to Search). Variants B (bottom-sheet) and C (stacked) dropped — not deferred, removed from scope entirely.
4. **Bottom tab bar shape:** **2 tabs only — Library / Account.** No "Pro" / "Editor" tabs. Editor isn't a peer destination — it's a child of Library reached via a calendar tile tap. While inside `/calendar/:id` the Library tab stays highlighted (iOS Mail / Inbox pattern).
5. **Upgrade nudge (Nudge-B):** symmetric across viewports. Lives in `SubscriptionTab.vue` (which is shared desktop + mobile). Free users see a tier chip + "Upgrade to Pro" CTA on the Subscription row. No mobile-only upgrade tab; no Library banner; no empty-state pitch.
6. **Tablet:** deferred. Single breakpoint at **1024 px (`lg`)** — below = mobile, above = desktop. Exposed as a named constant (`MOBILE_BREAKPOINT_PX`) so retuning is one line. iPads in portrait get the desktop layout (acceptable for v1).
7. **Test strategy:** **vitest unit (paired mobile/desktop branches) + Playwright smoke on Mobile Safari device target** (~10 cases). Adds Playwright as a new dev dep.
8. **Mobile upgrade page:** **2 tiers (Free + Pro)**, matching the desktop deviation from the handoff (Studio has no Stripe price).
9. **Login mobile treatment:** **faithful design (option A)** — separate `UiAuthShellMobile.vue` with floating-poster fan + atmosphere radial gradients. Real AniList covers (3 random) with procedural OKLCH fallback.
10. **Mobile nice-to-haves:** **drag-to-dismiss bottom sheets (via `vaul-vue`)** + **rubber-band scroll polish** are in scope. **Pull-to-refresh dropped entirely** — browser-native is sufficient; no manual refresh icon substitute.

## Non-goals (explicit)

- Tablet-specific layout (intermediate breakpoint).
- Native iOS/Android app shells.
- Pull-to-refresh on web app surfaces.
- PWA installability / web manifest / standalone display mode (separate workstream if desired).
- New global Schedule view, anime Discover/Browse view, or any other new feature surface — Track 3 is a responsive pass plus the mobile-specific patterns.
- Studio tier resurrection on either viewport.

## Architecture

### Viewport composable

New file: `frontend/src/composables/useViewportLayout.ts`

```ts
export const MOBILE_BREAKPOINT_PX = 1024

export function useViewportLayout() {
  // singleton: one shared ref<boolean>, one shared resize listener,
  // 100ms debounced; SSR-safe default; cleanup on onScopeDispose.
  return { isMobile, MOBILE_BREAKPOINT_PX }
}
```

- Singleton pattern — first call registers the resize listener; every consumer reads the same ref.
- Debounce 100 ms.
- Test seam: stubbed by mocking the module from a test util (`__tests__/test-utils/viewport.ts`).

### Route topology

| Route | Layout |
|---|---|
| `/login`, `/register`, `/forgot-password`, `/reset-password`, `/verify-email/*` | Branches on viewport inside `UiAuthShell` → desktop split or `UiAuthShellMobile` |
| `/calendars` | `MyCalendarsPage.vue` — single file, internal `lg:` branching |
| `/calendar/:id` | `CalendarEditorView.vue` thin shell → `CalendarEditorViewDesktop` (renamed from existing) or `CalendarEditorViewMobile` (new) |
| `/account` | NEW: list-of-sections route (mobile drilldown landing); desktop renders sidebar layout |
| `/account/profile`, `/account/preferences`, `/account/password`, `/account/subscription`, `/account/danger` | NEW sub-routes; desktop renders alongside sidebar, mobile renders full-width with back arrow |
| `/upgrade`, `/upgrade/success`, `/upgrade/cancelled` | Single file, internal `lg:` branching |
| `/404`, etc. | Single file |

### Bottom tab bar

New file: `frontend/src/components/ui/UiBottomTabBar.vue`

- Renders only when `isMobile && route.meta.bottomTabBar !== false`
- Public routes set `meta: { bottomTabBar: false }` in router config
- Two tabs: **Library** (`/calendars`) and **Account** (`/account`)
- Active tab is route-derived: paths starting with `/calendar` or `/calendars` → Library highlighted; paths starting with `/account` → Account highlighted; else neither
- Position: `sticky` inside the route's main scroll container (not `fixed`) — keyboard interaction-friendly
- Padding-bottom: `max(22px, env(safe-area-inset-bottom) + 8px)` for home-indicator clearance
- Backdrop blur via `backdrop-filter: blur(20px) saturate(160%)` matching design

### Auth shells

- `UiAuthShellMobile.vue` (new) — floating-poster fan (3 posters at `translateY(0/-20/5) rotate(-4/0/4)`), atmosphere radial gradients top, bottom-anchored form, brand pip + wordmark
- `UiAuthShell.vue` (existing) — viewport-branching shell that picks desktop split-pane or `UiAuthShellMobile`
- Login/Register/Forgot/Reset/VerifyPending all use the same shell — get mobile treatment for free

### Editor split

- `CalendarEditorView.vue` becomes a thin shell:
  ```vue
  <CalendarEditorViewMobile v-if="isMobile" />
  <CalendarEditorViewDesktop v-else />
  ```
- `CalendarEditorViewMobile.vue` (new) is lazy-loaded via dynamic import so desktop-only users don't pay the bundle cost.
- Existing logic in `CalendarEditorView.vue` migrates verbatim to `CalendarEditorViewDesktop.vue`.

### Account drilldown refactor

This is the largest unanticipated work item.

- Currently `/account` is a single route with internal tab state.
- Mobile drilldown UX requires real navigation entries so the device back button works.
- Refactor: `/account` becomes a list-of-sections landing route. Each tab becomes a sub-route (`/account/profile`, `/account/preferences`, `/account/password`, `/account/subscription`, `/account/danger`). The tab content components (`ProfileTab.vue`, etc.) are already isolated, so the lift is mostly router config + layout shell change.
- Desktop renders the section list as a sidebar with the active sub-route on the right (existing visual outcome, different routing structure).
- Mobile renders just the active sub-route at full width with a back arrow that returns to `/account`.

### iOS / safe-area

- TopBar padding-top: `max(54px, env(safe-area-inset-top) + 12px)` — design's 54 px is for notched iPhones; non-notched browsers get 54 px via `max()`.
- Tab bar padding-bottom: `max(22px, env(safe-area-inset-bottom) + 8px)`.
- `<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content">` added to `index.html`.
- Full-height containers use `100dvh` with `100vh` fallback for older browsers.

## Components

### Group A — responsive-only (existing markup, audit + small adjustments)

`NotFoundPage`, `VerifyEmailPendingPage`, `VerifyEmailConfirmPage`, `ForgotPasswordPage`, `ResetPasswordPage`, `UpgradeSuccessPage`, `UpgradeCanceledPage`.

Work per file: review viewport behavior at 390 px, fix `lg:` prefixes, scale type with the design's mobile values (display 36–48, body 14–15), make CTAs full-width.

### Group B — significant restructure, no new business logic

- **MyCalendarsPage** — desktop 3-col grid → mobile single-column stacked cards (h-24 collage strip + meta row). Dashed "+ New calendar" tile becomes a bottom row. Large TopBar variant ("My *calendars*" italic display headline).
- **AccountPage** — desktop sidebar tabs → mobile vertical sectioned list with drilldown sub-routes (see Architecture).
- **CalendarScheduleView** — day pills become horizontal-scroll, time-anchored episode rows stack at full width, active day pill uses `bg-accent-1`.
- **UpgradePage** — Free + Pro tiers stack vertically. Pro card retains accent border + glow shadow + "Most popular" chip. Atmosphere radial gradient + italic display headline preserved at smaller scale.
- **UpgradeInterruptModal** — desktop centered modal becomes mobile bottom sheet (via `vaul-vue`'s `<Drawer>`). Same content, different transform. Drag-to-dismiss + Esc + close-button + backdrop-click all close.

### Group C — net-new component

- **CalendarEditorViewMobile** (the big one):
  - Top bar: title + back arrow + settings cog
  - Full-width `UiSegmented` "Items · N / Search" below top bar
  - `<keep-alive>` around the two panel components — preserves scroll position + search query across tab swaps
  - **Items panel:** `MediaItemCard.vue` rows (banner fade-in mask preserved verbatim from Track 1 contract)
  - **Search panel:**
    - Empty state: chip suggestions + recent items (per design)
    - Results state: tap-to-select rows (no per-row Add buttons), banner fade-in on selected row, green `--success` border for already-added (dimmed, non-interactive)
    - Sticky batch-add CTA (`bottom: 90px` clears tab bar): "Add N to {calendar name}"
  - FAB on Items tab only: 56×56, `right: 18px`, `bottom: 104px`, `bg-accent-1`, jumps to Search tab + auto-focuses input
  - Recommendations section moved below items list (not side-by-side as on desktop)

### Mobile auth shell (separate)

- **UiAuthShellMobile** — floating-poster fan, atmosphere gradients, bottom-anchored form. AniList cover fetch with `sessionStorage` cache + procedural fallback.

### Cross-cutting

- `MediaItemCard.vue` — banner fade-in mask string unchanged. Mobile uses smaller poster (40×60) and tighter padding (10 px) but the *animation contract* is the same constant. Track 1 lock test continues to guard.
- `SubscriptionTab.vue` — gains the Nudge-B affordance for free users (tier chip + "Upgrade to Pro" CTA row). Past-due users see existing past-due banner *and not* the upgrade affordance.

## Data flow & state

- **Viewport state:** singleton ref in `useViewportLayout`.
- **Tab bar:** route-derived, no Pinia.
- **Editor segmented (mobile):** local `ref<'items' | 'search'>` inside `CalendarEditorViewMobile`. Not persisted.
- **FAB → Search:** sets segmented to `'search'` and `nextTick(() => searchInput.value?.focus())`.
- **Batch-add selection:** lifted to a Pinia store (`useEditorSelectionStore` or extension of existing calendar store) so a viewport flip doesn't drop the user's selection set. Cleared on route change.
- **Login fan posters:** `services/posters.ts` exports `getRandomCoverPosters(n)` — cached in `sessionStorage`, non-blocking AniList fetch, procedural fallback on error.
- **Drag-dismiss:** owned by `vaul-vue`'s `<Drawer.Root>`; `modelValue` bound to existing modal open state. Same close path as Esc / backdrop-click.
- **Pinia changes:** add the editor selection store; otherwise no changes.
- **Server changes:** none.

## i18n

New keys (added to both `en.json` and `pt.json` per project rule):

- `mobile.tabBar.{library,account}`
- `mobile.editor.{itemsTab,searchTab,fabAdd,batchAdd,emptySearch.title,emptySearch.subtitle,emptySearch.tryChips,emptySearch.recentLabel}`
- `mobile.editor.results.{tapToSelect,added,addNToTarget}`
- `mobile.schedule.{title,sub,emptyDay,headingItalic}`
- `mobile.account.backToList`
- `account.subscription.upgradeNudge.{ctaFree,tierChipFree}`
- `auth.mobile.posterAlt` (alt text for the floating posters)

Locale-parity vitest assertion is extended to cover `mobile.*` and the new `account.subscription.upgradeNudge` namespace.

## Testing

### vitest layer (unit)

- New helper: `frontend/src/__tests__/test-utils/viewport.ts` — `mockViewport(width)` mocks the `useViewportLayout` module before component import.
- Every viewport-branching component gets paired tests (`isMobile=true` and `isMobile=false`).
- New spec files:
  - `useViewportLayout.spec.ts` — composable behavior (toggle on resize, debounce, cleanup, SSR-safe default).
  - `UiBottomTabBar.spec.ts` — visibility gating, active tab calculation across all relevant routes.
  - `UiAuthShellMobile.spec.ts` — fan-poster slots, atmosphere layer, fallback rendering.
  - `CalendarEditorViewMobile.spec.ts` — segmented swap, FAB visibility/click, batch-add CTA, banner mask string preserved.
- Extended:
  - `MyCalendarsPage.spec.ts`, `AccountPage.spec.ts`, `SubscriptionTab.spec.ts`, `UpgradePage.spec.ts`, `UpgradeInterruptModal.spec.ts` — add mobile branch assertions.
  - Locale parity test extends to new namespaces.
  - Track 1 banner-fade-in lock test unchanged.

### Playwright layer (smoke)

- New: `frontend/playwright.config.ts`, `frontend/e2e/`, `npm run test:e2e`.
- Single project: Mobile Safari device target (iPhone 14 viewport).
- Cases (~10): auth login, library tile tap, editor tabbed flow, banner fade-in transition, account drilldown, upgrade page 2 tiers, tab bar visibility, viewport flip mid-session, safe-area assertion, drag-dismiss.
- CI: separate job parallel with vitest, ~2 min total. Fail-fast on viewport tests; rest may retry once.

### Acceptance for tests

`npm run test:unit` + `npm run test:e2e` + `npm run lint` + `npm run build` all green before Track 3 ships.

## Risks & mitigations

1. **iOS Safari `100vh`:** use `100dvh` with `100vh` fallback on full-height containers.
2. **AccountPage drilldown refactor:** real sub-routes for back-button correctness. Largest unanticipated work — phased explicitly in the commit topology.
3. **`vaul-vue` maturity:** pin version, vitest test for Esc + backdrop-click + drag-dismiss close paths, fallback to existing centered modal via feature flag if lib breaks.
4. **AniList fetch on Login:** non-blocking + sessionStorage cache + procedural fallback.
5. **Keep-alive memory:** scoped per-calendar; navigating away unmounts.
6. **Viewport flip mid-session:** lift selection state to Pinia so it survives the swap.
7. **iOS keyboard interaction:** tab bar is `position: sticky` (not `fixed`); viewport meta includes `interactive-widget=resizes-content`.
8. **Banner mask drift:** Track 1 contract test unchanged; new components import the same constant.
9. **Bundle size:** estimate +15 kb gz on index. Mitigate with lazy-loaded `CalendarEditorViewMobile` + `UiAuthShellMobile` (separate dynamic-import chunks). Acceptance: ≤ 20 kb gz growth.
10. **Drag-dismiss + screen readers:** keep close button + Esc as primary affordances; drag is additional.

Risks accepted without pre-emptive mitigation: older Android Chrome edge cases, iPad portrait getting desktop layout.

## Acceptance criteria

**Functional (per surface):**
- All 11 mobile surfaces render correctly at 390×844 without horizontal scroll, content cutoff, or tap-target overlap.
- Bottom tab bar renders only on authenticated mobile routes; never reflows; correct active highlighting.
- Editor mobile: Items↔Search swap preserves scroll + query; FAB only on Items; FAB → Search + focus; batch-add CTA appears when ≥1 selected; banner fade-in mask matches Track 1.
- Account drilldown: tapping a section pushes a sub-route; back arrow returns; deep-linking to `/account/subscription` lands directly.
- Free user on Account → Subscription sees the Nudge-B affordance; paid user does not; past-due user sees past-due banner only.
- Login renders the floating-poster fan; AniList fetch failure falls back to procedural posters without blocking the form.
- Drag-dismiss closes `UpgradeInterruptModal`; Esc + backdrop-click + close-button still close.

**Layout / UX:**
- All hit targets ≥ 44×44 px on mobile.
- Top safe-area ≥ 54 px or `env(safe-area-inset-top) + 12px`.
- Bottom safe-area: tab bar inner padding-bottom ≥ 22 px or `env(safe-area-inset-bottom) + 8px`.
- Type scale per design (display 36–48, body 14–15, micro 11).
- Viewport flip across 1024 px: layout swaps without route change; selection state survives.
- Full-height containers use `100dvh` with fallback.

**Tests + build:**
- vitest 100% pass, paired mobile/desktop tests on every branching component, locale parity extended.
- Playwright 100% pass on Mobile Safari device target, viewport-flip case included.
- Lint clean. Build clean. Bundle growth ≤ 20 kb gz.

## Suggested commit topology

**Phase 1 — Foundations (no user-visible changes)**

1. `feat(mobile): useViewportLayout composable + named breakpoint constant`
2. `feat(mobile): UiBottomTabBar primitive + route meta gating`
3. `chore(mobile): add vaul-vue dep + integration scaffold`
4. `feat(mobile): rubber-band scroll polish (overscroll-behavior + dvh + viewport meta)`
5. `chore(test): playwright config + mobile-safari device target + smoke harness`

**Phase 2 — Surface migration (user-visible changes)**

6. `refactor(account): split AccountPage into list + sub-routes`
7. `feat(mobile): AccountPage drilldown + SubscriptionTab Nudge-B`
8. `feat(mobile): MyCalendarsPage stacked layout + new-calendar dashed footer`
9. `feat(mobile): UiAuthShellMobile + Login/Register fan-poster shell`
10. `refactor(editor): split CalendarEditorView into desktop/mobile shells`
11. `feat(mobile): CalendarEditorViewMobile (segmented + FAB + keep-alive + batch-add CTA + banner fade-in)`
12. `feat(mobile): CalendarScheduleView mobile responsive`
13. `feat(mobile): UpgradePage + UpgradeInterruptModal (vaul-vue) + UpgradeSuccess responsive`
14. `feat(mobile): NotFoundPage + VerifyEmail* + Forgot/ResetPassword responsive sweep`
15. `docs(track-3): close out plan + capture deferred items`

Phase 1 commits don't need a feature flag — none of them change user-visible behavior on desktop. Phase 2 is where users on phones start seeing the new layouts.

## Deviations from design handoff (deliberate, recorded)

- **3-tab → 2-tab bottom bar.** Handoff specs Library / Editor / Account / Pro. We dropped Editor (it's a child of Library, not a peer) and Pro (asymmetric with desktop, no analytics to validate the conversion benefit). Resolves the "what does Editor mean from outside a calendar" question entirely.
- **No Studio tier on the mobile paywall.** Matches the Track 4 desktop deviation — no Stripe price exists.
- **Mobile pull-to-refresh dropped.** Web non-PWA PTR conflicts with browser-native PTR; no manual refresh substitute since browser-native is fine on its own.
- **Editor variants B + C dropped entirely.** Variant A was picked; the alternatives are not "deferred" — they're out of scope.

## Open questions for the implementation plan

These are *deliberately* not resolved here — they're sized for the implementation plan, not the design spec.

- Exact phasing within Phase 2: which of the 10 commits land before the others (e.g., does Account drilldown precede MyCalendars, or can they merge in either order without depending on each other?).
- Whether each Phase 2 commit ships behind a `mobile-track-3` feature flag during rollout, or directly merges (current plan: directly merge since each commit is independently testable).
- Specific `vaul-vue` version pin and whether to vendor a backup fallback.
