# Track 2 — Existing Surfaces Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-skin every existing screen onto the Track 1 token + component system, add the weekly schedule sub-route, restructure the account area into tabs, and remove DaisyUI.

**Architecture:** Frontend-heavy. One small backend change adds `calendar_items.added_at` and exposes a `recent_item_ids` array on `PageCalendar`. All other work is Vue templates swapping DaisyUI/raw markup for Track 1 `Ui*` primitives, plus a new sub-route `/calendar/:id/schedule` with two new view components. Account area is restructured from two pages into a shell + 4 child routes. DaisyUI is removed in the final phase after every consumer is migrated.

**Tech Stack:** Vue 3.5 + TS 5.9, Tailwind v4, Track 1 Ui primitives + tokens, vue-router 4 child routes, Pinia, vue-i18n 11, sqlx + Postgres, vitest + @vue/test-utils.

**Spec:** `docs/superpowers/specs/2026-04-30-track-2-existing-surfaces-design.md`

---

## Pre-Task: Resolve dev DB migration drift

**Context:** Three earlier migrations have checksum drift on the dev DB (see `memory/project_dev_db_migration_drift.md`). Task B1 adds a new migration; running it via the documented psycopg2 workaround perpetuates the drift. Resolve once, here.

**Files:**
- Modify: dev DB only (no repo files change)

- [ ] **Step 1: Inspect drift**

Run from repo root:

```
psql "$DATABASE_URL" -c "SELECT version, description, checksum FROM _sqlx_migrations ORDER BY version;"
```

Compare each row's `checksum` to `sha384sum migrations/<file>.sql` (or PowerShell `Get-FileHash -Algorithm SHA384`). Note the three drifted versions: `20260401000000`, `20260415000000`, `20260415000001`.

- [ ] **Step 2: Update checksums in place**

For each drifted version, copy the current file's sha384 hex digest and run:

```
psql "$DATABASE_URL" -c "UPDATE _sqlx_migrations SET checksum = decode('<hex>', 'hex') WHERE version = <version>;"
```

- [ ] **Step 3: Verify**

```
cd server && cargo sqlx migrate info
```

Expected: every migration shows `installed` with no checksum mismatch warnings.

- [ ] **Step 4: Update memory**

Edit `C:/Users/Nando Ferreira/.claude/projects/G--rustDev-anime-calendar/memory/project_dev_db_migration_drift.md` to mark drift resolved (date stamp 2026-04-30+).

No commit (DB-only change).

---

## Phase A — Auth, states, and shared shells

Re-skins of static screens. No business logic changes. Each task: replace DaisyUI/raw markup with Track 1 primitives, add testid hooks, ensure both `en.json` and `pt.json` strings exist.

### Task A1: `UiEmptyState` primitive

**Files:**
- Create: `frontend/src/components/ui/UiEmptyState.vue`
- Create: `frontend/src/components/ui/__tests__/UiEmptyState.spec.ts`
- Modify: `frontend/src/components/ui/index.ts` (export)

- [ ] **Step 1: Failing test**

```ts
import { mount } from '@vue/test-utils';
import UiEmptyState from '../UiEmptyState.vue';

describe('UiEmptyState', () => {
  it('renders title, body, and action slot', () => {
    const wrapper = mount(UiEmptyState, {
      props: { title: 'No items', body: 'Add one to begin' },
      slots: { action: '<button data-testid="cta">Add</button>' },
    });
    expect(wrapper.find('[data-testid="empty-state-title"]').text()).toBe('No items');
    expect(wrapper.find('[data-testid="empty-state-body"]').text()).toBe('Add one to begin');
    expect(wrapper.find('[data-testid="cta"]').exists()).toBe(true);
  });

  it('renders illustration slot when provided', () => {
    const wrapper = mount(UiEmptyState, {
      props: { title: 't', body: 'b' },
      slots: { illustration: '<svg data-testid="illo"/>' },
    });
    expect(wrapper.find('[data-testid="illo"]').exists()).toBe(true);
  });
});
```

- [ ] **Step 2: Run test, expect fail**

```
cd frontend && npx vitest run src/components/ui/__tests__/UiEmptyState.spec.ts
```

- [ ] **Step 3: Implement**

```vue
<script setup lang="ts">
defineProps<{ title: string; body: string }>();
</script>

<template>
  <div class="flex flex-col items-center justify-center text-center gap-4 p-8">
    <div v-if="$slots.illustration" class="opacity-80">
      <slot name="illustration" />
    </div>
    <h2 data-testid="empty-state-title" class="text-2xl font-display">{{ title }}</h2>
    <p data-testid="empty-state-body" class="text-base text-fg-muted max-w-prose">{{ body }}</p>
    <div v-if="$slots.action">
      <slot name="action" />
    </div>
  </div>
</template>
```

- [ ] **Step 4: Export**

In `frontend/src/components/ui/index.ts`, add `export { default as UiEmptyState } from './UiEmptyState.vue';`.

- [ ] **Step 5: Test passes**

```
cd frontend && npx vitest run src/components/ui/__tests__/UiEmptyState.spec.ts
```

- [ ] **Step 6: Commit**

```
git add frontend/src/components/ui/UiEmptyState.vue frontend/src/components/ui/__tests__/UiEmptyState.spec.ts frontend/src/components/ui/index.ts
git commit -m "feat(ui): add UiEmptyState primitive"
```

### Task A2: `Wordmark` brand component

**Files:**
- Create: `frontend/src/components/shared/Wordmark.vue`
- Create: `frontend/src/components/shared/__tests__/Wordmark.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { mount } from '@vue/test-utils';
import Wordmark from '../Wordmark.vue';

describe('Wordmark', () => {
  it('renders the app name', () => {
    const wrapper = mount(Wordmark);
    expect(wrapper.text()).toContain('Anime Calendar');
  });

  it('applies size variant', () => {
    const wrapper = mount(Wordmark, { props: { size: 'lg' } });
    expect(wrapper.attributes('data-size')).toBe('lg');
  });
});
```

- [ ] **Step 2: Run, expect fail**

- [ ] **Step 3: Implement**

```vue
<script setup lang="ts">
withDefaults(defineProps<{ size?: 'sm' | 'md' | 'lg' }>(), { size: 'md' });
</script>

<template>
  <span :data-size="size" class="font-display tracking-tight" :class="{
    'text-lg': size === 'sm',
    'text-2xl': size === 'md',
    'text-4xl': size === 'lg',
  }">
    Anime Calendar
  </span>
</template>
```

- [ ] **Step 4: Test passes**

- [ ] **Step 5: Commit**

```
git add frontend/src/components/shared/Wordmark.vue frontend/src/components/shared/__tests__/Wordmark.spec.ts
git commit -m "feat(shared): add Wordmark brand component"
```

### Task A3: Re-skin `LoginPage.vue`

**Files:**
- Modify: `frontend/src/components/LoginPage.vue`
- Modify: `frontend/src/components/__tests__/LoginPage.spec.ts`
- Modify: `frontend/src/locales/en.json`, `frontend/src/locales/pt.json` (verify keys exist; add `auth.login.tagline` if missing)

- [ ] **Step 1: i18n keys**

Add to both locale files under `auth.login`:
- `tagline`: en `"Track every season, share every calendar."` / pt `"Acompanhe cada temporada, partilhe cada calendário."`

- [ ] **Step 2: Update test for new structure**

Replace any DaisyUI class assertions with `data-testid` queries: `login-form`, `login-email`, `login-password`, `login-submit`, `login-google`, `login-forgot-link`, `login-register-link`, `login-tagline`.

- [ ] **Step 3: Re-skin layout**

Replace template with split layout: left = floating poster collage placeholder (`<div data-testid="login-poster-collage" class="hidden lg:block flex-1 bg-surface-elevated">`), right = card centered, max-w 420px. Inside card:
- `<Wordmark size="lg" />`
- `<h1 class="font-display text-display-md">{{ t('auth.login.title') }}</h1>`
- `<p data-testid="login-tagline">{{ t('auth.login.tagline') }}</p>`
- `<UiInput>` for email and password
- `<UiButton variant="primary" size="lg" data-testid="login-submit">`
- divider `<div class="text-fg-muted text-sm">{{ t('auth.login.or') }}</div>`
- `<UiButton variant="secondary" data-testid="login-google">`
- links: forgot password, register

Strip every `btn`, `input`, `card`, `divider` DaisyUI class. Use Track 1 primitives only.

- [ ] **Step 4: Tests pass**

```
cd frontend && npx vitest run src/components/__tests__/LoginPage.spec.ts
```

- [ ] **Step 5: Commit**

```
git add frontend/src/components/LoginPage.vue frontend/src/components/__tests__/LoginPage.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(auth): re-skin LoginPage to Track 1 system"
```

### Task A4: Re-skin `Register.vue`

**Files:**
- Modify: `frontend/src/components/Register.vue`
- Modify: `frontend/src/components/__tests__/Register.spec.ts`
- Modify: both locale files (verify `auth.register.*` keys; add `tagline` if missing)

- [ ] **Step 1: Mirror LoginPage layout** — same split shell, same Wordmark, same primitives.
- [ ] **Step 2: testids**: `register-form`, `register-email`, `register-password`, `register-confirm`, `register-submit`, `register-login-link`.
- [ ] **Step 3: Tests**: assert testids and submission flow unchanged.
- [ ] **Step 4: Run tests.**
- [ ] **Step 5: Commit.**

```
git add frontend/src/components/Register.vue frontend/src/components/__tests__/Register.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(auth): re-skin Register to Track 1 system"
```

### Task A5: Re-skin `ForgotPasswordPage.vue` and `ResetPasswordPage.vue`

**Files:**
- Modify: `frontend/src/components/ForgotPasswordPage.vue`
- Modify: `frontend/src/components/ResetPasswordPage.vue`
- Modify: corresponding spec files

- [ ] **Step 1**: Same auth shell. `UiInput`, `UiButton`. testids `forgot-email`, `forgot-submit`, `forgot-success`, `reset-password`, `reset-confirm`, `reset-submit`.
- [ ] **Step 2**: Tests assert testids; existing logic untouched.
- [ ] **Step 3**: Run tests.
- [ ] **Step 4**: Commit.

```
git add frontend/src/components/ForgotPasswordPage.vue frontend/src/components/ResetPasswordPage.vue frontend/src/components/__tests__/ForgotPasswordPage.spec.ts frontend/src/components/__tests__/ResetPasswordPage.spec.ts
git commit -m "feat(auth): re-skin password reset pages"
```

### Task A6: Re-skin verify-email pages

**Files:**
- Modify: `frontend/src/components/VerifyEmailPendingPage.vue`
- Modify: `frontend/src/components/VerifyEmailConfirmPage.vue`
- Modify: spec files

- [ ] **Step 1**: Auth shell + `UiEmptyState` for the pending state ("Check your inbox" with resend button in `action` slot).
- [ ] **Step 2**: Confirm page: spinner during processing, success/error state via `UiEmptyState`.
- [ ] **Step 3**: testids: `verify-pending`, `verify-resend`, `verify-confirm-status`.
- [ ] **Step 4**: Tests pass.
- [ ] **Step 5**: Commit.

```
git add frontend/src/components/VerifyEmailPendingPage.vue frontend/src/components/VerifyEmailConfirmPage.vue frontend/src/components/__tests__/VerifyEmailPendingPage.spec.ts frontend/src/components/__tests__/VerifyEmailConfirmPage.spec.ts
git commit -m "feat(auth): re-skin verify-email pages"
```

### Task A7: Re-skin `NotFoundPage.vue`

**Files:**
- Modify: `frontend/src/components/NotFoundPage.vue`

- [ ] **Step 1**: Replace with `<UiEmptyState :title="t('errors.notFound.title')" :body="t('errors.notFound.body')">` with `<UiButton @click="router.push('/')">` in action slot.
- [ ] **Step 2**: testid `not-found`.
- [ ] **Step 3**: Commit.

```
git add frontend/src/components/NotFoundPage.vue
git commit -m "feat(states): re-skin NotFoundPage"
```

### Task A8: Phase A gate

- [ ] Run full frontend suite: `cd frontend && npm run test:unit -- --run`
- [ ] Run typecheck + build: `cd frontend && npm run build`
- [ ] Run lint: `cd frontend && npm run lint`
- [ ] Manual smoke (via playwright MCP): visit `/login`, `/register`, `/forgot-password`, `/verify-email/pending`, `/404`. No console errors. All testids resolve.
- [ ] If smoke fails, prompt user.

---

## Phase B — MyCalendars + backend `recent_item_ids`

### Task B1: Migration `add_added_at_to_calendar_items`

**Files:**
- Create: `migrations/20260501000000_add_added_at_to_calendar_items.sql`

- [ ] **Step 1: Migration file**

```sql
ALTER TABLE calendar_items
  ADD COLUMN added_at TIMESTAMP NOT NULL DEFAULT NOW();

CREATE INDEX idx_calendar_items_calendar_added
  ON calendar_items (calendar_id, added_at DESC);
```

- [ ] **Step 2: Apply**

```
cd server && cargo sqlx migrate run
```

(Should run cleanly now that drift is resolved.)

- [ ] **Step 3: Verify**

```
psql "$DATABASE_URL" -c "\d calendar_items"
```

Expect `added_at` column.

- [ ] **Step 4: Commit**

```
git add migrations/20260501000000_add_added_at_to_calendar_items.sql
git commit -m "feat(db): add added_at to calendar_items"
```

### Task B2: Mapper change — fetch recent item ids

**Files:**
- Modify: `server/src/mappers/calendar.rs`
- Modify: `server/src/entity/calendar.rs` (or wherever `PageCalendar` lives)
- Modify: `.sqlx/` (regenerated)

- [ ] **Step 1: Add field to `PageCalendar`**

```rust
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct PageCalendar {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub subscription_token: String,
    pub item_count: i64,
    pub recent_item_ids: Vec<i32>,
}
```

- [ ] **Step 2: Update query** in `CalendarMapper::list_for_user` (or equivalent paged listing). Replace existing query body with one that uses a lateral join or subquery to fetch up to 4 most-recent `media_id` per calendar:

```rust
let rows = sqlx::query!(
    r#"
    SELECT
        c.id, c.user_id, c.name, c.subscription_token,
        COALESCE(ci_count.cnt, 0) AS "item_count!: i64",
        COALESCE(recent.ids, ARRAY[]::INTEGER[]) AS "recent_item_ids!: Vec<i32>"
    FROM calendars c
    LEFT JOIN LATERAL (
        SELECT COUNT(*) AS cnt FROM calendar_items WHERE calendar_id = c.id
    ) ci_count ON TRUE
    LEFT JOIN LATERAL (
        SELECT ARRAY_AGG(media_id ORDER BY added_at DESC) AS ids
        FROM (
            SELECT media_id, added_at
            FROM calendar_items
            WHERE calendar_id = c.id
            ORDER BY added_at DESC
            LIMIT 4
        ) recent_inner
    ) recent ON TRUE
    WHERE c.user_id = $1 AND c.deleted_at IS NULL
    ORDER BY c.id DESC
    "#,
    user_id
)
.fetch_all(&self.pool)
.await?;
```

Map each row to `PageCalendar` with the new field populated.

- [ ] **Step 3: Regenerate sqlx cache**

```
cargo sqlx prepare --workspace
```

- [ ] **Step 4: Update tests** in `mappers/calendar.rs` to assert `recent_item_ids` is populated for a calendar with items, and is an empty `Vec` for an empty calendar. Add a third assertion that ordering is most-recent-first.

- [ ] **Step 5: Run tests**

```
cd server && cargo test -p server mappers::calendar
```

- [ ] **Step 6: Commit**

```
git add server/src/mappers/calendar.rs server/src/entity/calendar.rs .sqlx/
git commit -m "feat(api): expose recent_item_ids on PageCalendar"
```

### Task B3: `PosterCollage.vue`

**Files:**
- Create: `frontend/src/components/shared/PosterCollage.vue`
- Create: `frontend/src/components/shared/__tests__/PosterCollage.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { mount } from '@vue/test-utils';
import PosterCollage from '../PosterCollage.vue';

describe('PosterCollage', () => {
  it('renders up to 4 posters in a 2x2 grid', () => {
    const wrapper = mount(PosterCollage, {
      props: {
        urls: ['a.jpg', 'b.jpg', 'c.jpg', 'd.jpg', 'e.jpg'],
      },
    });
    const imgs = wrapper.findAll('[data-testid="poster-tile"]');
    expect(imgs).toHaveLength(4);
  });

  it('fills empty slots when fewer than 4 urls', () => {
    const wrapper = mount(PosterCollage, { props: { urls: ['a.jpg'] } });
    expect(wrapper.findAll('[data-testid="poster-tile"]')).toHaveLength(1);
    expect(wrapper.findAll('[data-testid="poster-empty"]')).toHaveLength(3);
  });

  it('renders fully empty when no urls', () => {
    const wrapper = mount(PosterCollage, { props: { urls: [] } });
    expect(wrapper.findAll('[data-testid="poster-empty"]')).toHaveLength(4);
  });
});
```

- [ ] **Step 2: Implement**

```vue
<script setup lang="ts">
const props = defineProps<{ urls: string[] }>();
const slots = Array.from({ length: 4 }, (_, i) => props.urls[i] ?? null);
</script>

<template>
  <div class="grid grid-cols-2 gap-1 aspect-square w-full overflow-hidden rounded-lg bg-surface-elevated">
    <template v-for="(url, i) in slots" :key="i">
      <img
        v-if="url"
        :src="url"
        data-testid="poster-tile"
        class="object-cover w-full h-full"
        loading="lazy"
        alt=""
      />
      <div v-else data-testid="poster-empty" class="bg-surface-muted" />
    </template>
  </div>
</template>
```

- [ ] **Step 3: Run tests, expect pass.**
- [ ] **Step 4: Commit.**

```
git add frontend/src/components/shared/PosterCollage.vue frontend/src/components/shared/__tests__/PosterCollage.spec.ts
git commit -m "feat(shared): add PosterCollage component"
```

### Task B4: TS types — extend `PageCalendar`

**Files:**
- Modify: `frontend/src/services/calendars.ts` (or wherever `PageCalendar` type lives)

- [ ] **Step 1**: Add `recent_item_ids: number[]` to the type.
- [ ] **Step 2**: Verify typecheck passes: `cd frontend && npm run build`.
- [ ] **Step 3**: Commit.

```
git add frontend/src/services/calendars.ts
git commit -m "feat(types): add recent_item_ids to PageCalendar"
```

### Task B5: `CalendarTile.vue`

**Files:**
- Create: `frontend/src/components/shared/CalendarTile.vue`
- Create: `frontend/src/components/shared/__tests__/CalendarTile.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { mount, flushPromises } from '@vue/test-utils';
import CalendarTile from '../CalendarTile.vue';

vi.mock('@/services/anilist', () => ({
  getItems: vi.fn(async (ids: number[]) =>
    ids.map((id) => ({ id, coverImage: { large: `cover-${id}.jpg` } })),
  ),
}));

describe('CalendarTile', () => {
  it('renders name, item count, and owner avatar', async () => {
    const wrapper = mount(CalendarTile, {
      props: {
        calendar: {
          id: 1,
          user_id: 7,
          name: 'My Calendar',
          subscription_token: 'tok',
          item_count: 12,
          recent_item_ids: [101, 102],
        },
        ownerAvatarUrl: '/avatar.png',
      },
    });
    await flushPromises();
    expect(wrapper.find('[data-testid="calendar-tile-name"]').text()).toBe('My Calendar');
    expect(wrapper.find('[data-testid="calendar-tile-count"]').text()).toContain('12');
    expect(wrapper.find('[data-testid="calendar-tile-avatar"]').exists()).toBe(true);
  });

  it('emits click', async () => {
    const wrapper = mount(CalendarTile, {
      props: {
        calendar: {
          id: 1, user_id: 7, name: 'C', subscription_token: 't', item_count: 0, recent_item_ids: [],
        },
      },
    });
    await wrapper.find('[data-testid="calendar-tile"]').trigger('click');
    expect(wrapper.emitted('click')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Implement**

```vue
<script setup lang="ts">
import { ref, watchEffect } from 'vue';
import { useI18n } from 'vue-i18n';
import { getItems } from '@/services/anilist';
import type { PageCalendar } from '@/services/calendars';
import PosterCollage from './PosterCollage.vue';
import UiAvatar from '@/components/ui/UiAvatar.vue';

const props = defineProps<{ calendar: PageCalendar; ownerAvatarUrl?: string }>();
defineEmits<{ click: [] }>();

const { t } = useI18n();
const coverUrls = ref<string[]>([]);

watchEffect(async () => {
  const ids = props.calendar.recent_item_ids;
  if (ids.length === 0) {
    coverUrls.value = [];
    return;
  }
  try {
    const items = await getItems(ids);
    coverUrls.value = items
      .map((item) => item.coverImage?.large)
      .filter((u): u is string => Boolean(u));
  } catch {
    coverUrls.value = [];
  }
});
</script>

<template>
  <button
    data-testid="calendar-tile"
    class="flex flex-col gap-3 p-4 rounded-xl bg-surface-elevated hover:bg-surface-muted transition text-left"
    @click="$emit('click')"
  >
    <PosterCollage :urls="coverUrls" />
    <div class="flex items-center justify-between gap-2">
      <div class="flex flex-col min-w-0">
        <span data-testid="calendar-tile-name" class="font-medium truncate">{{ calendar.name }}</span>
        <span data-testid="calendar-tile-count" class="text-sm text-fg-muted">
          {{ t('calendars.tile.itemCount', { count: calendar.item_count }) }}
        </span>
      </div>
      <UiAvatar
        v-if="ownerAvatarUrl"
        data-testid="calendar-tile-avatar"
        :src="ownerAvatarUrl"
        size="sm"
      />
    </div>
  </button>
</template>
```

- [ ] **Step 3: i18n keys** in both locales:
- `calendars.tile.itemCount`: en `"{count} items"` / pt `"{count} itens"`

- [ ] **Step 4: Tests pass.**
- [ ] **Step 5: Commit.**

```
git add frontend/src/components/shared/CalendarTile.vue frontend/src/components/shared/__tests__/CalendarTile.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(shared): add CalendarTile component"
```

### Task B6: Re-skin `MyCalendarsPage.vue`

**Files:**
- Modify: `frontend/src/components/MyCalendarsPage.vue`
- Modify: spec file

- [ ] **Step 1**: Replace existing list with 3-col grid (`grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4`). Each calendar → `<CalendarTile>`. Plus one "empty tile" CTA (`<button data-testid="calendar-tile-new">` with dashed border, plus icon, label `t('calendars.create')`).
- [ ] **Step 2**: testids: `my-calendars`, `calendar-tile-new`, plus tile testids per item.
- [ ] **Step 3**: Empty zero-calendars state: `<UiEmptyState>` with create CTA in action slot.
- [ ] **Step 4**: Update tests to assert grid renders correct count, click on `calendar-tile-new` triggers create handler, click on tile navigates.
- [ ] **Step 5**: Run tests + build.
- [ ] **Step 6**: Commit.

```
git add frontend/src/components/MyCalendarsPage.vue frontend/src/components/__tests__/MyCalendarsPage.spec.ts
git commit -m "feat(calendars): re-skin MyCalendarsPage to grid of tiles"
```

### Task B7: Phase B gate

- [ ] Backend tests pass.
- [ ] Frontend tests + build + lint pass.
- [ ] Smoke (playwright MCP): log in → land on `/calendars` → grid renders → poster collages load → click tile navigates → click new tile triggers create.
- [ ] If smoke fails, prompt user.

---

## Phase C — Calendar editor + weekly schedule

### Task C1: `useDayLabels` composable

**Files:**
- Create: `frontend/src/composables/useDayLabels.ts`
- Create: `frontend/src/composables/__tests__/useDayLabels.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { describe, it, expect } from 'vitest';
import { createI18n } from 'vue-i18n';
import { withSetup } from '@/test-utils/withSetup';
import { useDayLabels } from '../useDayLabels';

describe('useDayLabels', () => {
  it('returns 7 day labels Mon-Sun in EN', () => {
    const i18n = createI18n({ legacy: false, locale: 'en', messages: {
      en: { schedule: { days: { mon: 'Mon', tue: 'Tue', wed: 'Wed', thu: 'Thu', fri: 'Fri', sat: 'Sat', sun: 'Sun' } } },
    } });
    const [{ labels }] = withSetup(() => useDayLabels(), { plugins: [i18n] });
    expect(labels.value).toEqual(['Mon','Tue','Wed','Thu','Fri','Sat','Sun']);
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

export function useDayLabels() {
  const { t } = useI18n();
  const labels = computed(() => [
    t('schedule.days.mon'),
    t('schedule.days.tue'),
    t('schedule.days.wed'),
    t('schedule.days.thu'),
    t('schedule.days.fri'),
    t('schedule.days.sat'),
    t('schedule.days.sun'),
  ]);
  return { labels };
}
```

- [ ] **Step 3: i18n keys** in both locales under `schedule.days.{mon..sun}`.
- [ ] **Step 4: Tests pass.**
- [ ] **Step 5: Commit.**

### Task C2: `useWeekRange` composable

**Files:**
- Create: `frontend/src/composables/useWeekRange.ts`
- Create: `frontend/src/composables/__tests__/useWeekRange.spec.ts`

- [ ] **Step 1: Failing tests**

```ts
import { describe, it, expect } from 'vitest';
import { useWeekRange, parseIsoWeek, formatIsoWeek } from '../useWeekRange';

describe('parseIsoWeek', () => {
  it('parses YYYY-Www to a Monday Date', () => {
    const d = parseIsoWeek('2026-W18');
    expect(d.getUTCDay()).toBe(1);
    expect(formatIsoWeek(d)).toBe('2026-W18');
  });
  it('returns null for invalid', () => {
    expect(parseIsoWeek('garbage')).toBeNull();
  });
});

describe('useWeekRange', () => {
  it('produces 7 days starting Monday', () => {
    const { days } = useWeekRange('2026-W18');
    expect(days.value).toHaveLength(7);
    expect(days.value[0].getUTCDay()).toBe(1);
    expect(days.value[6].getUTCDay()).toBe(0);
  });
});
```

- [ ] **Step 2: Implement**

```ts
import { computed, type Ref, isRef } from 'vue';

export function parseIsoWeek(s: string): Date | null {
  const m = /^(\d{4})-W(\d{2})$/.exec(s);
  if (!m) return null;
  const year = Number(m[1]);
  const week = Number(m[2]);
  // ISO week 1 contains Jan 4. Compute Monday of that week, then add (week-1)*7 days.
  const jan4 = new Date(Date.UTC(year, 0, 4));
  const jan4Day = jan4.getUTCDay() || 7;
  const week1Monday = new Date(jan4);
  week1Monday.setUTCDate(jan4.getUTCDate() - (jan4Day - 1));
  const monday = new Date(week1Monday);
  monday.setUTCDate(week1Monday.getUTCDate() + (week - 1) * 7);
  return monday;
}

export function formatIsoWeek(d: Date): string {
  const target = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
  const dayNum = target.getUTCDay() || 7;
  target.setUTCDate(target.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(target.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((+target - +yearStart) / 86400000 + 1) / 7);
  return `${target.getUTCFullYear()}-W${String(week).padStart(2, '0')}`;
}

export function useWeekRange(input: string | Ref<string>) {
  const value = computed(() => (isRef(input) ? input.value : input));
  const monday = computed(() => parseIsoWeek(value.value) ?? parseIsoWeek(formatIsoWeek(new Date()))!);
  const days = computed(() => {
    const out: Date[] = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(monday.value);
      d.setUTCDate(monday.value.getUTCDate() + i);
      out.push(d);
    }
    return out;
  });
  return { monday, days };
}
```

- [ ] **Step 3: Tests pass.**
- [ ] **Step 4: Commit.**

```
git add frontend/src/composables/useDayLabels.ts frontend/src/composables/useWeekRange.ts frontend/src/composables/__tests__/useDayLabels.spec.ts frontend/src/composables/__tests__/useWeekRange.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(schedule): add useDayLabels and useWeekRange composables"
```

### Task C3: `CalendarPage.vue` shell split + new sub-route

**Files:**
- Modify: `frontend/src/components/CalendarPage.vue`
- Modify: `frontend/src/router/index.ts`
- Create: `frontend/src/components/calendar/CalendarEditorView.vue`
- Modify: spec file

- [ ] **Step 1: Extract editor view**

Move the current contents of `CalendarPage.vue` (items list + search panel + recommendations) into a new `CalendarEditorView.vue`. Keep all logic and sub-components attached.

- [ ] **Step 2: Make `CalendarPage` a shell**

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import UiSegmented from '@/components/ui/UiSegmented.vue';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();

const tab = computed<'editor' | 'schedule'>(() =>
  route.name === 'calendar.schedule' ? 'schedule' : 'editor',
);

function setTab(next: 'editor' | 'schedule') {
  if (next === tab.value) return;
  const id = route.params.id;
  router.push(next === 'editor' ? `/calendar/${id}` : `/calendar/${id}/schedule`);
}
</script>

<template>
  <div class="flex flex-col gap-4 p-4">
    <header class="flex items-center justify-between">
      <UiSegmented
        :model-value="tab"
        :options="[
          { value: 'editor', label: t('calendar.tabs.editor') },
          { value: 'schedule', label: t('calendar.tabs.schedule') },
        ]"
        data-testid="calendar-tabs"
        @update:model-value="setTab"
      />
    </header>
    <router-view />
  </div>
</template>
```

- [ ] **Step 3: Add child routes**

In `router/index.ts`, replace existing `/calendar/:id` with:

```ts
{
  path: '/calendar/:id',
  component: () => import('@/components/CalendarPage.vue'),
  children: [
    { path: '', name: 'calendar.editor', component: () => import('@/components/calendar/CalendarEditorView.vue') },
    { path: 'schedule', name: 'calendar.schedule', component: () => import('@/components/calendar/CalendarScheduleView.vue') },
  ],
}
```

- [ ] **Step 4: i18n keys**: `calendar.tabs.editor`, `calendar.tabs.schedule` in both locales.
- [ ] **Step 5: Update existing CalendarPage tests** to assert tab segmented renders, switching pushes the right route.
- [ ] **Step 6: Tests + build pass.**
- [ ] **Step 7: Commit.**

```
git add frontend/src/components/CalendarPage.vue frontend/src/components/calendar/CalendarEditorView.vue frontend/src/router/index.ts frontend/src/components/__tests__/CalendarPage.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(calendar): split CalendarPage into shell + editor view + schedule sub-route"
```

### Task C4: `ScheduleDayColumn.vue`

**Files:**
- Create: `frontend/src/components/calendar/ScheduleDayColumn.vue`
- Create: `frontend/src/components/calendar/__tests__/ScheduleDayColumn.spec.ts`

- [ ] **Step 1: Failing test**

```ts
import { mount } from '@vue/test-utils';
import ScheduleDayColumn from '../ScheduleDayColumn.vue';

describe('ScheduleDayColumn', () => {
  it('renders day label, date, and entries', () => {
    const wrapper = mount(ScheduleDayColumn, {
      props: {
        label: 'Mon',
        date: new Date(Date.UTC(2026, 4, 4)),
        entries: [
          { id: 1, title: 'Show A', episode: 5, time: '20:00', coverUrl: 'a.jpg' },
        ],
      },
    });
    expect(wrapper.find('[data-testid="day-label"]').text()).toBe('Mon');
    expect(wrapper.find('[data-testid="day-date"]').text()).toContain('4');
    expect(wrapper.findAll('[data-testid="schedule-entry"]')).toHaveLength(1);
  });

  it('shows empty state when no entries', () => {
    const wrapper = mount(ScheduleDayColumn, {
      props: { label: 'Tue', date: new Date(), entries: [] },
    });
    expect(wrapper.find('[data-testid="day-empty"]').exists()).toBe(true);
  });
});
```

- [ ] **Step 2: Implement**

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';

interface Entry { id: number; title: string; episode?: number; time?: string; coverUrl?: string }

defineProps<{ label: string; date: Date; entries: Entry[] }>();
const { t } = useI18n();
</script>

<template>
  <div class="flex flex-col gap-2 min-w-0">
    <div class="flex flex-col gap-0.5 px-2 py-1">
      <span data-testid="day-label" class="text-xs uppercase tracking-wide text-fg-muted">{{ label }}</span>
      <span data-testid="day-date" class="text-lg font-medium">{{ date.getUTCDate() }}</span>
    </div>
    <div class="flex flex-col gap-2">
      <div
        v-for="entry in entries"
        :key="entry.id"
        data-testid="schedule-entry"
        class="flex gap-2 p-2 rounded-md bg-surface-elevated"
      >
        <img v-if="entry.coverUrl" :src="entry.coverUrl" class="w-10 h-14 rounded object-cover" alt="" />
        <div class="flex flex-col min-w-0">
          <span class="text-sm font-medium truncate">{{ entry.title }}</span>
          <span v-if="entry.episode" class="text-xs text-fg-muted">{{ t('schedule.episode', { n: entry.episode }) }}</span>
          <span v-if="entry.time" class="text-xs text-fg-muted">{{ entry.time }}</span>
        </div>
      </div>
      <div v-if="entries.length === 0" data-testid="day-empty" class="text-xs text-fg-muted px-2 py-4 text-center">
        {{ t('schedule.noEntries') }}
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: i18n**: `schedule.episode` (`"Ep. {n}"`), `schedule.noEntries`.
- [ ] **Step 4: Tests pass. Commit.**

```
git add frontend/src/components/calendar/ScheduleDayColumn.vue frontend/src/components/calendar/__tests__/ScheduleDayColumn.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(schedule): add ScheduleDayColumn"
```

### Task C5: `CalendarScheduleView.vue`

**Files:**
- Create: `frontend/src/components/calendar/CalendarScheduleView.vue`
- Create: `frontend/src/components/calendar/__tests__/CalendarScheduleView.spec.ts`

- [ ] **Step 1: Implement**

```vue
<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { useDayLabels } from '@/composables/useDayLabels';
import { useWeekRange, formatIsoWeek } from '@/composables/useWeekRange';
import { fetchScheduleForCalendar } from '@/services/calendars';
import UiButton from '@/components/ui/UiButton.vue';
import ScheduleDayColumn from './ScheduleDayColumn.vue';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const { labels } = useDayLabels();

const week = computed(() => (route.query.week as string) || formatIsoWeek(new Date()));
const { days } = useWeekRange(week);

const entriesByDay = ref<Record<string, Array<{ id: number; title: string; episode?: number; time?: string; coverUrl?: string }>>>({});

watchEffect(async () => {
  const calendarId = Number(route.params.id);
  const data = await fetchScheduleForCalendar(calendarId, week.value);
  entriesByDay.value = data;
});

function shiftWeek(delta: -1 | 1) {
  const d = days.value[0];
  d.setUTCDate(d.getUTCDate() + delta * 7);
  router.replace({ query: { ...route.query, week: formatIsoWeek(d) } });
}

function isoDay(d: Date) {
  return d.toISOString().slice(0, 10);
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between">
      <UiButton variant="ghost" size="sm" data-testid="schedule-prev" @click="shiftWeek(-1)">
        {{ t('schedule.prev') }}
      </UiButton>
      <span data-testid="schedule-week" class="font-medium">{{ week }}</span>
      <UiButton variant="ghost" size="sm" data-testid="schedule-next" @click="shiftWeek(1)">
        {{ t('schedule.next') }}
      </UiButton>
    </div>
    <div class="grid grid-cols-1 sm:grid-cols-7 gap-3">
      <ScheduleDayColumn
        v-for="(d, i) in days"
        :key="isoDay(d)"
        :label="labels[i]"
        :date="d"
        :entries="entriesByDay[isoDay(d)] ?? []"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Service stub**

In `frontend/src/services/calendars.ts`, add:

```ts
export async function fetchScheduleForCalendar(
  calendarId: number,
  isoWeek: string,
): Promise<Record<string, Array<{ id: number; title: string; episode?: number; time?: string; coverUrl?: string }>>> {
  const { data } = await api.get(`/calendars/${calendarId}/schedule`, { params: { week: isoWeek } });
  return data;
}
```

If a backend schedule endpoint does not exist, derive from `calendar.items` client-side (group by airing date) and document the limitation in the spec; do NOT add a backend endpoint in Track 2 (out of scope).

- [ ] **Step 3: Test**: mount with mocked fetch returning entries on two days; assert 7 columns render, two have entries, others empty.
- [ ] **Step 4: i18n**: `schedule.prev`, `schedule.next`.
- [ ] **Step 5: Tests + build pass.**
- [ ] **Step 6: Commit.**

```
git add frontend/src/components/calendar/CalendarScheduleView.vue frontend/src/components/calendar/__tests__/CalendarScheduleView.spec.ts frontend/src/services/calendars.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(schedule): add CalendarScheduleView weekly grid"
```

### Task C6: Re-skin editor sub-components + adopt `UiBannerFade`

**Files:**
- Modify: `frontend/src/components/calendar/*` (every sub-component used by editor view)
- Modify: `frontend/src/components/shared/MediaItemCard.vue`

- [ ] **Step 1**: For each sub-component (items list, search panel, search row, recommendations strip): swap DaisyUI classes (`btn`, `input`, `card`, `badge`) for Track 1 primitives (`UiButton`, `UiInput`, `UiChip`). Keep all event handlers and emit signatures unchanged.
- [ ] **Step 2**: `MediaItemCard.vue` — wrap the existing banner element in `<UiBannerFade :active="selected">` (Track 1 primitive). Remove the local mask CSS. The mask string contract lives only inside `UiBannerFade` per spec.
- [ ] **Step 3**: Update editor layout to 60/40 split (`grid-cols-[3fr_2fr]` desktop, stacked mobile).
- [ ] **Step 4**: testids: `editor-items-list`, `editor-search-panel`, `editor-recommendations`.
- [ ] **Step 5**: Existing tests should mostly continue to pass — replace any DaisyUI class assertions with testid queries.
- [ ] **Step 6**: Run tests + build.
- [ ] **Step 7**: Commit.

```
git add frontend/src/components/calendar/ frontend/src/components/shared/MediaItemCard.vue
git commit -m "feat(calendar): re-skin editor sub-components and adopt UiBannerFade"
```

### Task C7: Phase C gate

- [ ] All tests pass.
- [ ] Build + lint pass.
- [ ] Smoke (playwright MCP): open calendar editor → tab to schedule → navigate weeks → tab back to editor → search → select item → banner fade-in fires.
- [ ] If smoke fails, prompt user.

---

## Phase D — Account restructure

### Task D1: `PRO_ACCENTS` constant + `AccentPicker.vue`

**Files:**
- Create: `frontend/src/constants/proAccents.ts`
- Create: `frontend/src/components/account/AccentPicker.vue`
- Create: `frontend/src/components/account/__tests__/AccentPicker.spec.ts`

- [ ] **Step 1: Constant**

```ts
import type { Accent } from '@/composables/useTheme';

export const PRO_ACCENTS: ReadonlySet<Accent> = new Set(['matcha', 'sakura', 'citron']);
```

- [ ] **Step 2: Test**

```ts
import { mount } from '@vue/test-utils';
import AccentPicker from '../AccentPicker.vue';

describe('AccentPicker', () => {
  it('renders all 5 accent swatches', () => {
    const wrapper = mount(AccentPicker, { props: { modelValue: 'coral' } });
    expect(wrapper.findAll('[data-testid^="accent-swatch-"]')).toHaveLength(5);
  });

  it('shows Pro chip on pro accents', () => {
    const wrapper = mount(AccentPicker, { props: { modelValue: 'coral' } });
    expect(wrapper.find('[data-testid="accent-pro-matcha"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="accent-pro-sakura"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="accent-pro-citron"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="accent-pro-coral"]').exists()).toBe(false);
  });

  it('emits update:modelValue on click', async () => {
    const wrapper = mount(AccentPicker, { props: { modelValue: 'coral' } });
    await wrapper.find('[data-testid="accent-swatch-iris"]').trigger('click');
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['iris']);
  });
});
```

- [ ] **Step 3: Implement**

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Accent } from '@/composables/useTheme';
import { PRO_ACCENTS } from '@/constants/proAccents';
import UiChip from '@/components/ui/UiChip.vue';

const ACCENTS: Accent[] = ['coral', 'iris', 'matcha', 'sakura', 'citron'];

defineProps<{ modelValue: Accent }>();
defineEmits<{ 'update:modelValue': [Accent] }>();

const { t } = useI18n();
</script>

<template>
  <div class="flex flex-wrap gap-3">
    <button
      v-for="accent in ACCENTS"
      :key="accent"
      :data-testid="`accent-swatch-${accent}`"
      :data-accent-preview="accent"
      class="relative flex flex-col items-center gap-2 p-3 rounded-lg bg-surface-elevated hover:bg-surface-muted transition"
      :aria-pressed="modelValue === accent"
      @click="$emit('update:modelValue', accent)"
    >
      <span
        class="w-10 h-10 rounded-full"
        :style="{ backgroundColor: `var(--accent-${accent})` }"
      />
      <span class="text-sm font-medium">{{ t(`account.preferences.accents.${accent}`) }}</span>
      <UiChip
        v-if="PRO_ACCENTS.has(accent)"
        :data-testid="`accent-pro-${accent}`"
        variant="info"
        size="sm"
      >
        {{ t('account.preferences.proChip') }}
      </UiChip>
      <span
        v-if="modelValue === accent"
        class="absolute inset-0 rounded-lg ring-2 ring-accent pointer-events-none"
      />
    </button>
  </div>
</template>
```

- [ ] **Step 4: i18n** in both locales:
- `account.preferences.accents.{coral,iris,matcha,sakura,citron}`
- `account.preferences.proChip`: en `"Pro"` / pt `"Pro"`
- `account.preferences.proNote`: en `"Free during early access"` / pt `"Gratuito durante o acesso antecipado"`

- [ ] **Step 5: Note**: `PRO_ACCENTS` is informational only in Track 2. No gating logic. Track 4 will introduce entitlement.

- [ ] **Step 6: Tests pass. Commit.**

```
git add frontend/src/constants/proAccents.ts frontend/src/components/account/AccentPicker.vue frontend/src/components/account/__tests__/AccentPicker.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(account): add AccentPicker with informational Pro chips"
```

### Task D2: `AccountPage.vue` shell + 4 child routes

**Files:**
- Create: `frontend/src/components/AccountPage.vue`
- Create: `frontend/src/components/account/ProfileTab.vue`
- Create: `frontend/src/components/account/PreferencesTab.vue`
- Create: `frontend/src/components/account/PasswordTab.vue`
- Create: `frontend/src/components/account/DangerZoneTab.vue`
- Modify: `frontend/src/router/index.ts`

- [ ] **Step 1: Shell**

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();

const tabs = computed(() => [
  { name: 'account.profile', label: t('account.tabs.profile'), testid: 'account-tab-profile' },
  { name: 'account.preferences', label: t('account.tabs.preferences'), testid: 'account-tab-preferences' },
  { name: 'account.password', label: t('account.tabs.password'), testid: 'account-tab-password' },
  { name: 'account.danger', label: t('account.tabs.danger'), testid: 'account-tab-danger' },
]);
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-[200px_1fr] gap-6 p-4">
    <nav class="flex md:flex-col gap-1" data-testid="account-sidebar">
      <button
        v-for="tab in tabs"
        :key="tab.name"
        :data-testid="tab.testid"
        class="text-left px-3 py-2 rounded-md hover:bg-surface-muted transition"
        :class="route.name === tab.name ? 'bg-surface-elevated font-medium' : ''"
        @click="router.push({ name: tab.name })"
      >
        {{ tab.label }}
      </button>
    </nav>
    <main>
      <router-view />
    </main>
  </div>
</template>
```

- [ ] **Step 2: ProfileTab** — port content from existing `UserDetailsPage.vue` (avatar, name, email, save button) using `UiInput` + `UiButton`.
- [ ] **Step 3: PreferencesTab** — language switcher, theme switcher, `<AccentPicker v-model="accent" />`, plus a note paragraph rendering `t('account.preferences.proNote')`. On change, persist via `userSettingsStore.updateSettings` and apply via `useTheme().applyAccent(accent)`.
- [ ] **Step 4: PasswordTab** — port from existing settings page.
- [ ] **Step 5: DangerZoneTab** — port delete-account flow; wrap action in `<UiButton variant="danger">` and confirm via existing modal pattern.
- [ ] **Step 6: Routes**

```ts
{
  path: '/account',
  component: () => import('@/components/AccountPage.vue'),
  meta: { requiresAuth: true },
  children: [
    { path: '', redirect: { name: 'account.profile' } },
    { path: 'profile', name: 'account.profile', component: () => import('@/components/account/ProfileTab.vue') },
    { path: 'preferences', name: 'account.preferences', component: () => import('@/components/account/PreferencesTab.vue') },
    { path: 'password', name: 'account.password', component: () => import('@/components/account/PasswordTab.vue') },
    { path: 'danger', name: 'account.danger', component: () => import('@/components/account/DangerZoneTab.vue') },
  ],
}
```

- [ ] **Step 7: i18n**: `account.tabs.{profile,preferences,password,danger}` plus port any existing keys from old page namespaces (`userDetails.*`, `userSettings.*`) into `account.*` and update consumers.
- [ ] **Step 8: Update header nav** wherever the account link points — should target `/account` (which redirects to `/account/profile`).
- [ ] **Step 9: Test**: mount shell with each child route active, assert sidebar highlight tracks the route; click each tab, assert push.
- [ ] **Step 10: Tests + build pass. Commit.**

```
git add frontend/src/components/AccountPage.vue frontend/src/components/account/ frontend/src/router/index.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(account): restructure into shell + 4 tab routes"
```

### Task D3: Delete legacy account pages

**Files:**
- Delete: `frontend/src/components/UserDetailsPage.vue`
- Delete: `frontend/src/components/UserSettingsPage.vue`
- Delete: their spec files
- Modify: any remaining references / lazy-route imports

- [ ] **Step 1**: `git grep` for `UserDetailsPage` and `UserSettingsPage`. Delete files; remove orphan imports + routes.
- [ ] **Step 2**: Build + tests pass.
- [ ] **Step 3**: Commit.

```
git add -A
git commit -m "chore(account): remove legacy UserDetails/UserSettings pages"
```

### Task D4: Phase D gate

- [ ] Tests + build + lint pass.
- [ ] Smoke (playwright MCP): visit `/account` → redirects to profile → all 4 tabs navigate → preferences picker writes setting and applies live → Pro chip visible on matcha/sakura/citron.
- [ ] If smoke fails, prompt user.

---

## Phase E — Cleanup, DaisyUI removal, audits

### Task E1: Migrate any remaining `Toast`/`Alert` consumers to `UiToast`

**Files:**
- Modify: any consumer using DaisyUI `alert` or `toast` classes

- [ ] **Step 1**: `git grep -E '(alert-|toast-)'` scoped to `frontend/src`. For each match, swap to Track 1 `UiToast` API.
- [ ] **Step 2**: Tests + build pass.
- [ ] **Step 3**: Commit.

### Task E2: Migrate `Modal` consumers to `UiModal`

- [ ] **Step 1**: `git grep -E 'modal(-|\b)'` in `frontend/src`. For each, swap to `UiModal`.
- [ ] **Step 2**: Tests + build pass.
- [ ] **Step 3**: Commit.

### Task E3: DaisyUI usage audit

**Files:**
- Read-only audit, report only

- [ ] **Step 1**: Run a comprehensive grep for DaisyUI-specific class prefixes:

```
grep -rn -E '\b(btn|input|card|badge|alert|toast|modal|drawer|menu|navbar|tabs|select|checkbox|radio|toggle|range|progress|stat|chat|join|swap)(-[a-z]+)?\b' frontend/src
```

- [ ] **Step 2**: Audit each match: is it Tailwind utility (e.g., `tabs` happens to be a Tailwind class) or DaisyUI? List true DaisyUI consumers.
- [ ] **Step 3**: For each true DaisyUI consumer, swap to Track 1 primitive or raw Tailwind utility classes. One commit per file or grouped by component family.
- [ ] **Step 4**: Tests + build pass after each commit.

### Task E4: Remove DaisyUI

**Files:**
- Modify: `frontend/package.json`
- Modify: `frontend/src/assets/main.css` (or wherever DaisyUI is loaded)
- Modify: `frontend/tailwind.config.cjs` if applicable

- [ ] **Step 1**: Verify zero remaining DaisyUI consumers (re-run E3 grep, expect empty).
- [ ] **Step 2**: Remove `@plugin "daisyui"` (or `daisyui` from Tailwind config) and the `daisyui` npm dep.
- [ ] **Step 3**: Run `npm install` to update lockfile.
- [ ] **Step 4**: Build + tests + lint pass.
- [ ] **Step 5**: Commit.

```
git add frontend/package.json frontend/package-lock.json frontend/src/assets/main.css frontend/tailwind.config.cjs
git commit -m "chore(deps): remove DaisyUI"
```

### Task E5: i18n compliance sweep

- [ ] **Step 1**: Invoke the `vue-i18n-auditor` agent on `frontend/src/components`.
- [ ] **Step 2**: For each finding, wrap with `t()` and add the key to both locale files.
- [ ] **Step 3**: Tests + build pass.
- [ ] **Step 4**: Commit per logical group.

### Task E6: Final smoke + Phase E gate

- [ ] Full frontend test suite passes.
- [ ] Backend test suite passes.
- [ ] Build + lint clean.
- [ ] Manual smoke (playwright MCP) end-to-end: register → verify email → log in → land on calendars → create calendar → add items in editor → switch to schedule tab → navigate weeks → open account → change accent → log out → 404 → forgot password.
- [ ] Inspect production CSS: confirm bundle no longer ships DaisyUI rules (size delta noted in PR description).
- [ ] If smoke fails, prompt user.

---

## Self-review checklist (run before opening PR)

- [ ] Every spec section maps to at least one task.
- [ ] No `TODO`/`TBD`/placeholder text.
- [ ] Type names match between tasks (`Accent`, `PageCalendar`, etc.).
- [ ] Every new string has both `en.json` and `pt.json` entries.
- [ ] `MediaItemCard` no longer carries the mask string locally — it lives only in `UiBannerFade`.
- [ ] DaisyUI grep returns zero matches in `frontend/src`.
- [ ] Migration drift resolved — `cargo sqlx migrate info` reports clean.

---

## Post-Plan Follow-Ups

Tracked here so they survive the plan close-out.

### FU-1: Airing-this-season chip + stat on MyCalendarsPage

**Why:** Desktop design (`design_handoff_anime_calendar/screens-auth-list.jsx:137`) shows a "N airing" chip on each tile and a "K airing this season" entry in the page stats row. Backend currently does not expose per-calendar airing counts, so the chip + stat were dropped from B5/B6 and need to be added later.

**Sketch:** Either extend `PageCalendar` with an `airing_count` field computed by joining against the existing `airing_schedule` cached data, or compute client-side after `recent_item_ids` resolution. Backend-side is preferred.

### FU-4: Account tabs polish (post-D2)

**Why:** D2 reviewer flagged several non-blocking quality items deferred from the AccountPage commit.

**Sketch:**
- **`useUserDetails` composable**: ProfileTab and PasswordTab both fetch `GET /user/details` independently on mount. Hoist into a small composable with in-flight Promise dedup (matches the project convention from CLAUDE.md). Avatar+name read from localStorage in ProfileTab also belongs there.
- **Drop dead `router.push('/login')` in `onMounted`** (ProfileTab + PasswordTab). The router guard already enforces `requiresAuth: true` on `/account`. PreferencesTab does this correctly (early `return` only).
- **Mobile sidebar overflow**: AccountPage uses `flex md:flex-col` so mobile is `flex-row`. With 4 tabs including "Danger Zone" / "Zona Perigosa", a 320px viewport overflows. Either add `flex-wrap` or convert to a horizontal scrollable strip with `overflow-x-auto`. Coordinate with Track 3 mobile companion.
- **Test additions to `AccountPage.spec.ts`**: assert sidebar contains exactly 4 buttons; assert tab order matches `[profile, preferences, password, danger]`; add a test that `/account` redirects to `/account/profile`.
- **Accent persistence semantics**: `setAccent()` fires immediately on swatch change for instant preview, but `userSettingsStore.updateSettings` is only called on Save. If user navigates away without saving, server-wins reconcile reverts on next load. Document this UX as intentional (preview-on-change, commit-on-save) or auto-save on swatch change with a request abort token.

### FU-3: UiButton success/warning variants + UiRadio primitive

**Why:** C6 reskin had to drop `btn-success` (green submit on settings form) and `btn-warning` (yellow Clear on items list) because `UiButton` only ships `primary | secondary | ghost | danger`. Both call sites currently use `primary` (settings submit) and `secondary` (items clear), losing the original semantic colour. Same trip surfaced that radio inputs are duplicated 6 times across `CalendarSettingsForm` and `ItemSearchPanel` with hand-rolled `accent-accent-1` styling.

**Sketch:**
- Add `success` and `warning` variants to `UiButton`'s `tv()` map using `bg-success`/`bg-warning` tokens (already defined in `tokens.css`). Re-thread `CalendarSettingsForm` submit and `CalendarItemsList` clear.
- Extract `UiRadio.vue` with a `{ modelValue, value, label }` prop surface. Replace the 6 hand-rolled radios in `CalendarSettingsForm` (3 language options) and `ItemSearchPanel` (3 media-type options).

### FU-2: Schedule view polish (post-C5)

**Why:** C5 reviewer flagged several UX/quality items deferred as non-blocking:

- **User timezone**: schedule entry times are emitted in UTC (`HH:MM`). Should respect `userSettingsStore.timezone` so users see local airing times. Pull TZ from the store at the view layer (or pass it into `fetchScheduleForCalendar` as a parameter — keep the service pure).
- **Localized week label**: the header currently shows the raw ISO week string `2026-W18`. Render via `Intl.DateTimeFormat` as e.g. "Apr 27 – May 3, 2026" using the existing locale. i18n key `schedule.weekLabel` with interpolated `{start}` / `{end}`.
- **Service unit tests**: `frontend/src/services/calendars.ts` derivation logic is currently only covered indirectly via the view test (which mocks the service entirely). Add `frontend/src/services/__tests__/calendars.spec.ts` covering: Sunday 23:59 inclusion, next-Monday 00:00 exclusion, language fallback when `calendar.language` undefined, sort by airing time, episodes outside the week excluded, empty `airing_schedule` array.
- **Entry id as string key**: `id: item.id * 10000 + entry.episode` collides for shows with >9999 episodes (Sazae-san already exceeds 8000). Switch to `string`-typed key `${item.id}:${entry.episode}` since it's only used as a Vue `:key`. Requires updating `ScheduleEntry.id: number` → `string` in `frontend/src/types/schedule.ts`.
- **Mobile layout**: `grid-cols-1 sm:grid-cols-7` collapses to 1 column on mobile (good) and 7 cramped columns at `sm` (640px). Either bump the 7-col breakpoint to `md` (768px) or add a horizontal-scroll variant to keep day columns readable. Coordinate with Track 3 (mobile companion).

