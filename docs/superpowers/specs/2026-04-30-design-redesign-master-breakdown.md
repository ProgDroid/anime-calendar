# Design Redesign — Master Breakdown

**Source**: `design_handoff_anime_calendar/` (Claude Design hifi handoff, 2026-04-30)
**Status**: Master tracking doc. Each track below gets its own brainstorm → spec → plan → implementation cycle.
**Sequencing decision**: Option A — four sequential tracks. Foundations must land before any surface work; upgrade flow is deferred until monetization is on the roadmap.

## Why split

The handoff bundles four logically independent tracks. A single spec at this scope would be too vague to execute. Splitting preserves YAGNI (track 4 may never ship as designed) and lets each track ship in isolation behind well-defined token / component contracts.

## Tracks

### Track 1 — Foundations (BLOCKER for 2 and 3)

Design tokens + base components + iconography. Everything else depends on this.

- OKLCH accent system (Coral / Iris / Matcha / Sakura / Citron) wired to `:root` CSS vars, swappable at runtime.
- Type scale (Geist / Geist Mono / Instrument Serif), radii, spacing, motion, shadows.
- Base components: Button (Primary/Secondary/Ghost/Danger × sm/md/lg), Input, Segmented Control, Chip variants, Modal scrim, Toast, Avatar.
- **Banner fade-in primitive** (signature interaction — `MediaItemCard` selected state, `--d-3` 360ms `--ease-out`, mask values preserved verbatim from README).
- Icon set ported from `foundations.jsx` `window.Icon` — 24×24, 1.6 stroke (1.8 functional), `currentColor`. Do not substitute Lucide / Heroicons.
- Tailwind v4 + DaisyUI v5 integration: tokens live in `@theme` / CSS vars, not in `tailwind.config`. DaisyUI themes (`light` / `dark`) extended with the accent var system.
- Theme + accent state: persisted in `userSettingsStore` (already exists), applied to `:root` via a small composable.

**Files most affected**: `frontend/src/assets/main.css`, `frontend/tailwind.config.cjs`, new `frontend/src/components/ui/` directory, new `frontend/src/composables/useTheme.ts`, `frontend/src/components/shared/MediaItemCard.vue` (banner fade-in).

**Open questions**: font hosting (Google Fonts vs self-host from `vercel/geist-font`); whether to keep DaisyUI or replace with hand-rolled primitives.

### Track 2 — Existing surfaces redesign (depends on Track 1)

UI-only re-skin of every existing screen. No new business logic.

- **Login** (`LoginPage.vue`) — split layout, floating poster collage, Instrument Serif display headline.
- **My Calendars** (`MyCalendarsPage.vue`) — 3-col grid of poster-collage tiles, empty-tile CTA.
- **Calendar editor** (`CalendarPage.vue` + `calendar/*`) — split 60/40, items list (left) + search panel (right) with banner fade-in. Recommendations section below items.
- **Weekly schedule view** (NEW within editor) — `CalendarTimelineScreen` equivalent, Mon–Sun columns.
- **Account** (`UserDetailsPage.vue` + `UserSettingsPage.vue`) — sidebar tabs (Profile · Preferences · Password · Danger zone). Preferences lists 5 accent themes; Matcha/Sakura/Citron carry `Pro` chip but everything-free-during-early-access copy.
- **States** — `NotFoundPage.vue`, `VerifyEmailPendingPage.vue`, `VerifyEmailConfirmPage.vue`, `ForgotPasswordPage.vue`, `ResetPasswordPage.vue` re-skinned, plus an empty state.
- **Register** (`Register.vue`) — re-skin to match Login.

**i18n**: every new string lands in both `en.json` and `pt.json` (project rule).

**Open questions**: weekly schedule routing (sub-route of `/calendar/:id` or sibling?); whether the Pro chip on accents is shown today or hidden until Track 4.

### Track 3 — Mobile companion (depends on Track 2)

Responsive pass over Track 2. The handoff renders mobile inside an iOS device frame for canvas display only — production drops the frame and uses real viewport.

- Three editor variants in handoff; **A · Tabbed is recommended default** (segmented Items / Search toggle, coral FAB on Items tab, batch-add CTA over tab bar).
- Tab bar (Library / Editor / Account / Pro). "Pro" tab links to Track 4 surfaces; while Track 4 is deferred, this tab links to a placeholder or hides.
- Hit targets ≥44 px. FAB 18 px right / 104 px from bottom (clears tab bar). Items tab only.
- Search interaction (locked spec): tap-to-select, no per-row Add buttons. Selected = coral border + banner fade-in + filled coral check. Already-added = green `--success` border + "Added" chip, dimmed, non-interactive.
- Type scale sized down: display 36–48, body 14–15, micro 11.

**Open questions**: tablet breakpoint between 430 and 1280 (handoff explicitly defers); whether mobile gets a separate route tree or shares with desktop via responsive layout.

### Track 4 — Upgrade flow (DEFERRED until monetization lands)

Interrupt → Paywall → Checkout → Success. Backend + frontend + payments.

- Backend: Pro tier model, entitlement check, Stripe (Checkout vs embedded Elements — open question).
- Frontend: 4 new screens per `screens-upgrade.jsx`, plus access-gating logic on Pro accents.
- README explicitly says "implement when monetisation lands". Tier names and prices in handoff are illustrative.

**Do not start until**: business has signed off on Pro/Studio tiers and prices, and a payment processor is chosen.

## Cross-cutting rules

- Project conventions still apply: all strings via `$t()` in both locales; `axios.isAxiosError` guards; debounced storage watchers; auth guard untouched.
- `MediaItemCard.vue` banner fade-in is a contract — same exact gradient mask string desktop and mobile (see README line 178).
- Do **not** port `design-canvas.jsx` or `tweaks-panel.jsx` — they are canvas scaffold only.
- Posters in handoff are algorithmically generated; production uses AniList cover URLs (already wired via `anilist` crate).

## Sequencing

1. Track 1 — Foundations (this is brainstormed next, in this conversation)
2. Track 2 — Existing surfaces redesign (own brainstorm session)
3. Track 3 — Mobile companion (own brainstorm session)
4. Track 4 — Upgrade flow (own brainstorm session, deferred)

Each track produces its own `YYYY-MM-DD-<track>-design.md` spec and corresponding implementation plan. This master doc is the pointer.
