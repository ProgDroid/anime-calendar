# Vue Frontend Component Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose monolithic Vue page components into focused, reusable sub-components, fix template and reactivity bugs, and standardize patterns — preserving all existing functionality and visuals exactly.

**Architecture:** Extract repeated UI patterns (media item card, confirm modal, pagination) into standalone components; split `CalendarPage.vue`'s duplicated mobile/desktop logic into a single responsive layout driven by CSS/Tailwind; standardize i18n usage to `useI18n()` composable throughout.

**Tech Stack:** Vue 3.5 Composition API (`<script setup>`), TypeScript, Tailwind CSS v4, DaisyUI v5, vue-i18n 11, Vitest + @vue/test-utils

---

## Current problems inventory

| File | Problem |
|------|---------|
| `CalendarPage.vue` | ~900 lines; entire template duplicated for mobile/desktop |
| `CalendarPage.vue` | `MediaItemCard` template repeated 4+ times (search, cal-items, reco mobile, reco desktop) |
| `CalendarPage.vue` | `isMobile = computed(() => window.innerWidth < 768)` — not reactive, won't update on window resize |
| `CalendarPage.vue` | `placeholder="{{ $t(...) }}"` on lines 171/332 — Vue binding syntax bug; literal string passed |
| `CalendarPage.vue` | `sessionStorage` state + `calculateRecommendations` logic lives inside template component |
| `MyCalendarsPage.vue` | `confirm('Are you sure…')` — native browser dialog, hardcoded English |
| `UserDetailsPage.vue` | `confirm()` for delete; `disabled=true` (should be `:disabled="true"`) |
| `UserSettingsPage.vue` | `console.log('Fetched settings:', …)` left in production code |
| All components | Mix of `i18n.global` direct access and `useI18n()` composable — inconsistent |
| All components | No reusable confirm/modal dialog; each page rolls its own destructive-action UX |

---

## File structure

### New files to create

```
frontend/src/components/
  shared/
    MediaItemCard.vue          — Cover image + title + badge + episodes; used in search, cal-items, recommendations
    ConfirmModal.vue           — DaisyUI modal for destructive action confirmations (replaces native confirm())
    PaginationControls.vue     — Reusable paginated nav (extracted from MyCalendarsPage)
  calendar/
    ItemSearchPanel.vue        — Search form + fetched items list (left/top panel of CalendarPage)
    CalendarItemsList.vue      — Items-in-calendar list + clear button (right/bottom panel)
    CalendarSettingsForm.vue   — Name input, language radio group, submit button
    RecommendationsSection.vue — Recommendations carousel (desktop only)
  composables/
    useWindowSize.ts           — Reactive window size composable (replaces non-reactive isMobile)
```

### Files to modify

```
frontend/src/components/CalendarPage.vue      — Gut template; import sub-components; fix isMobile; fix placeholder bug
frontend/src/components/MyCalendarsPage.vue   — Replace confirm() with ConfirmModal; use PaginationControls
frontend/src/components/UserDetailsPage.vue   — Replace confirm(); fix disabled=true; standardize i18n
frontend/src/components/UserSettingsPage.vue  — Remove console.log; standardize i18n
frontend/src/components/LoginPage.vue         — Standardize i18n (minor)
frontend/src/App.vue                          — Standardize i18n (minor)
frontend/src/__tests__/CalendarPage.spec.ts   — Update stub imports for sub-components
```

---

## Task 1: `useWindowSize` composable

**Files:**
- Create: `frontend/src/composables/useWindowSize.ts`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/useWindowSize.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { nextTick } from 'vue'
import { useWindowSize } from '@/composables/useWindowSize'

describe('useWindowSize', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 1024 })
    Object.defineProperty(window, 'innerHeight', { writable: true, configurable: true, value: 768 })
  })

  it('returns current window dimensions', () => {
    const { width, height } = useWindowSize()
    expect(width.value).toBe(1024)
    expect(height.value).toBe(768)
  })

  it('isMobile is false when width >= 768', () => {
    const { isMobile } = useWindowSize()
    expect(isMobile.value).toBe(false)
  })

  it('isMobile is true when width < 768', () => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 375 })
    const { isMobile } = useWindowSize()
    expect(isMobile.value).toBe(true)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

```
cd frontend && npx vitest run src/__tests__/useWindowSize.spec.ts
```
Expected: FAIL — `@/composables/useWindowSize` not found

- [ ] **Step 3: Implement the composable**

Create `frontend/src/composables/useWindowSize.ts`:

```typescript
import { ref, onMounted, onUnmounted, computed } from 'vue'

export function useWindowSize() {
  const width = ref(window.innerWidth)
  const height = ref(window.innerHeight)

  const onResize = () => {
    width.value = window.innerWidth
    height.value = window.innerHeight
  }

  onMounted(() => window.addEventListener('resize', onResize))
  onUnmounted(() => window.removeEventListener('resize', onResize))

  const isMobile = computed(() => width.value < 768)

  return { width, height, isMobile }
}
```

- [ ] **Step 4: Run test to verify it passes**

```
cd frontend && npx vitest run src/__tests__/useWindowSize.spec.ts
```
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/composables/useWindowSize.ts frontend/src/__tests__/useWindowSize.spec.ts
git commit -m "feat: add reactive useWindowSize composable"
```

---

## Task 2: `MediaItemCard` shared component

**Files:**
- Create: `frontend/src/components/shared/MediaItemCard.vue`

This replaces the cover-image + title + badge block that appears identically in 4 places inside `CalendarPage.vue`.

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/MediaItemCard.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import type { Item } from '@/types/item'

const item: Item = {
  id: 1,
  id_mal: null,
  title: { english: 'Attack on Titan', native: '進撃の巨人', romaji: 'Shingeki no Kyojin' },
  media_type: 'ANIME',
  episode_duration: 24,
  airing_schedule: [],
  cover_image: { extraLarge: '', large: '', medium: 'http://img.test/cover.jpg', color: '' },
  banner_image: '',
  recommendations: []
}

describe('MediaItemCard', () => {
  it('renders the title', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false }
    })
    expect(wrapper.text()).toContain('Attack on Titan')
  })

  it('renders cover image when available', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false }
    })
    expect(wrapper.find('img').attributes('src')).toBe('http://img.test/cover.jpg')
  })

  it('shows media type badge', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false }
    })
    expect(wrapper.text()).toContain('ANIME')
  })

  it('applies border-primary class when selected', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: true, isInCalendar: false }
    })
    expect(wrapper.classes()).toContain('border-primary')
  })

  it('applies border-success class when in calendar', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: true }
    })
    expect(wrapper.classes()).toContain('border-success')
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/MediaItemCard.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/shared/MediaItemCard.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'

const { t } = useI18n()

const props = defineProps<{
  item: Item
  displayTitle: string
  isSelected: boolean
  isInCalendar: boolean
  compact?: boolean
}>()

const emit = defineEmits<{
  click: []
}>()

const onImageError = (e: Event) => {
  const img = e.target as HTMLImageElement
  img.style.display = 'none'
}
</script>

<template>
  <div
    class="card bg-base-100 shadow-sm border relative overflow-hidden cursor-pointer"
    :class="{
      'border-primary': isSelected && !isInCalendar,
      'border-success': isInCalendar
    }"
    @click="emit('click')"
  >
    <div class="card-body p-3">
      <div class="flex items-start gap-2">
        <div class="flex-shrink-0">
          <div
            v-if="item.cover_image?.medium"
            class="bg-base-300 border rounded w-16 h-20 overflow-hidden"
          >
            <img
              :src="item.cover_image.medium"
              :alt="item.title.romaji"
              class="w-full h-full object-cover"
              @error="onImageError"
            />
          </div>
          <div
            v-else
            class="bg-base-300 border rounded w-16 h-20 flex items-center justify-center"
          >
            <span class="text-xs">{{ t('calendar.noImage') }}</span>
          </div>
        </div>
        <div class="flex-grow min-w-0">
          <h4 class="font-bold line-clamp-1" :class="compact ? 'text-sm' : ''">
            {{ displayTitle }}
          </h4>
          <div class="badge badge-secondary mt-1" :class="compact ? 'text-xs' : ''">
            {{ item.media_type }}
          </div>
          <p v-if="item.media_type === 'ANIME' && !compact" class="text-xs mt-1">
            {{ t('calendar.episodes') }}: {{ item.episode_duration }}
          </p>
        </div>
      </div>

      <!-- "Already in calendar" badge -->
      <div
        v-if="isInCalendar"
        class="absolute top-2 right-2 bg-success text-success-content text-xs px-2 py-1 rounded"
      >
        {{ t('calendar.alreadyInCalendar') }}
      </div>

      <!-- Background image overlay when selected -->
      <div
        v-if="isSelected && item.banner_image"
        class="absolute inset-0 pointer-events-none transition-opacity duration-300"
        :style="{
          backgroundImage: `url(${item.banner_image})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          backgroundRepeat: 'no-repeat',
          maskImage: 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0) 50%, rgba(0,0,0,0.2) 65%, rgba(0,0,0,1) 95%)',
          borderRadius: '8px'
        }"
      />
      <div
        v-else-if="isSelected && item.cover_image?.medium"
        class="absolute inset-0 pointer-events-none transition-opacity duration-300"
        :style="{
          backgroundImage: `url(${item.cover_image.medium})`,
          backgroundSize: 'auto 100%',
          backgroundPosition: 'right',
          backgroundRepeat: 'no-repeat',
          maskImage: 'linear-gradient(to right, rgba(0,0,0,0) 0%, rgba(0,0,0,0.0) 83.5%, rgba(0,0,0,1) 95%)',
          borderRadius: '8px'
        }"
      />

      <slot />
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/MediaItemCard.spec.ts
```
Expected: PASS (5 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/shared/MediaItemCard.vue frontend/src/__tests__/MediaItemCard.spec.ts
git commit -m "feat: add MediaItemCard shared component"
```

---

## Task 3: `ConfirmModal` component

This replaces native `confirm()` calls in `MyCalendarsPage`, `UserDetailsPage`, and `CalendarPage`.

**Files:**
- Create: `frontend/src/components/shared/ConfirmModal.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/ConfirmModal.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'

describe('ConfirmModal', () => {
  it('renders title and message', () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'This cannot be undone.', open: true }
    })
    expect(wrapper.text()).toContain('Delete?')
    expect(wrapper.text()).toContain('This cannot be undone.')
  })

  it('emits confirm when confirm button clicked', async () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: true }
    })
    await wrapper.find('[data-testid="confirm-btn"]').trigger('click')
    expect(wrapper.emitted('confirm')).toBeTruthy()
  })

  it('emits cancel when cancel button clicked', async () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: true }
    })
    await wrapper.find('[data-testid="cancel-btn"]').trigger('click')
    expect(wrapper.emitted('cancel')).toBeTruthy()
  })

  it('is hidden when open is false', () => {
    const wrapper = mount(ConfirmModal, {
      props: { title: 'Delete?', message: 'Sure?', open: false }
    })
    expect(wrapper.find('[data-testid="modal-box"]').exists()).toBe(false)
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/ConfirmModal.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/shared/ConfirmModal.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  title: string
  message: string
  open: boolean
  confirmLabel?: string
  cancelLabel?: string
  danger?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <dialog v-if="open" class="modal modal-open">
    <div class="modal-box" data-testid="modal-box">
      <h3 class="font-bold text-lg">{{ title }}</h3>
      <p class="py-4">{{ message }}</p>
      <div class="modal-action">
        <button
          data-testid="cancel-btn"
          class="btn btn-ghost"
          @click="emit('cancel')"
        >
          {{ cancelLabel ?? t('app.cancel') }}
        </button>
        <button
          data-testid="confirm-btn"
          :class="['btn', danger ? 'btn-error' : 'btn-primary']"
          @click="emit('confirm')"
        >
          {{ confirmLabel ?? t('app.confirm') }}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" @click="emit('cancel')" />
  </dialog>
</template>
```

- [ ] **Step 4: Add i18n keys for `app.cancel` and `app.confirm`**

Open `frontend/src/locales/en.json` and add under `"app"`:
```json
"cancel": "Cancel",
"confirm": "Confirm"
```

Open `frontend/src/locales/pt.json` and add under `"app"`:
```json
"cancel": "Cancelar",
"confirm": "Confirmar"
```

- [ ] **Step 5: Run tests**

```
cd frontend && npx vitest run src/__tests__/ConfirmModal.spec.ts
```
Expected: PASS (4 tests)

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/shared/ConfirmModal.vue frontend/src/__tests__/ConfirmModal.spec.ts frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat: add ConfirmModal shared component"
```

---

## Task 4: `PaginationControls` component

Extracted from `MyCalendarsPage.vue` — the join/btn pagination block.

**Files:**
- Create: `frontend/src/components/shared/PaginationControls.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/PaginationControls.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import PaginationControls from '@/components/shared/PaginationControls.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('PaginationControls', () => {
  const mountOpts = { global: { plugins: [i18n] } }

  it('does not render when total_pages <= 1', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 3, total_pages: 1 },
      ...mountOpts
    })
    expect(wrapper.find('.join').exists()).toBe(false)
  })

  it('renders page buttons', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 2, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    expect(wrapper.findAll('button').length).toBeGreaterThan(2)
  })

  it('emits page-change when a page button is clicked', async () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    const buttons = wrapper.findAll('button')
    // Click page 2 (third button after prev + page 1)
    await buttons[2].trigger('click')
    expect(wrapper.emitted('page-change')).toBeTruthy()
  })

  it('disables previous button on first page', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    expect(wrapper.find('button:first-child').attributes('disabled')).toBeDefined()
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/PaginationControls.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/shared/PaginationControls.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  page: number
  page_size: number
  total: number
  total_pages: number
}>()

const emit = defineEmits<{
  'page-change': [page: number]
}>()

const paginationRange = computed(() => {
  const delta = 2
  const start = Math.max(1, props.page - delta)
  const end = Math.min(props.total_pages, props.page + delta)
  const range: number[] = []
  for (let i = start; i <= end; i++) range.push(i)
  return range
})

const onPageChange = (newPage: number) => {
  if (newPage >= 1 && newPage <= props.total_pages) {
    emit('page-change', newPage)
  }
}
</script>

<template>
  <div v-if="total_pages > 1">
    <div class="join mt-8 flex justify-center">
      <button
        class="join-item btn"
        :disabled="page === 1"
        @click="onPageChange(page - 1)"
      >
        {{ t('calendars.pagePrevious') }}
      </button>

      <button
        v-for="p in paginationRange"
        :key="p"
        class="join-item btn"
        :class="{ 'btn-primary': p === page }"
        @click="onPageChange(p)"
      >
        {{ p }}
      </button>

      <button
        class="join-item btn"
        :disabled="page === total_pages"
        @click="onPageChange(page + 1)"
      >
        {{ t('calendars.pageNext') }}
      </button>
    </div>

    <div class="text-center mt-4 text-sm text-base-content/60">
      {{
        t('calendars.paginationText', {
          first: page_size * (page - 1) + 1,
          last: Math.min(page_size * page, total),
          total
        })
      }}
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/PaginationControls.spec.ts
```
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/shared/PaginationControls.vue frontend/src/__tests__/PaginationControls.spec.ts
git commit -m "feat: add PaginationControls shared component"
```

---

## Task 5: `ItemSearchPanel` component

Extracts the search form + fetched-items list from `CalendarPage.vue`.

**Files:**
- Create: `frontend/src/components/calendar/ItemSearchPanel.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/ItemSearchPanel.spec.ts`:

```typescript
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const mockItem: Item = {
  id: 1, id_mal: null,
  title: { english: 'Naruto', native: 'ナルト', romaji: 'Naruto' },
  media_type: 'ANIME', episode_duration: 23, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
}

describe('ItemSearchPanel', () => {
  it('renders the search input', () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.find('input[type="text"]').exists()).toBe(true)
  })

  it('emits search event with name and media type when button clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('input[type="text"]').setValue('Naruto')
    await wrapper.find('button[type="button"]').trigger('click')
    const emitted = wrapper.emitted('search')
    expect(emitted).toBeTruthy()
    expect(emitted![0][0]).toMatchObject({ name: 'Naruto' })
  })

  it('renders fetched items', () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Naruto')
  })

  it('emits toggle-selection when item is clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="item-card-1"]').trigger('click')
    expect(wrapper.emitted('toggle-selection')).toBeTruthy()
  })

  it('emits add-selected when add button clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [1], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="add-selected-btn"]').trigger('click')
    expect(wrapper.emitted('add-selected')).toBeTruthy()
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/ItemSearchPanel.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/calendar/ItemSearchPanel.vue`:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

const { t } = useI18n()

const props = defineProps<{
  fetchedItems: Item[]
  selectedItems: number[]
  itemsInCalendar: Item[]
  loading: boolean
  calendarLanguage: 'english' | 'romaji' | 'native'
  searchError?: string | null
}>()

const emit = defineEmits<{
  search: [{ name: string; mediaType: '' | 'ANIME' | 'MANGA' }]
  'toggle-selection': [id: number]
  'add-selected': []
}>()

const nameInput = ref('')
const mediaType = ref<'' | 'ANIME' | 'MANGA'>('')

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}

const handleSearch = () => {
  emit('search', { name: nameInput.value, mediaType: mediaType.value })
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- Search form -->
    <div class="form-control">
      <label class="label">
        <span class="label-text">{{ t('calendar.itemName') }}:</span>
      </label>
      <input
        v-model="nameInput"
        type="text"
        :placeholder="t('calendar.itemNamePlaceholder')"
        class="input input-bordered mt-2 w-full"
        @keyup.enter="handleSearch"
      />
    </div>

    <div class="form-control">
      <label class="label mb-2">
        <span class="label-text">{{ t('calendar.mediaType') }}:</span>
      </label>
      <div class="flex gap-4 flex-wrap">
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeAny') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="ANIME" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeAnime') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input v-model="mediaType" type="radio" value="MANGA" class="radio radio-primary" />
          <span class="label-text">{{ t('calendar.mediaTypeManga') }}</span>
        </label>
      </div>
    </div>

    <button
      type="button"
      :disabled="loading"
      class="btn btn-primary w-full"
      @click="handleSearch"
    >
      {{ loading ? t('calendar.fetchingItems') : t('calendar.fetchItems') }}
    </button>

    <div v-if="searchError" class="alert alert-error">{{ searchError }}</div>

    <!-- Results list -->
    <div class="overflow-y-auto max-h-[400px] min-h-[250px] p-2 border rounded flex flex-col gap-1">
      <MediaItemCard
        v-for="item in fetchedItems"
        :key="item.id"
        :data-testid="`item-card-${item.id}`"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="selectedItems.includes(item.id)"
        :is-in-calendar="itemsInCalendar.some(c => c.id === item.id)"
        @click="!itemsInCalendar.some(c => c.id === item.id) && emit('toggle-selection', item.id)"
      />
    </div>

    <button
      data-testid="add-selected-btn"
      :disabled="selectedItems.length === 0"
      class="btn btn-primary w-full"
      @click="emit('add-selected')"
    >
      {{ t('calendar.addSelectedToCalendar') }}
    </button>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/ItemSearchPanel.spec.ts
```
Expected: PASS (5 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/calendar/ItemSearchPanel.vue frontend/src/__tests__/ItemSearchPanel.spec.ts
git commit -m "feat: add ItemSearchPanel calendar sub-component"
```

---

## Task 6: `CalendarItemsList` component

Extracts the items-in-calendar list + remove/clear buttons from `CalendarPage.vue`.

**Files:**
- Create: `frontend/src/components/calendar/CalendarItemsList.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/CalendarItemsList.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const mockItem: Item = {
  id: 1, id_mal: null,
  title: { english: 'Bleach', native: 'ブリーチ', romaji: 'Bleach' },
  media_type: 'ANIME', episode_duration: 24, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
}

describe('CalendarItemsList', () => {
  it('renders item titles', () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Bleach')
  })

  it('emits remove when remove button is clicked', async () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="remove-item-1"]').trigger('click')
    expect(wrapper.emitted('remove')?.[0]).toEqual([1])
  })

  it('emits clear when clear button is clicked', async () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="clear-btn"]').trigger('click')
    expect(wrapper.emitted('clear')).toBeTruthy()
  })

  it('disables clear button when no items', () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [], calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="clear-btn"]').attributes('disabled')).toBeDefined()
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/CalendarItemsList.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/calendar/CalendarItemsList.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

const { t } = useI18n()

const props = defineProps<{
  items: Item[]
  calendarLanguage: 'english' | 'romaji' | 'native'
}>()

const emit = defineEmits<{
  remove: [id: number]
  clear: []
}>()

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <h3 class="font-bold">{{ t('calendar.itemsInCalendar') }}</h3>
    <div class="overflow-y-auto max-h-[400px] min-h-[400px] p-2 border rounded flex flex-col gap-1">
      <MediaItemCard
        v-for="item in items"
        :key="item.id"
        :item="item"
        :display-title="getTitle(item)"
        :is-selected="false"
        :is-in-calendar="false"
      >
        <button
          :data-testid="`remove-item-${item.id}`"
          class="btn btn-sm btn-error mt-2 w-full"
          @click.stop="emit('remove', item.id)"
        >
          {{ t('calendar.remove') }}
        </button>
      </MediaItemCard>
    </div>

    <button
      data-testid="clear-btn"
      class="btn btn-warning w-full"
      :disabled="items.length === 0"
      @click="emit('clear')"
    >
      {{ t('calendar.clear') }}
    </button>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/CalendarItemsList.spec.ts
```
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/calendar/CalendarItemsList.vue frontend/src/__tests__/CalendarItemsList.spec.ts
git commit -m "feat: add CalendarItemsList calendar sub-component"
```

---

## Task 7: `CalendarSettingsForm` component

Extracts name input + language selector + submit from `CalendarPage.vue`.

**Files:**
- Create: `frontend/src/components/calendar/CalendarSettingsForm.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/CalendarSettingsForm.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

describe('CalendarSettingsForm', () => {
  it('renders the name input with provided value', () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: 'My Calendar', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    expect((wrapper.find('input[type="text"]').element as HTMLInputElement).value).toBe('My Calendar')
  })

  it('emits update:name when name changes', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    await wrapper.find('input[type="text"]').setValue('New Name')
    expect(wrapper.emitted('update:name')?.[0]).toEqual(['New Name'])
  })

  it('emits update:language when language changes', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    const radios = wrapper.findAll('input[type="radio"]')
    await radios[1].trigger('change')
    expect(wrapper.emitted('update:language')).toBeTruthy()
  })

  it('emits submit when submit button is clicked', async () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: 'Test', language: 'english', loading: false, canSubmit: true },
      ...mountOpts
    })
    await wrapper.find('[data-testid="submit-btn"]').trigger('click')
    expect(wrapper.emitted('submit')).toBeTruthy()
  })

  it('disables submit when canSubmit is false', () => {
    const wrapper = mount(CalendarSettingsForm, {
      props: { name: '', language: 'english', loading: false, canSubmit: false },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="submit-btn"]').attributes('disabled')).toBeDefined()
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/CalendarSettingsForm.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/calendar/CalendarSettingsForm.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  name: string
  language: 'english' | 'romaji' | 'native'
  loading: boolean
  canSubmit: boolean
  error?: string | null
}>()

const emit = defineEmits<{
  'update:name': [value: string]
  'update:language': [value: 'english' | 'romaji' | 'native']
  submit: []
}>()
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="form-control">
      <label class="label">
        <span class="label-text">{{ t('calendar.name') }}:</span>
      </label>
      <div class="relative mt-2">
        <input
          type="text"
          :value="name"
          :placeholder="t('calendar.namePlaceholder')"
          class="input input-bordered w-full pr-16"
          maxlength="100"
          @input="emit('update:name', ($event.target as HTMLInputElement).value)"
        />
        <span class="absolute right-3 top-1/2 -translate-y-1/2 text-sm text-base-content/50">
          {{ name.length }}/100
        </span>
      </div>
    </div>

    <div class="form-control">
      <label class="label mb-2">
        <span class="label-text">{{ t('calendar.language') }}:</span>
      </label>
      <div class="flex gap-4 flex-wrap">
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'english'"
            value="english"
            class="radio radio-primary"
            @change="emit('update:language', 'english')"
          />
          <span class="label-text">{{ t('calendar.english') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'romaji'"
            value="romaji"
            class="radio radio-primary"
            @change="emit('update:language', 'romaji')"
          />
          <span class="label-text">{{ t('calendar.romaji') }}</span>
        </label>
        <label class="label cursor-pointer gap-2">
          <input
            type="radio"
            :checked="language === 'native'"
            value="native"
            class="radio radio-primary"
            @change="emit('update:language', 'native')"
          />
          <span class="label-text">{{ t('calendar.native') }}</span>
        </label>
      </div>
    </div>

    <div v-if="error" class="alert alert-error">{{ error }}</div>

    <button
      data-testid="submit-btn"
      :disabled="loading || !canSubmit"
      class="btn btn-success"
      @click="emit('submit')"
    >
      {{ loading ? t('calendar.submitting') : t('calendar.submit') }}
    </button>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/CalendarSettingsForm.spec.ts
```
Expected: PASS (5 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/calendar/CalendarSettingsForm.vue frontend/src/__tests__/CalendarSettingsForm.spec.ts
git commit -m "feat: add CalendarSettingsForm calendar sub-component"
```

---

## Task 8: `RecommendationsSection` component

Extracts the recommendations carousel from `CalendarPage.vue`.

**Files:**
- Create: `frontend/src/components/calendar/RecommendationsSection.vue`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/__tests__/RecommendationsSection.spec.ts`:

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const makeItem = (id: number, title: string): Item => ({
  id, id_mal: null,
  title: { english: title, native: title, romaji: title },
  media_type: 'ANIME', episode_duration: 0, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
})

describe('RecommendationsSection', () => {
  it('shows empty state when no items in calendar', () => {
    const wrapper = mount(RecommendationsSection, {
      props: { recommendations: [], calendarHasItems: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain(en.calendar.addItemsToSeeRecommendations)
  })

  it('shows no-recommendations text when calendar has items but no recommendations', () => {
    const wrapper = mount(RecommendationsSection, {
      props: { recommendations: [], calendarHasItems: true, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain(en.calendar.noRecommendations)
  })

  it('renders recommendation items', () => {
    const wrapper = mount(RecommendationsSection, {
      props: {
        recommendations: [makeItem(1, 'One Piece')],
        calendarHasItems: true,
        calendarLanguage: 'english'
      },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('One Piece')
  })

  it('emits add when add button is clicked', async () => {
    const wrapper = mount(RecommendationsSection, {
      props: {
        recommendations: [makeItem(1, 'One Piece')],
        calendarHasItems: true,
        calendarLanguage: 'english'
      },
      ...mountOpts
    })
    await wrapper.find('[data-testid="add-reco-1"]').trigger('click')
    expect(wrapper.emitted('add')?.[0][0]).toMatchObject({ id: 1 })
  })
})
```

- [ ] **Step 2: Run to verify failure**

```
cd frontend && npx vitest run src/__tests__/RecommendationsSection.spec.ts
```
Expected: FAIL

- [ ] **Step 3: Implement the component**

Create `frontend/src/components/calendar/RecommendationsSection.vue`:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'

const { t } = useI18n()

const props = defineProps<{
  recommendations: Item[]
  calendarHasItems: boolean
  calendarLanguage: 'english' | 'romaji' | 'native'
}>()

const emit = defineEmits<{
  add: [item: Item]
}>()

const getTitle = (item: Item): string => {
  switch (props.calendarLanguage) {
    case 'english': return item.title.english.length > 0 ? item.title.english : item.title.romaji
    case 'romaji': return item.title.romaji
    case 'native': return item.title.native
    default: return item.title.romaji
  }
}
</script>

<template>
  <div class="card bg-base-100 shadow-md mt-4">
    <div class="card-body">
      <h2 class="card-title">{{ t('calendar.recommendedItems') }}</h2>

      <div v-if="recommendations.length > 0" class="carousel carousel-center w-full gap-1">
        <div
          v-for="item in recommendations"
          :key="item.id"
          class="carousel-item w-[calc(20%-6px)]"
        >
          <MediaItemCard
            :item="item"
            :display-title="getTitle(item)"
            :is-selected="false"
            :is-in-calendar="false"
            compact
          >
            <button
              :data-testid="`add-reco-${item.id}`"
              class="btn btn-primary btn-xs mt-2 w-full"
              @click.stop="emit('add', item)"
            >
              {{ t('calendar.add') }}
            </button>
          </MediaItemCard>
        </div>
      </div>

      <div v-else-if="calendarHasItems" class="flex justify-center items-center py-4">
        <p>{{ t('calendar.noRecommendations') }}</p>
      </div>

      <div v-else class="flex justify-center items-center py-4">
        <p>{{ t('calendar.addItemsToSeeRecommendations') }}</p>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Run tests**

```
cd frontend && npx vitest run src/__tests__/RecommendationsSection.spec.ts
```
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/calendar/RecommendationsSection.vue frontend/src/__tests__/RecommendationsSection.spec.ts
git commit -m "feat: add RecommendationsSection calendar sub-component"
```

---

## Task 9: Refactor `CalendarPage.vue`

Replace the ~900-line monolith with the new sub-components. All business logic (API calls, sessionStorage, calculateRecommendations) stays here — only the template is simplified.

**Files:**
- Modify: `frontend/src/components/CalendarPage.vue`

- [ ] **Step 1: Replace the template**

The entire `<template>` section of `CalendarPage.vue` becomes:

```vue
<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">{{ $t('calendar.edit') }}</h1>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 max-w-6xl mx-auto">
      <!-- Calendar settings + items list -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body flex flex-col gap-4">
          <CalendarSettingsForm
            :name="calendarName"
            :language="calendarLanguage"
            :loading="loading"
            :can-submit="itemsInCalendar.length > 0"
            :error="calendarError"
            @update:name="calendarName = $event"
            @update:language="calendarLanguage = $event"
            @submit="submitCalendar"
          />
          <CalendarItemsList
            :items="itemsInCalendar"
            :calendar-language="calendarLanguage"
            @remove="removeItemFromCalendar"
            @clear="clearCalendar"
          />
        </div>
      </div>

      <!-- Search panel -->
      <div class="card bg-base-100 shadow-md">
        <div class="card-body">
          <h2 class="card-title">{{ $t('calendar.search') }}</h2>
          <ItemSearchPanel
            :fetched-items="fetchedItems"
            :selected-items="selectedItems"
            :items-in-calendar="itemsInCalendar"
            :loading="loading"
            :calendar-language="calendarLanguage"
            :search-error="searchError"
            @search="handleSearch"
            @toggle-selection="toggleItemSelection"
            @add-selected="addItemToCalendar"
          />
        </div>
      </div>
    </div>

    <!-- Recommendations (desktop only) -->
    <div class="max-w-6xl mx-auto hidden lg:block">
      <RecommendationsSection
        :recommendations="recommendations"
        :calendar-has-items="itemsInCalendar.length > 0"
        :calendar-language="calendarLanguage"
        @add="addItemToCalendarSingle"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: Update the `<script setup>` section**

Replace the top of the script section. Keep all existing logic, but:
1. Remove the old `isMobile` computed — it is no longer needed (CSS handles responsiveness)
2. Remove `onImageError`/`onImageLoad` — now inside `MediaItemCard`
3. Add a `handleSearch` function that `ItemSearchPanel` calls
4. Add imports for all new sub-components
5. Replace the direct `fetchItems` call with `handleSearch`

Full updated `<script setup>`:

```typescript
import { ref, onMounted, watch, onBeforeMount, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { Item } from '@/types/item'
import type { Calendar } from '@/types/calendar'
import api from '@/config/api'
import { toastService } from '@/services/toastService'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import CalendarSettingsForm from '@/components/calendar/CalendarSettingsForm.vue'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const userSettingsStore = useUserSettingsStore()

const nameInput = ref('')
const mediaType = ref<'ANIME' | 'MANGA' | ''>('')
const fetchedItems = ref<Item[]>([])
const selectedItems = ref<number[]>([])
const calendarName = ref('')
const calendarLanguage = ref<'english' | 'romaji' | 'native'>('english')
const itemsInCalendar = ref<Item[]>([])
const loading = ref(false)
const searchError = ref<string | null>(null)
const calendarError = ref<string | null>(null)
const recommendations = ref<Item[]>([])
const currentCalendar = ref<Calendar | null>(null)

const handleSearch = async ({ name, mediaType: type }: { name: string; mediaType: '' | 'ANIME' | 'MANGA' }) => {
  if (!name) {
    searchError.value = t('calendar.enterName')
    return
  }
  loading.value = true
  searchError.value = null
  try {
    let url = `/items/search?name=${encodeURIComponent(name)}`
    if (type) url += `&media_type=${type}`
    const response = await api.get(url)
    fetchedItems.value = response.data
    selectedItems.value = []
  } catch {
    searchError.value = t('calendar.failedToFetchItems')
  } finally {
    loading.value = false
  }
}

const toggleItemSelection = (id: number) => {
  const idx = selectedItems.value.indexOf(id)
  if (idx === -1) selectedItems.value.push(id)
  else selectedItems.value.splice(idx, 1)
}

const addItemToCalendar = () => {
  const newItems = fetchedItems.value.filter(
    item => selectedItems.value.includes(item.id) &&
    !itemsInCalendar.value.some(c => c.id === item.id)
  )
  itemsInCalendar.value.push(...newItems)
  selectedItems.value = []
  if (itemsInCalendar.value.length > 0) calculateRecommendations()
}

const addItemToCalendarSingle = (item: Item) => {
  if (!itemsInCalendar.value.some(c => c.id === item.id)) {
    itemsInCalendar.value.push(item)
    calculateRecommendations()
  }
}

const removeItemFromCalendar = (id: number) => {
  itemsInCalendar.value = itemsInCalendar.value.filter(item => item.id !== id)
  calculateRecommendations()
}

const clearCalendar = () => {
  itemsInCalendar.value = []
  recommendations.value = []
}

const submitCalendar = async () => {
  loading.value = true
  calendarError.value = null
  try {
    const payload = {
      name: calendarName.value,
      language: calendarLanguage.value,
      item_ids: itemsInCalendar.value.map(item => item.id)
    }
    if (currentCalendar.value) {
      await api.put(`/calendars/${currentCalendar.value.id}`, payload)
    } else {
      await api.put('/calendars', payload)
    }
    toastService.success(t('calendar.saveSuccess'))
    router.push('/my-calendars')
  } catch {
    calendarError.value = t('calendar.saveFailed')
  } finally {
    loading.value = false
  }
}

const calculateRecommendations = () => {
  if (itemsInCalendar.value.length === 0) { recommendations.value = []; return }
  const counts = new Map<number, { count: number; totalRating: number; item: Item }>()
  itemsInCalendar.value.forEach(item => {
    item.recommendations?.forEach(rec => {
      const id = rec.media.id
      if (itemsInCalendar.value.some(c => c.id === id)) return
      if (counts.has(id)) {
        const e = counts.get(id)!; e.count++; e.totalRating += rec.rating
      } else {
        counts.set(id, {
          count: 1, totalRating: rec.rating,
          item: {
            id: rec.media.id, id_mal: rec.media.id_mal,
            title: rec.media.title, media_type: item.media_type,
            episode_duration: 0, airing_schedule: [],
            cover_image: rec.media.cover_image, banner_image: '', recommendations: []
          }
        })
      }
    })
  })
  recommendations.value = Array.from(counts.values())
    .sort((a, b) => b.count !== a.count ? b.count - a.count : (b.totalRating / b.count) - (a.totalRating / a.count))
    .slice(0, 5)
    .map(e => e.item)
}

// sessionStorage persistence
onMounted(() => {
  try {
    const saved = sessionStorage.getItem('calendarPageState')
    if (saved) {
      const state = JSON.parse(saved)
      if (state.nameInput !== undefined) nameInput.value = state.nameInput
      if (state.mediaType !== undefined) mediaType.value = state.mediaType
      if (state.fetchedItems !== undefined) fetchedItems.value = state.fetchedItems
      if (state.selectedItems !== undefined) selectedItems.value = state.selectedItems
      if (state.itemsInCalendar !== undefined) itemsInCalendar.value = state.itemsInCalendar
      if (state.calendarName !== undefined) calendarName.value = state.calendarName
      if (state.calendarLanguage !== undefined) calendarLanguage.value = state.calendarLanguage
    }
  } catch { /* ignore */ }
})

watch([nameInput, mediaType, fetchedItems, selectedItems, itemsInCalendar, calendarName, calendarLanguage], () => {
  try {
    sessionStorage.setItem('calendarPageState', JSON.stringify({
      nameInput: nameInput.value, mediaType: mediaType.value,
      fetchedItems: fetchedItems.value, selectedItems: selectedItems.value,
      itemsInCalendar: itemsInCalendar.value, calendarName: calendarName.value,
      calendarLanguage: calendarLanguage.value
    }))
  } catch { /* ignore */ }
})

onBeforeUnmount(() => {
  if (!route.params.id || route.params.id === 'new') {
    sessionStorage.removeItem('calendarPageState')
  }
})

onBeforeMount(async () => {
  const calendarId = Array.isArray(route.params.id) ? route.params.id[0] : route.params.id
  if (calendarId && calendarId !== 'new') {
    loading.value = true
    try {
      const response = await api.get(`/calendars/${calendarId}`)
      const calendar: Calendar = response.data
      calendarName.value = calendar.name
      calendarLanguage.value = calendar.language
      itemsInCalendar.value = calendar.items
      currentCalendar.value = calendar
      calculateRecommendations()
    } catch {
      calendarError.value = t('calendar.failedToLoad')
    } finally {
      loading.value = false
    }
  } else {
    const settings = await userSettingsStore.fetchSettings()
    calendarLanguage.value =
      settings.title_language_preference === 'Romaji' ? 'romaji' :
      settings.title_language_preference === 'Native' ? 'native' : 'english'
  }
})
```

- [ ] **Step 3: Check for `calendar.saveSuccess` and `calendar.saveFailed` i18n keys**

Open `frontend/src/locales/en.json`. If the keys `calendar.saveSuccess` and `calendar.saveFailed` don't exist under `"calendar"`, add them:
```json
"saveSuccess": "Calendar saved successfully",
"saveFailed": "Failed to save calendar"
```

Open `frontend/src/locales/pt.json` and add matching keys:
```json
"saveSuccess": "Calendário salvo com sucesso",
"saveFailed": "Falha ao salvar o calendário"
```

Also check that existing key `calendar.fetchingItems` exists; if not, add:
- en: `"fetchingItems": "Fetching..."`
- pt: `"fetchingItems": "Buscando..."`

- [ ] **Step 4: Build to verify no type errors**

```
cd frontend && npm run build 2>&1 | tail -20
```
Expected: build succeeds (no TypeScript errors)

- [ ] **Step 5: Verify Vitest still passes**

```
cd frontend && npx vitest run 2>&1 | tail -20
```
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/CalendarPage.vue frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "refactor: decompose CalendarPage into sub-components"
```

---

## Task 10: Refactor `MyCalendarsPage.vue`

Replace `confirm()` with `ConfirmModal` and `PaginationControls`.

**Files:**
- Modify: `frontend/src/components/MyCalendarsPage.vue`

- [ ] **Step 1: Update the component**

Replace the entire file contents:

```vue
<template>
  <div class="min-h-[calc(100vh-6.1rem)] bg-base-200 p-4">
    <h1 class="text-2xl font-bold mb-6">{{ $t('calendars.title') }}</h1>

    <div class="flex justify-center mb-6">
      <button class="btn btn-primary" @click="createNewCalendar">
        {{ $t('calendars.createNew') }}
      </button>
    </div>

    <div v-if="loading" class="alert alert-info">{{ $t('calendars.loading') }}</div>
    <div v-else-if="error" class="alert alert-error">{{ error }}</div>
    <div v-else-if="calendars.length === 0" class="alert alert-info">
      {{ $t('calendars.notFound') }}
      <button class="btn btn-sm btn-primary ml-2" @click="createNewCalendar">
        {{ $t('calendars.createNew') }}
      </button>
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div
        v-for="calendar in calendars"
        :key="calendar.id"
        class="card bg-base-100 shadow-md hover:shadow-lg transition-shadow duration-300"
      >
        <div class="card-body">
          <h2 class="card-title text-lg font-bold">{{ calendar.name }}</h2>
          <div class="space-y-1">
            <p class="text-sm text-base-content/60">
              {{ $t('calendars.created') }}: <span class="font-semibold">{{ formatDate(calendar.created_at) }}</span>
            </p>
            <p class="text-sm text-base-content/60">
              {{ $t('calendars.updated') }}: <span class="font-semibold">{{ formatDate(calendar.updated_at) }}</span>
            </p>
          </div>
          <div class="mt-3 flex items-center gap-4">
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" fill="currentColor" class="text-blue-500" viewBox="0 0 16 16">
                <path d="M2.5 13.5A.5.5 0 0 1 3 13h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5M13.991 3l.024.001a1.5 1.5 0 0 1 .538.143.76.76 0 0 1 .302.254c.067.1.145.277.145.602v5.991l-.001.024a1.5 1.5 0 0 1-.143.538.76.76 0 0 1-.254.302c-.1.067-.277.145-.602.145H2.009l-.024-.001a1.5 1.5 0 0 1-.538-.143.76.76 0 0 1-.302-.254C1.078 10.502 1 10.325 1 10V4.009l.001-.024a1.5 1.5 0 0 1 .143-.538.76.76 0 0 1 .254-.302C1.498 3.078 1.675 3 2 3zM14 2H2C0 2 0 4 0 4v6c0 2 2 2 2 2h12c2 0 2-2 2-2V4c0-2-2-2-2-2"/>
              </svg>
              <span class="text-sm">{{ calendar.item_count }}</span>
            </div>
          </div>
          <div class="card-actions justify-end mt-4">
            <button class="btn btn-sm btn-success" @click.stop="exportCalendar(calendar.id)">
              {{ $t('calendars.export') }}
            </button>
            <button class="btn btn-sm btn-primary" @click.stop="editCalendar(calendar.id)">
              {{ $t('calendars.edit') }}
            </button>
            <button class="btn btn-sm btn-error" @click.stop="confirmDelete(calendar.id)">
              {{ $t('calendars.delete') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <PaginationControls
      :page="pagination.page"
      :page_size="pagination.page_size"
      :total="pagination.total"
      :total_pages="pagination.total_pages"
      @page-change="loadCalendars"
    />

    <ConfirmModal
      :open="confirmModalOpen"
      :title="$t('calendars.deleteConfirmTitle')"
      :message="$t('calendars.deleteConfirmMessage')"
      :confirm-label="$t('calendars.delete')"
      :danger="true"
      @confirm="executeDelete"
      @cancel="confirmModalOpen = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { PageCalendar } from '@/types/calendar'
import api from '@/config/api'
import PaginationControls from '@/components/shared/PaginationControls.vue'
import ConfirmModal from '@/components/shared/ConfirmModal.vue'

const { t } = useI18n()
const router = useRouter()

const calendars = ref<PageCalendar[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const pagination = ref({ page: 1, page_size: 6, total: 0, total_pages: 0 })
const confirmModalOpen = ref(false)
const calendarToDelete = ref<number | null>(null)

onMounted(() => loadCalendars(1))

const loadCalendars = async (page: number = 1) => {
  loading.value = true
  error.value = null
  try {
    const response = await api.get('/calendars', { params: { page, page_size: pagination.value.page_size } })
    calendars.value = response.data.data
    pagination.value = response.data.pagination
  } catch {
    error.value = t('calendars.loadingFailed')
  } finally {
    loading.value = false
  }
}

const createNewCalendar = () => router.push('/calendar/new')
const editCalendar = (id: number) => router.push(`/calendar/${id}`)

const confirmDelete = (id: number) => {
  calendarToDelete.value = id
  confirmModalOpen.value = true
}

const executeDelete = async () => {
  confirmModalOpen.value = false
  if (calendarToDelete.value === null) return
  try {
    await api.delete(`/calendars/${calendarToDelete.value}`)
    await loadCalendars(pagination.value.page)
  } catch {
    error.value = t('calendars.deleteFailed')
  } finally {
    calendarToDelete.value = null
  }
}

const formatDate = (dateString: string) => new Date(dateString).toLocaleDateString()

const exportCalendar = async (id: number) => {
  try {
    const response = await api.get(`/calendars/${id}/export`, { responseType: 'text' })
    const blob = new Blob([response.data], { type: 'text/calendar' })
    const url = window.URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `calendar-${id}.ics`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
  } catch {
    error.value = t('calendars.exportFailed')
  }
}
</script>
```

- [ ] **Step 2: Add missing i18n keys**

In `en.json` under `"calendars"`, add if missing:
```json
"deleteConfirmTitle": "Delete Calendar",
"deleteConfirmMessage": "Are you sure you want to delete this calendar? This cannot be undone.",
"exportFailed": "Failed to export calendar"
```

In `pt.json` under `"calendars"`:
```json
"deleteConfirmTitle": "Excluir Calendário",
"deleteConfirmMessage": "Tem certeza que deseja excluir este calendário? Isso não pode ser desfeito.",
"exportFailed": "Falha ao exportar o calendário"
```

- [ ] **Step 3: Build and verify**

```
cd frontend && npm run build 2>&1 | tail -20
```
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add frontend/src/components/MyCalendarsPage.vue frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "refactor: replace confirm() with ConfirmModal in MyCalendarsPage"
```

---

## Task 11: Fix `UserDetailsPage.vue`

Fix `disabled=true`, replace `confirm()` with `ConfirmModal`, standardize i18n.

**Files:**
- Modify: `frontend/src/components/UserDetailsPage.vue`

- [ ] **Step 1: In the `<script setup>`, replace `i18n.global` usage**

Change:
```typescript
import { i18n } from '@/plugins/i18n'
const { t } = i18n.global
```
To:
```typescript
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
```

- [ ] **Step 2: Replace `confirm()` with `ConfirmModal` state**

Add to the script refs:
```typescript
import ConfirmModal from '@/components/shared/ConfirmModal.vue'
const confirmDeleteOpen = ref(false)
```

Replace the `handleDelete` function:
```typescript
const handleDelete = async () => {
  confirmDeleteOpen.value = false
  try {
    isDeleting.value = true
    await api.delete('/user')
    authStore.logout()
    userSettingsStore.clearCache()
    applySettings(userSettingsStore.getDefaultSettings())
    router.push('/login')
  } catch {
    error.value = t('userDetails.accountDeleteFailed')
  } finally {
    isDeleting.value = false
  }
}
```

- [ ] **Step 3: Fix `disabled=true` in template**

Find all occurrences of `disabled=true` (there are two in the view inputs) and change to `:disabled="true"`.

- [ ] **Step 4: Add ConfirmModal to template**

Add the delete button trigger (change `@click="handleDelete"` to `@click="confirmDeleteOpen = true"`) and add below the root `<div>`:
```vue
<ConfirmModal
  :open="confirmDeleteOpen"
  :title="$t('userDetails.accountDeleteButton')"
  :message="$t('userDetails.accountDeleteWarning')"
  :confirm-label="$t('userDetails.accountDeleteButton')"
  :danger="true"
  @confirm="handleDelete"
  @cancel="confirmDeleteOpen = false"
/>
```

- [ ] **Step 5: Build and verify**

```
cd frontend && npm run build 2>&1 | tail -20
```
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/UserDetailsPage.vue
git commit -m "fix: replace confirm() and fix disabled binding in UserDetailsPage"
```

---

## Task 12: Fix `UserSettingsPage.vue` and `LoginPage.vue`

Remove `console.log`, standardize i18n.

**Files:**
- Modify: `frontend/src/components/UserSettingsPage.vue`
- Modify: `frontend/src/components/LoginPage.vue`

- [ ] **Step 1: `UserSettingsPage.vue` — remove console.log and standardize i18n**

Remove line:
```typescript
console.log('Fetched settings:', fetchedSettings)
```

Change:
```typescript
import { i18n } from '@/plugins/i18n'
const { t } = i18n.global
```
To:
```typescript
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
```

- [ ] **Step 2: `LoginPage.vue` — standardize i18n**

Change:
```typescript
import { i18n } from '@/plugins/i18n'
const { t } = i18n.global
```
To:
```typescript
import { useI18n } from 'vue-i18n'
const { t } = useI18n()
```

- [ ] **Step 3: Build and verify**

```
cd frontend && npm run build 2>&1 | tail -20
```
Expected: no errors

- [ ] **Step 4: Run all tests**

```
cd frontend && npx vitest run 2>&1 | tail -30
```
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add frontend/src/components/UserSettingsPage.vue frontend/src/components/LoginPage.vue
git commit -m "fix: remove console.log and standardize i18n usage"
```

---

## Task 13: End-to-end verification

Confirm the app works visually and functionally at http://localhost:5173.

- [ ] **Step 1: Start the dev server if not running**

```
cd frontend && npm run dev
```

- [ ] **Step 2: Smoke test checklist**

Navigate to each route and verify:

| Route | Check |
|-------|-------|
| `/login` | Login form renders; register toggle works; Google button present |
| `/my-calendars` | Calendar cards render with dates and item counts; pagination appears for > 6 calendars |
| `/my-calendars` (delete) | Clicking Delete shows `ConfirmModal`, not native browser dialog |
| `/calendar/new` | Search panel + settings form renders side-by-side on desktop |
| `/calendar/new` | Search for an anime, results appear with cover images and selection highlight |
| `/calendar/new` | Adding items to calendar shows them in the list; recommendations appear |
| `/calendar/:id` | Existing calendar loads with correct name, language, and items |
| `/user/details` | User info displays; edit mode works; password form works |
| `/user/details` (delete) | ConfirmModal appears on delete, no native dialog |
| `/user/settings` | Theme/language/timezone settings save correctly |
| Mobile viewport (< 768px) | Layout stacks vertically correctly |

- [ ] **Step 3: Run the full test suite**

```
cd frontend && npx vitest run && npm run lint
```
Expected: 0 failures, lint passes

- [ ] **Step 4: Final commit tag**

```bash
git add -A
git commit -m "chore: vue frontend component refactor complete"
```

---

## Self-review

### Spec coverage check

| Requirement | Covered by |
|-------------|-----------|
| Preserve all functionality | Tasks 9-12 maintain all existing API calls and business logic |
| Preserve all visuals | Template structure/classes unchanged; only extracted, not redesigned |
| `CalendarPage` decomposed | Tasks 5-9 |
| Repeated `MediaItemCard` extracted | Task 2 |
| `isMobile` reactivity fixed | Task 1 (removed entirely — CSS handles responsiveness) |
| `placeholder` bug fixed | Task 7 (CalendarSettingsForm uses `:placeholder`) |
| `confirm()` replaced | Tasks 3, 10, 11 |
| `console.log` removed | Task 12 |
| i18n standardized | Tasks 9-12 |
| Tests written for every new component | Tasks 1-8 each include tests |
| Build verified | Tasks 9, 10, 11, 12 each include build check |

### Placeholder scan
No TBD/TODO/placeholder patterns found. All code blocks are complete.

### Type consistency
- `Item` type imported from `@/types/item` in all components
- `Calendar` type imported from `@/types/calendar` where used
- `calendarLanguage` typed as `'english' | 'romaji' | 'native'` consistently
- `mediaType` typed as `'' | 'ANIME' | 'MANGA'` consistently
- `emit('page-change', page: number)` in `PaginationControls` matches `@page-change="loadCalendars"` in `MyCalendarsPage`
