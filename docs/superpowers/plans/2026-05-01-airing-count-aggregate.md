# Airing-count aggregate (Track 2 follow-up)

**Date filed:** 2026-05-01
**Refined:** 2026-05-01 (Path B — derive from existing `airing_schedule`)
**Source:** FU-4 stretch from `docs/superpowers/plans/2026-05-01-track-2-design-diff.md`
**Status:** In progress

Goal: render an "N airing" chip per calendar tile + a "N airing this season" stat at the MyCalendarsPage header. Per-calendar count is computed from the existing `Item.airing_schedule` (any future-dated `airing_at` ⇒ counts as airing).

## Why Path B (vs original "MediaStatus" plan)

The original plan assumed AniList's `Media.status` enum was already pulled. It is not — neither the GraphQL schema (`anilist/schemas/items.graphql`) nor the queries (`anilist/queries/get_items.graphql`) include `status`. Adding it would require a new enum in `common`, schema/query edits, a cache version bump, and ~3h of plumbing across `common` + `anilist` + `server`.

Existing `Item.airing_schedule: Vec<Schedule>` already carries everything we need. An item is airing iff any `Schedule.airing_at` is in the future. Matches the user-facing "airing this season" semantic better than `MediaStatus::Releasing` would (a hiatus show wouldn't have upcoming episodes — Path B correctly excludes it).

Decisions (from refinement Q&A):
- **Definition of airing**: any future-dated `airing_at`. Can scope to e.g. 90 days later if it feels off.
- **No per-calendar aggregate cache** — derived in-memory from already-cached `Item` blobs (cheap), and avoids stale-window bugs.
- **Chip hidden at 0** — no chip / page-stat row when count is 0.
- **Tile chip placement**: bottom row inline with item count (`screens-extras.jsx:48` and README:36 — "X items · N airing"), warning-colored dot + label.

## Plan

### Step 1 — Backend: extend `PageCalendar` + compute count

**File:** `server/src/controllers/calendar.rs`

- [ ] Add `pub airing_count: usize` to `PageCalendar` (line 374) — match `item_count`'s type for consistency.
- [ ] Inject `anilist: web::Data<Anilist>` into `get_calendars` handler (line 411). Pattern matches sibling handlers at lines 88, 217, 292, 508.
- [ ] After the mapper returns `(calendars, total_count)`:
  1. Collect the union of `calendar.item_ids` across all paginated calendars into a deduped `Vec<Id>`.
  2. Single batched call: `let items = anilist.get_items(all_ids).await;`.
  3. Build a `HashMap<Id, &Item>` (or `HashMap<Id, &[Schedule]>`).
  4. Capture `let now = chrono::Utc::now().timestamp();` once before the loop.
  5. For each calendar, `airing_count = calendar.item_ids.iter().filter(|id| map.get(id).is_some_and(|item| item.airing_schedule.iter().any(|s| s.airing_at > now))).count()`.
- [ ] No new SQL → no `cargo sqlx prepare` needed.
- [ ] No cache key changes → no version bump.

### Step 2 — Backend tests

**File:** `server/src/controllers/calendar.rs` (existing `#[cfg(test)] mod tests`)

- [ ] Pure helper extracted as `fn count_airing(item_ids: &[Id], items_by_id: &HashMap<Id, &Item>, now: i64) -> usize` so the count logic is testable without standing up Anilist. Unit-test it directly:
  - Empty calendar → 0.
  - Calendar with one item that has only past `airing_at` → 0.
  - Calendar with one item whose schedule has at least one future `airing_at` → 1.
  - Calendar with three items, two with future episodes, one without → 2.
  - Item id present in `item_ids` but missing from `items_by_id` (Anilist miss) → not counted.
- [ ] If feasible, an integration test on `get_calendars` end-to-end is nice-to-have but not gating. The pure helper covers the logic; the controller wiring is mechanical.

### Step 3 — Frontend types

**File:** `frontend/src/types/calendar.ts`

- [ ] Add `airing_count: number;` to `PageCalendar` interface (line 12-20).

### Step 4 — Frontend tile chip

**File:** `frontend/src/components/shared/CalendarTile.vue` (modify the bottom-row block at lines 116–122)

Insert chip between item count and the existing separator+updated, hidden when 0:

```vue
<div class="flex items-center gap-2 text-sm text-fg-3">
  <span data-testid="calendar-tile-count">
    {{ t('calendars.tile.itemCount', { count: calendar.item_count }) }}
  </span>
  <span aria-hidden="true">·</span>
  <template v-if="calendar.airing_count > 0">
    <span
      data-testid="calendar-tile-airing"
      class="inline-flex items-center gap-1 text-warning"
    >
      <span class="w-1.5 h-1.5 rounded-full bg-warning" aria-hidden="true" />
      {{ t('calendars.tile.airing', { count: calendar.airing_count }) }}
    </span>
    <span aria-hidden="true">·</span>
  </template>
  <span data-testid="calendar-tile-updated">{{ updatedLabel }}</span>
</div>
```

### Step 5 — Frontend page-level stat

**File:** `frontend/src/components/MyCalendarsPage.vue`

- [ ] Add computed: `const totalAiring = computed(() => calendars.value.reduce((sum, c) => sum + (c.airing_count ?? 0), 0))`
- [ ] In the stats block (lines 18–26), append after `totalItems` (with separator, hidden at 0):
  ```vue
  <template v-if="totalAiring > 0">
    <span aria-hidden="true">·</span>
    <span data-testid="my-calendars-airing" class="inline-flex items-center gap-1 text-warning">
      <span class="w-1.5 h-1.5 rounded-full bg-warning" aria-hidden="true" />
      {{ $t('calendars.stats.airing', { count: totalAiring }) }}
    </span>
  </template>
  ```

### Step 6 — i18n keys

**Files:** `frontend/src/locales/en.json`, `frontend/src/locales/pt.json`

- [ ] `calendars.tile.airing`: en `"{count} airing"` / pt `"{count} a transmitir"`
- [ ] `calendars.stats.airing`: en `"{count} airing this season"` / pt `"{count} a transmitir esta temporada"`

### Step 7 — Frontend tests

**Files:** `frontend/src/components/shared/__tests__/CalendarTile.spec.ts`, `frontend/src/components/__tests__/MyCalendarsPage.spec.ts` (or whatever the existing test files are named)

- [ ] CalendarTile: chip visible when `airing_count > 0`, hidden when 0. Use `data-testid="calendar-tile-airing"`.
- [ ] MyCalendarsPage: airing total visible only when at least one calendar has `airing_count > 0`. Use `data-testid="my-calendars-airing"`.
- [ ] Existing tests pass with the new field added to fixture calendars (default to 0).

### Step 8 — Verify

- [ ] `cargo build` (set `AWS_LC_SYS_PREBUILT_NASM=1` per project instructions)
- [ ] Backend tests green
- [ ] `cd frontend && npm run lint && npm run test:unit && npm run build` green
- [ ] Manual smoke: `/my-calendars` page shows the chip on calendars with future-airing items and the page-level total reflects them.

## Estimate

- Step 1 (backend wiring + count logic): ~45 min
- Step 2 (backend tests): ~30 min
- Steps 3–6 (frontend types + tile + page + i18n): ~45 min
- Step 7 (frontend tests): ~30 min
- Step 8 (verify): ~15 min
- **Total: ~2.5h focused work**

## Suggested commits

1. `feat(calendars): add airing_count aggregate to PageCalendar list response`
2. `feat(calendars): render airing chip on tile and page-level airing stat`

## Out of scope

- Refresh job for Anilist data (cache TTL handles it).
- Showing airing status on the editor page item list.
- Scoping to a date window (e.g. "next 90 days") — defer if the simple "any future episode" feels off.
