# Audit follow-up Step 3 — Shared editor composable + H-3 + F2-10 + F2-12

**Date:** 2026-06-11
**Scope:** Audit follow-up sequence step 3 (see `AUDIT.md` → "Recommended sequence" → step 3).
**Findings closed:** F2-11, F2-12 (root cause + fix), H-3, F2-10.
**Method:** One frontend refactor (shared composable) that surfaces F2-11/F2-12, plus two contained fixes (H-3 backend, F2-10 frontend).

---

## Goals

1. **F2-11** — eliminate the editor draft-persistence bugs: mobile's dead stub that still deletes the desktop draft on unmount; the global draft key that shows calendar A's draft under `/calendar/B`; drafts lost on desktop↔mobile viewport flip.
2. **F2-12** — SSE item toasts ("42 added an item") must render a human display name.
3. **H-3** — close the concurrent-`add_item` cap bypass with the same advisory-lock pattern `put` uses.
4. **F2-10** — stop the router guard from clobbering theme/locale with fabricated defaults for unauth users, and from desyncing `data-theme` behind `useTheme`.
5. Extract the ~85-line `submitCalendar`, the SSE watcher, and the bootstrap/load logic that are duplicated near-verbatim between the two editor views into one composable, so the F2-11/F2-12 fixes live in one place.

Non-goals: changing the editor UX, search behavior, or the owner-funded show-cap semantics; touching `applySettings`'s other 3 call sites; the broader F2-28 store-hygiene cluster.

---

## Part A — `composables/useCalendarEditor.ts` (new)

A single orchestrator composable owning the **shared core** of both editor views. Matches the existing composable idiom (`usePresence`, `useCalendarSearch`, `useRecommendations`).

### Signature

```ts
export function useCalendarEditor(): {
  // identity / route
  calendarId: string            // route param, 'new' for create
  isExistingCalendar: boolean
  // editor state
  calendarName: Ref<string>
  calendarLanguage: Ref<'english' | 'romaji' | 'native'>
  calendarEventStyle: Ref<EventStyle>
  itemsInCalendar: Ref<Item[]>
  calendarError: Ref<string | null>
  currentCalendar: Ref<Calendar | null>
  submitLoading: Ref<boolean>
  isFreeTier: Ref<boolean>
  // derived
  isOwner: ComputedRef<boolean>
  editorShowCount: ComputedRef<number | null>
  // item mutations (shared core)
  addItems: (items: Item[]) => void
  removeItemFromCalendar: (id: number) => void
  clearCalendar: () => void
  submitCalendar: () => Promise<void>
  // presence / live sync
  viewers: Ref<Viewer[]>
  showCollisionBanner: Ref<boolean>
  reloadPage: () => void
  // recommendations
  recommendations: Ref<Item[]>
  calculateRecommendations: (items: Item[]) => void
}
```

Notes:
- **`loading` stays in each view.** Desktop computes `loading = searchLoading || submitLoading`; mobile computes `loading = submitLoading`. The composable exposes `submitLoading` only.
- **`addItems(items: Item[])`** is the shared add core: filter out items already in `itemsInCalendar`, push, `calculateRecommendations`, and `if (isFreeTier) void usage.refresh()`. Each view keeps a thin wrapper:
  - Desktop `addItemToCalendar()` → `addItems(fetchedItems.filter(selected & not present))` + clear `selectedItems`.
  - Desktop `addItemToCalendarSingle(item)` → `addItems([item])`.
  - Mobile `addItemFromSearch(items)` → `addItems(items)` + `selection.clear()` + `tab = 'items'`.
  - Mobile `addRecommendation(item)` → `addItems([item])`.
- The composable internally instantiates `usePresence(numericId)` (existing-calendar only), `useRecommendations`, `useUsageStore`, `useUserSettingsStore`, `useAuthStore`, `useI18n`, `useRouter`, `useRoute`.
- The SSE `watch(lastEvent, ...)` (collision banner + toasts + member/kick) lives inside the composable. Kick → `router.push('/my-calendars')`. **F2-12 fix applied here** (see Part C).
- The tier-bootstrap + calendar-load + user-settings load runs once at composable setup (an internal `void (async () => {...})()`), identical to today's logic. `currentUserId` becomes an internal ref feeding `isOwner`.

### What stays in each view

- **Desktop**: `useCalendarSearch` (`fetchedItems`/`selectedItems`/`handleSearch`/`toggleItemSelection`), its `loading` computed, the add wrappers, the two-column template.
- **Mobile**: `useEditorSelectionStore`, tab state + `tabOptions` + `jumpToSearch` + FAB, `onBeforeRouteUpdate`/`onBeforeRouteLeave` selection clear, the add wrappers, the mobile template.

---

## Part B — F2-11: keyed, portable drafts (inside `useCalendarEditor`)

- **Key:** `calendarDraft:${calendarId}` where `calendarId` is the route param (`'new'` or the numeric id). Replaces the global `'calendarPageState'`.
- **Persisted shape (core fields only):**
  ```ts
  { calendarName, calendarLanguage, calendarEventStyle, itemsInCalendar }
  ```
  Desktop's transient `fetchedItems`/`selectedItems` are **no longer persisted** — making the draft portable across the desktop↔mobile viewport flip (their search UIs differ) and shrinking the blob. Trade-off accepted in brainstorming: on reload the search box starts empty.
- **Restore:** on setup, read `calendarDraft:${calendarId}`; apply only if present. Because the key is id-scoped, a draft for A is never applied under B. For existing calendars, the draft (if any) is applied *after* the server load resolves so unsaved local edits win — matching today's desktop behavior where `onMounted` restore ran and `onBeforeMount` load also ran. **Ordering:** restore overlays the server-loaded values (restore runs after load completes) so in-progress edits survive a reload of an existing calendar. For `new`, there is no server load.
- **Persist:** debounced 1000ms watcher on the 4 core fields (cleared in `onBeforeUnmount`), on **both** views.
- **Clear:** on successful `submitCalendar` (remove the id-scoped key); and in `onBeforeUnmount` only when `calendarId === 'new'` (mirrors today's new-draft cleanup, now id-scoped so it can't wipe a sibling).

This is a behavior change worth stating plainly: existing-calendar drafts now persist per-id (previously a single shared slot), and survive viewport flips.

---

## Part C — F2-12: backend `display` on item frames

### Backend
- `services/calendar_events.rs`: add `display: String` to `CalendarEvent::ItemAdded` and `CalendarEvent::ItemRemoved` (after `actor`). Update the doc comment to note item frames now carry a human `display` alongside the numeric `actor`.
- `controllers/calendar.rs::add_item` and `::remove_item`: inject `web::Data<UserMapper>`; source `display` exactly like `accept_invitation`:
  ```rust
  let display = users.get_user_by_id(actor_id).await.ok()
      .map(|u| u.username).unwrap_or_else(|| actor_id.to_string());
  ```
  Fetch once, before publishing. Set `display` on the published frame. (In `add_item` this slots into the H-3 rework; see Part D.)
- Register `UserMapper` is already an injected `web::Data` (used elsewhere) — confirm in the test app builders that exercise these two handlers.

### Frontend
- `types/sharing.ts`: add `display: string` to the `item_added` and `item_removed` members of the `CalendarEvent` union.
- Both editor watchers (now single, in `useCalendarEditor`): change `t('sharing.toasts.itemAdded', { actor: frame.actor })` → `{ actor: frame.display }` (and `itemRemoved`). The echo filter still uses `frame.actor !== String(authStore.userId)` (numeric id) — unchanged.
- e2e/unit stubs that publish item frames must include `display`.

No i18n key changes (`sharing.toasts.itemAdded/itemRemoved` already take an `{actor}` slot; we just feed it the display string).

---

## Part D — H-3: advisory lock on `add_item`

Mirror `put`'s pattern. In `controllers/calendar.rs::add_item`:
1. Inject `pool: web::Data<sqlx::PgPool>`.
2. Keep `get_by_id_any_owner` + `authz.assert_can(... ItemMutate)` **pool-bound** (read-only; not part of the race).
3. `let mut tx = pool.begin()...`; `pg_advisory_xact_lock(i64::from(cal.user_id))` — lock on the **owner** id (owner-funded cap), matching how `put` locks the writing user.
4. Inside the lock, **owner only** (`actor_id == cal.user_id`): `entitlement.assert_can_add_show_in_tx(&mut tx, actor_id, body.item_id).await` (already exists; used by `put`).
5. `CalendarMapper::add_item_idempotent_with(&mut *tx, calendar_id, body.item_id).await` (the `_with` variant already exists at `mappers/calendar.rs:716`).
6. `tx.commit()`.
7. Post-commit: cache invalidation + `publish_calendar(ItemAdded{..display..})` exactly as today (now with the Part C `display`).

Editor-initiated adds (actor ≠ owner) still skip the cap check (unchanged owner-funded semantics) but now also run the insert inside the lock — harmless and consistent.

`remove_item` does **not** need the lock (no cap), but **does** get the Part C `display` change (inject `UserMapper`, source display).

### Test (H-3)
A backend integration test asserting two concurrent owner `add_item`-equivalents cannot exceed `free_show_cap`. Given the codebase convention (`feedback_no_sqlx_test_use_test_pool`), use `test_pool()` and drive the in-tx cap path; assert the second over-cap insert errors with `PaymentRequired`. (A true concurrency race is hard to unit-test deterministically; the meaningful regression guard is that the cap check now runs `_in_tx` on the locked connection — assert the in-tx path rejects the over-cap add, mirroring `put`'s existing coverage.)

---

## Part E — F2-10: router-guard reconcile

In `router/index.ts` `beforeEach`, replace:
```ts
if (!to.meta.public) {
  const settings = await userSettingsStore.fetchSettings()
  if (settings) applySettings(settings)
}
```
with the canonical `main.ts` pattern:
```ts
if (!to.meta.public && authStore.isAuthenticated()) {
  const settings = await userSettingsStore.fetchSettings()
  useTheme().reconcileFromServer({
    theme_preference: settings.theme_preference,
    accent_preference: settings.accent_preference,
  })
  i18n.global.locale.value = settings.language_preference
}
```
- Guarding on `isAuthenticated()` removes the unauth fabricated-defaults clobber (the documented `feedback_unauth_default_reconcile` hazard).
- Routing theme/accent through `useTheme().reconcileFromServer()` instead of `applySettings`'s direct `data-theme` write removes the toggle-desync (`useTheme` is the single source of truth for theme/accent).
- `applySettings` is **retired from the router only**; its 3 other callers (`App.vue`, `PreferencesTab`, `DangerZoneTab`) keep it.
- `routerGuard.spec.ts` (currently asserts `applySettings` called / not-called) updates to assert `reconcileFromServer` + locale set when authed, and neither when unauth.

---

## Testing strategy

- **Composable**: `useCalendarEditor` is exercised through the two view specs (mount, assert behavior) rather than a standalone harness, since it depends heavily on route/stores/SSE. Existing `CalendarEditorViewDesktop`/`Mobile` specs must stay green; add cases for keyed-draft restore (A's draft not shown under B) and item-toast display name.
- **F2-11**: unit-level test of the keyed draft (restore matches id, clear on submit, no cross-id bleed). Use `createMemoryHistory` per `feedback_test_history_pollution`; never `vi.mock('vue-router')` (`feedback_vue_router_mock_leaks_across_workers`).
- **F2-12**: assert a published `item_added` frame with `display: 'Alice'` toasts "Alice added an item", not the numeric actor.
- **H-3**: backend in-tx cap-rejection test via `test_pool()`/`test_tx()`.
- **F2-10**: `routerGuard.spec.ts` rewrite (authed → reconcile + locale; unauth → neither).
- KeepAlive/jsdom navigation: spy on `router.push`, don't assert `currentRoute` (`feedback_vue_keepalive_jsdom_navigation`).

## Verification gate (sequential — `feedback_parallel_verification_flakiness`)
1. `AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server --lib`
2. clippy (project pedantic+nursery allow-list — `reference_clippy_command`) on touched files
3. `npm --prefix frontend run test:unit`
4. `npm --prefix frontend run lint`
5. `npm --prefix frontend run build`
6. `npm --prefix frontend run test:e2e`

Regenerate `.sqlx/` if any `sqlx::query!` changes (none expected — `add_item_idempotent_with` already cached).

## Commit plan (per `feedback_split_backend_frontend_on_breaking_api_shape`)
F2-12 changes a wire shape (item frame gains `display`). Ship backend (Parts C-backend + D) and frontend (Parts A, B, C-frontend, E) as **separate, separately-verified commits**. Order: backend first (additive field — old frontend ignores it), then frontend refactor + remaining fixes.

## Out of scope / deferred
- F2-28 store-hygiene cluster, M-30 calendar-fetch SQL consolidation, the rest of the Step 4 backlog.
- `applySettings`'s direct `data-theme` mutation for its other callers (PreferencesTab/DangerZone/App.vue) — only the router path is corrected here.
