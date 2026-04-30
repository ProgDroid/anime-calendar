# Track 2 — Existing Surfaces Redesign

**Source**: `design_handoff_anime_calendar/` (Claude Design hifi handoff)
**Master breakdown**: `2026-04-30-design-redesign-master-breakdown.md`
**Track 1 spec**: `2026-04-30-track-1-foundations-design.md`
**Date**: 2026-04-30
**Status**: Approved design. Ready for implementation plan.

## 1. Goals & Non-Goals

### Goals

1. Re-skin every existing user-visible screen to the handoff design: Login, Register, ForgotPassword, ResetPassword, VerifyEmailPending, VerifyEmailConfirm, NotFound, MyCalendars, CalendarPage (split view), and the new tabbed Account page consolidating UserDetailsPage + UserSettingsPage.
2. Add a **Weekly schedule** sub-route at `/calendar/:id/schedule` — Mon–Sun columns, week navigation via `?week=YYYY-Www`, "not airing this week" section below the grid.
3. Restructure Account into `/account/{profile,preferences,password,danger}` sub-routes with shared sidebar chrome. Old `/user/details` and `/user/settings` redirect (replace) to corresponding new paths.
4. Surface the accent picker as 5 first-class accent options in Preferences. Matcha/Sakura/Citron carry an informational `Pro` chip; selection works for all 5 ("Everything is free during early access" copy explains).
5. Migrate Toast (`Toast.vue` → `<UiToast>` via unchanged `toastService` API) and ConfirmModal seams. Delete legacy components.
6. Backend: `PageCalendar` DTO gets `recent_item_ids: Vec<i32>` (≤4 most-recent); requires new `calendar_items.added_at` column.
7. **Final commit removes DaisyUI** from the project.

### Non-goals (deferred)

- Multi-editor calendars / avatar stacks of editors. Avatar slot on MyCalendars tiles renders the single owner only. Multi-editor is its own future feature cycle (handoff designs reusable when picked up).
- Upgrade flow surfaces (Track 4).
- Mobile responsive pass (Track 3).
- Storybook / visual review tooling.
- Local items table or item caching infrastructure beyond the existing AniList/Redis pattern.

## 2. Surface groups & ordering

Track 2 ships in five sequential groups. Each ends with a coherent slice; tests update alongside.

### Group 1 — Auth & states

Centered single-column wordmark composition shared across all.
- `LoginPage.vue` — split-screen, floating poster collage on left, centered form column on right
- `Register.vue` — matches Login layout
- `ForgotPasswordPage.vue`, `ResetPasswordPage.vue`
- `VerifyEmailPendingPage.vue`, `VerifyEmailConfirmPage.vue`
- `NotFoundPage.vue`
- New: `components/shared/EmptyState.vue` (centered single-column composition, reused by 404 and future empty states)
- All inputs swap to `<UiInput>`; all buttons to `<UiButton>`; wordmark + accent system applied

### Group 2 — MyCalendars

- `MyCalendarsPage.vue` becomes 3-column grid of poster-collage tiles
- New: `components/shared/CalendarTile.vue` (tile shell)
- New: `components/shared/PosterCollage.vue` (4-up image grid; fills missing slots with `--accent-1-soft`)
- New: empty-tile (dashed border + plus icon "Create calendar") as a tile variant
- Backend: `calendar_items.added_at` migration + `PageCalendar.recent_item_ids` DTO field + mapper update

### Group 3 — Calendar editor & Weekly schedule

- `CalendarPage.vue` becomes parent shell with `<router-view />` + segmented toggle (Items / Schedule)
- New child: `components/calendar/CalendarSplitView.vue` (existing items list + search panel, re-skinned)
- New child: `components/calendar/CalendarScheduleView.vue` (Mon–Sun grid + week nav + not-airing section)
- New child: `components/calendar/ScheduleDayColumn.vue` (single day column, used by ScheduleView)
- New helper: `composables/useWeekRange.ts`
- New helper: `composables/useDayLabels.ts`
- Re-skin: `CalendarItemsList.vue`, `ItemSearchPanel.vue`, `RecommendationsSection.vue`, `MediaItemCard.vue`
- `MediaItemCard.vue` adopts `<UiBannerFade>` for selected state
- Routes: `/calendar/:id` (default child = SplitView); `/calendar/:id/schedule` (child = ScheduleView)

### Group 4 — Account restructure

- New: `components/account/AccountPage.vue` (sidebar shell + `<router-view />`)
- New tabs: `AccountProfileTab.vue`, `AccountPreferencesTab.vue`, `AccountPasswordTab.vue`, `AccountDangerTab.vue`
- New: `components/account/AccentPicker.vue` (5-up accent picker with Pro chips)
- New constant: `frontend/src/constants/accents.ts` — `PRO_ACCENTS = new Set(['matcha', 'sakura', 'citron'])`
- Migrate logic from `UserDetailsPage.vue` (profile + danger zone) and `UserSettingsPage.vue` (preferences) into corresponding tabs
- Routes added: `/account/profile`, `/account/preferences`, `/account/password`, `/account/danger` (lazy-loaded)
- Old routes become redirects: `/user/details` → `/account/profile`, `/user/settings` → `/account/preferences` (replace, not push)
- Delete: `UserDetailsPage.vue`, `UserSettingsPage.vue`

### Group 5 — Cleanup & DaisyUI removal

- Migrate `Toast.vue` consumers via the `toastService` seam: internal rendering swaps to `<UiToast>`; `Toast.vue` deleted
- Migrate `ConfirmModal.vue`: wraps `<UiModal>` (or replaced if call sites can target `<UiModal>` directly); legacy deleted
- Pre-removal audit: grep for any remaining `btn|btn-|card|card-|input-bordered|modal|modal-|dropdown|dropdown-|tabs|tab-|alert|badge` class references AND `var(--p|s|b1|...)` DaisyUI variable references. Zero matches required.
- Remove `@plugin "daisyui"` from `frontend/src/assets/main.css`
- Remove vestigial `@tailwind base/components/utilities` lines (Tailwind v4 doesn't need them)
- Remove `daisyui` from `package.json` `devDependencies`; run `npm install`
- Update `frontend/tailwind.config.cjs`: drop `require('daisyui')` from plugins, delete the `daisyui` config block. If config file becomes empty in Tailwind v4, delete the file
- Run vue-i18n-auditor as a final sweep
- Run full gate sequence (type-check + test:unit + lint + build) and Playwright smoke across every route
- Bundle-size sanity: confirm DaisyUI removal yields a measurable bundle reduction

## 3. Backend changes

### Migration: `calendar_items.added_at`

```sql
ALTER TABLE calendar_items
    ADD COLUMN added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP;
```

Backfill: `CURRENT_TIMESTAMP` for all existing rows. Acceptable; legacy ordering becomes arbitrary but stabilizes for new inserts. Resolves the master breakdown's open concern about persistence ordering for "recently added."

**Pre-task**: resolve the pre-existing dev DB checksum drift on migrations `20260401000000`, `20260415000000`, `20260415000001` before adding this migration. Running `cargo sqlx prepare` is unaffected, but plain `sqlx migrate run` is blocked. Two paths: reset the dev DB and replay all migrations, or update `_sqlx_migrations` checksums to match current file contents. Pick whichever is less disruptive.

### `PageCalendar` DTO

Current shape (`server/src/controllers/calendar.rs`):

```rust
pub struct PageCalendar {
    pub id: Id,
    pub item_count: usize,
    pub name: String,
    pub subscription_token: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

Add:

```rust
pub recent_item_ids: Vec<i32>,  // up to 4, ordered by added_at DESC
```

The backend returns IDs only, not URLs. AniList rate-limit pressure stays off the calendars-list path.

### Mapper change

`get_calendars_by_user_paginated` (or its successor) returns recent IDs per calendar. Approximate SQL using a window function:

```sql
WITH ranked AS (
    SELECT ci.calendar_id,
           ci.item_id,
           ROW_NUMBER() OVER (PARTITION BY ci.calendar_id ORDER BY ci.added_at DESC) AS rn
    FROM calendar_items ci
)
SELECT c.id, c.name, c.subscription_token, c.created_at, c.updated_at,
       COALESCE(COUNT(ci.item_id) FILTER (WHERE ci.item_id IS NOT NULL), 0) AS item_count,
       COALESCE(ARRAY_AGG(ranked.item_id ORDER BY ranked.rn) FILTER (WHERE ranked.rn <= 4), '{}') AS recent_item_ids
FROM calendars c
LEFT JOIN calendar_items ci ON ci.calendar_id = c.id
LEFT JOIN ranked ON ranked.calendar_id = c.id AND ranked.rn <= 4
WHERE c.user_id = $1 AND c.deleted_at IS NULL
GROUP BY c.id;
```

(Pseudo-SQL — finalized at implementation time.)

### Frontend resolves cover URLs in one batched call

After `getCalendars()` returns, the frontend collects the union set of `recent_item_ids` across all calendars (≤4 × N calendars), makes one batched `getItems(ids)` call, and builds a `Map<itemId, coverUrl>`. Two round-trips total, no N+1.

### Backend tests

- Existing calendar mapper tests get `recent_item_ids: vec![]` added to expectations.
- One new test: calendar with 5 items returns exactly 4 IDs in `added_at DESC` order.
- One new test: empty calendar returns empty `Vec`.

### Regenerate sqlx offline cache

Project rule: `cargo sqlx prepare --workspace` after any query macro change; commit `.sqlx/`.

## 4. Frontend architecture

### New file map

```
frontend/src/
├── components/
│   ├── shared/
│   │   ├── EmptyState.vue          # NEW
│   │   ├── PosterCollage.vue       # NEW — props: urls (0..4 strings)
│   │   └── CalendarTile.vue        # NEW
│   ├── calendar/
│   │   ├── CalendarSplitView.vue   # NEW — extracted from current CalendarPage body
│   │   ├── CalendarScheduleView.vue # NEW
│   │   └── ScheduleDayColumn.vue   # NEW
│   └── account/                    # NEW directory
│       ├── AccountPage.vue
│       ├── AccountProfileTab.vue
│       ├── AccountPreferencesTab.vue
│       ├── AccountPasswordTab.vue
│       ├── AccountDangerTab.vue
│       └── AccentPicker.vue
├── composables/
│   ├── useWeekRange.ts             # NEW — pure date math
│   └── useDayLabels.ts             # NEW — Intl.DateTimeFormat-driven
├── constants/
│   └── accents.ts                  # NEW — PRO_ACCENTS set
└── (deleted) components/Toast.vue, components/UserDetailsPage.vue, components/UserSettingsPage.vue
```

### Routing changes

```ts
// added — Account
{
  path: '/account',
  component: () => import('.../AccountPage.vue'),
  redirect: '/account/profile',
  children: [
    { path: 'profile',     component: () => import('.../AccountProfileTab.vue') },
    { path: 'preferences', component: () => import('.../AccountPreferencesTab.vue') },
    { path: 'password',    component: () => import('.../AccountPasswordTab.vue') },
    { path: 'danger',      component: () => import('.../AccountDangerTab.vue') },
  ],
}

// modified — CalendarPage gets children
{
  path: '/calendar/:id',
  component: () => import('.../CalendarPage.vue'),
  children: [
    { path: '',         component: () => import('.../CalendarSplitView.vue') },
    { path: 'schedule', component: () => import('.../CalendarScheduleView.vue') },
  ],
}

// redirects (replace by default in Vue Router 4 redirect routes)
{ path: '/user/details',  redirect: '/account/profile' }
{ path: '/user/settings', redirect: '/account/preferences' }
```

### Data flow per surface

- **Auth & states**: stateless re-skin; existing form state stays in component refs.
- **MyCalendars**: `getCalendars()` returns `recent_item_ids[]` per tile. Page collects union set, calls `getItems(ids)` once, builds `Map<itemId, coverUrl>`, passes per-tile slices to `<PosterCollage>`. Tiles with empty IDs receive `urls=[]` (placeholder fill). On `getItems` failure, tiles still render names + counts.
- **CalendarPage (parent)**: fetches calendar data once. Provides via a `useCurrentCalendar()` composable (or `provide/inject`) so both child views read the same source. Segmented toggle is two `<router-link>`s.
- **CalendarScheduleView**: reads `?week` query param via `useRoute()`. Computes Mon–Sun range via `useWeekRange().fromIsoWeek(weekParam, userTz)`. For each item, finds airing entries within range, groups by day. Items with zero entries within range go below the grid in "not airing this week." Week-nav buttons push `?week=YYYY-Www±1`. Malformed `?week` falls back to current week silently.
- **AccountPage**: owns the sidebar; tabs are child routes. `AccentPicker` calls `useTheme().setAccent()` directly — `useTheme` already handles persistence + server reconcile.

### `useWeekRange.ts` API

```ts
export function useWeekRange() {
  function fromIsoWeek(isoWeek: string, tz: string): { start: Date; end: Date } { /* Mon 00:00 – Sun 23:59:59 in tz */ }
  function currentIsoWeek(tz: string): string { /* 'YYYY-Www' for now in tz */ }
  function shiftIsoWeek(isoWeek: string, delta: number): string { /* ±N weeks */ }
  return { fromIsoWeek, currentIsoWeek, shiftIsoWeek }
}
```

Pure functions, no Vue reactivity inside. Consumer holds reactive state. ~50 lines, no new dependency (`Intl.DateTimeFormat` + `Date`).

### `useDayLabels.ts` API

```ts
export function useDayLabels() {
  const { locale } = useI18n()
  // Returns ['Mon','Tue',...] localized via Intl.DateTimeFormat(locale.value, { weekday: 'short' })
  const labels = computed(() => /* ... */)
  return { labels }
}
```

## 5. DaisyUI removal

Already detailed in Section 2 / Group 5. Key gates:

1. Audit grep returns zero matches for DaisyUI classes AND DaisyUI variables.
2. Build still produces working CSS.
3. Bundle size measurably smaller.
4. Playwright smoke across every route confirms visual integrity.

## 6. Testing, error handling, edge cases, accessibility

### Testing

- Each re-skinned page: existing `.spec.ts` continues passing. DaisyUI-class selectors get rewritten to `data-testid`.
- New components: focused specs per file.
- `useWeekRange.ts`: pure-function tests for DST transitions, year rollover, ISO week numbering edge cases.
- `useDayLabels.ts`: tests with `en` and `pt` locales.
- Integration: `MyCalendarsPage.spec.ts` mocks `getCalendars` + `getItems`, asserts cover URL routing.
- Backend: new `added_at` migration test, `recent_item_ids` mapper happy-path + edge cases (0 items, >4 items).

### Error handling

- AniList failure post-calendar-list: tiles render with `urls=[]` placeholder; calendar names + counts intact. No spinner, no error toast.
- Schedule view with empty week: grid renders with empty columns; "not airing this week" section below contains all items.
- Malformed `?week`: silently falls back to current week.
- Account legacy redirects: `replace`-style, no back-button pollution; covered by router test.
- Accent change with offline server: `useTheme` already handles — local update immediate, PATCH error logged, next `fetchSettings` reconciles.

### Edge cases

- Week rollover during view: `currentIsoWeek(tz)` recomputes on refresh; `?week` stays stable when explicitly navigated.
- Long calendar names: `text-overflow: ellipsis` + 1-line clamp on tile.
- Sub-route param change: Vue Router 4 re-instantiates child components when parent `:id` changes.
- Account tab switch during async PATCH: module-scoped debounce in `useTheme` continues regardless of which tab is mounted.

### Accessibility

- AccountPage sidebar: `<nav>`, `aria-current="page"` on active link.
- Segmented toggle: `<UiSegmented>` already provides `role="tablist"` + `aria-selected`.
- Calendar tiles: full tile is one `<router-link>`; `aria-label="Open {name}"`; poster collage `aria-hidden="true"`.
- EmptyState: headline is `<h1>`/`<h2>` per route context; icons `aria-hidden`.
- Schedule view week-nav buttons: `aria-label="Previous week"` / `"Next week"`. Current week label `aria-live="polite"`.
- Reduced motion: already honored by tokens.css and Track 1 components.
- Color contrast: spot-check `--fg-3` on tile / sidebar backgrounds during implementation.

### Cross-cutting project rules

- All new strings via `t()` in **both** `en.json` and `pt.json`.
- vue-i18n-auditor runs at end of Track 2.
- `axios.isAxiosError(err)` for any error code branching.
- `data-testid` selectors over class selectors in tests.
- sqlx: regenerate `.sqlx/` after migration + mapper changes; resolve dev DB checksum drift first.
- `/account/*` is **not** public; the existing auth guard already blocks unauthenticated access.
- `auth.login.title`-style hierarchical i18n key naming. New top-level namespaces likely: `account`, `schedule`, `tile`, `empty`.

## 7. Out of scope / open questions

- Pro chip UI in Track 2 is informational; gate-triggering behavior lands in Track 4.
- Item details cache (local items table, item cover URL Redis cache) — not in this track. If AniList rate limits become a problem, address separately.
- Tablet breakpoints — Track 3 territory.
- Light theme polish per surface — fix as discovered during smoke; punt non-blocking issues.
