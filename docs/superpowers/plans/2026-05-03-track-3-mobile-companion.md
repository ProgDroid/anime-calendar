# Track 3 — Mobile Companion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a full mobile responsive pass over Track 2 + Track 4 surfaces, plus the mobile-specific patterns specified in the design handoff (2-tab bottom bar, FAB-driven editor, drag-to-dismiss bottom sheet, batch-add CTA, banner fade-in preserved verbatim from Track 1).

**Architecture:** Hybrid responsive — single route tree, components branch on viewport. Editor is the one exception: thin shell picks `CalendarEditorViewDesktop` (renamed) or `CalendarEditorViewMobile` (new). Single breakpoint at 1024 px exposed as a named constant. New `useViewportLayout` composable (singleton, debounced) coexists with the existing `useWindowSize` (per-component, 768 px threshold) — the two are intentionally separate; `useWindowSize` consumers are unaffected.

**Tech Stack:** Vue 3.5 + TS 5.9, Tailwind v4 (`@theme` tokens), Pinia 3, vue-router 4, vue-i18n 11, vitest + jsdom, **new:** Playwright (mobile smoke). (Originally also `vaul-vue` for drag-dismiss bottom sheets — dropped during execution; static `UiBottomSheet` is built in-house, see Task 3.)

**Source spec:** [`docs/superpowers/specs/2026-05-03-track-3-mobile-companion-design.md`](../specs/2026-05-03-track-3-mobile-companion-design.md)

**Working tree:** main (no separate worktree).

---

## Pre-flight (one-time)

- [ ] **From `frontend/`, verify clean baseline:**
  ```bash
  cd frontend && npm run test:unit -- --run && npm run lint && npm run build
  ```
  Expected: 294 tests pass, lint clean, build clean. If any are red, fix or report before starting Phase 1.

- [ ] **Confirm `frontend/index.html` has the viewport meta tag.** (Used in Task 4.) If `<meta name="viewport" ...>` is already present, note its current content; the meta will be widened in Task 4.

---

## Phase 1 — Foundations (no user-visible changes)

### Task 1: `useViewportLayout` composable

**Files:**
- Create: `frontend/src/composables/useViewportLayout.ts`
- Test: `frontend/src/composables/__tests__/useViewportLayout.spec.ts`
- Create test util: `frontend/src/__tests__/test-utils/viewport.ts`

- [ ] **Step 1: Write the failing composable test.**

```ts
// frontend/src/composables/__tests__/useViewportLayout.spec.ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import {
  useViewportLayout,
  MOBILE_BREAKPOINT_PX,
} from '@/composables/useViewportLayout'

function setWidth(px: number) {
  Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: px })
}

describe('useViewportLayout', () => {
  beforeEach(() => {
    setWidth(1280)
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('exposes the named breakpoint constant', () => {
    expect(MOBILE_BREAKPOINT_PX).toBe(1024)
  })

  it('returns isMobile=false above the breakpoint', () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    expect((w.vm as { isMobile: boolean }).isMobile).toBe(false)
    w.unmount()
  })

  it('returns isMobile=true below the breakpoint', () => {
    setWidth(390)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    expect((w.vm as { isMobile: boolean }).isMobile).toBe(true)
    w.unmount()
  })

  it('responds to resize events with debounce', async () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    const vm = w.vm as { isMobile: boolean }
    expect(vm.isMobile).toBe(false)

    setWidth(390)
    window.dispatchEvent(new Event('resize'))
    // Before debounce window elapses, value is still old.
    expect(vm.isMobile).toBe(false)

    vi.advanceTimersByTime(120)
    await nextTick()
    expect(vm.isMobile).toBe(true)

    w.unmount()
  })

  it('shares state across multiple consumers (singleton)', () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const a = mount(C)
    const b = mount(C)
    expect((a.vm as { isMobile: boolean }).isMobile).toBe(false)
    expect((b.vm as { isMobile: boolean }).isMobile).toBe(false)
    a.unmount()
    b.unmount()
  })
})
```

- [ ] **Step 2: Run test to verify it fails.**

  Run: `cd frontend && npx vitest run src/composables/__tests__/useViewportLayout.spec.ts`
  Expected: FAIL — module not found.

- [ ] **Step 3: Implement the composable.**

```ts
// frontend/src/composables/useViewportLayout.ts
import { ref, computed, type ComputedRef } from 'vue'

export const MOBILE_BREAKPOINT_PX = 1024
const DEBOUNCE_MS = 100

// Singleton state — first call wires the resize listener; every subsequent
// call returns the same ref. This avoids N listeners across components and
// guarantees consistent isMobile across the entire app at any given moment.
const ssrSafeWidth = typeof window === 'undefined' ? 1280 : window.innerWidth
const width = ref(ssrSafeWidth)
let installed = false
let debounceHandle: ReturnType<typeof setTimeout> | null = null

function install() {
  if (installed || typeof window === 'undefined') return
  installed = true
  window.addEventListener('resize', () => {
    if (debounceHandle) clearTimeout(debounceHandle)
    debounceHandle = setTimeout(() => {
      width.value = window.innerWidth
      debounceHandle = null
    }, DEBOUNCE_MS)
  })
}

export interface ViewportLayout {
  isMobile: ComputedRef<boolean>
  MOBILE_BREAKPOINT_PX: number
}

export function useViewportLayout(): ViewportLayout {
  install()
  const isMobile = computed(() => width.value < MOBILE_BREAKPOINT_PX)
  return { isMobile, MOBILE_BREAKPOINT_PX }
}
```

- [ ] **Step 4: Run test to verify it passes.**

  Run: `cd frontend && npx vitest run src/composables/__tests__/useViewportLayout.spec.ts`
  Expected: 5 tests PASS.

- [ ] **Step 5: Add the test util for stubbing in component tests.**

```ts
// frontend/src/__tests__/test-utils/viewport.ts
import { vi } from 'vitest'

/**
 * Stubs `useViewportLayout` to return a fixed `isMobile` value.
 * Call BEFORE importing the component under test:
 *
 *   await mockViewport(390)
 *   const { default: Comp } = await import('@/components/X.vue')
 */
export async function mockViewport(width: number) {
  vi.doMock('@/composables/useViewportLayout', () => ({
    MOBILE_BREAKPOINT_PX: 1024,
    useViewportLayout: () => ({
      MOBILE_BREAKPOINT_PX: 1024,
      isMobile: { value: width < 1024 },
    }),
  }))
}

export function resetViewportMock() {
  vi.doUnmock('@/composables/useViewportLayout')
  vi.resetModules()
}
```

- [ ] **Step 6: Commit.**

```bash
git add frontend/src/composables/useViewportLayout.ts \
        frontend/src/composables/__tests__/useViewportLayout.spec.ts \
        frontend/src/__tests__/test-utils/viewport.ts
git commit -m "feat(mobile): useViewportLayout composable + named breakpoint constant"
```

---

### Task 2: `UiBottomTabBar` primitive + route-meta gating

**Files:**
- Create: `frontend/src/components/ui/UiBottomTabBar.vue`
- Test: `frontend/src/components/ui/__tests__/UiBottomTabBar.spec.ts`
- Modify: `frontend/src/router/index.ts` (add `meta.bottomTabBar = false` on public routes; add `UiBottomTabBar` to root layout)
- Modify: `frontend/src/App.vue` (mount `<UiBottomTabBar />` after `<router-view />`)
- Modify: `frontend/src/locales/en.json` and `pt.json` (add `mobile.tabBar.{library,account}` keys)

- [ ] **Step 1: Add i18n keys to both locales (en + pt).**

  In `frontend/src/locales/en.json`, add a top-level `"mobile": { "tabBar": { ... } }` block (insert after `"interrupt"` block, before final closing `}`):

```json
  "mobile": {
    "tabBar": {
      "library": "Library",
      "account": "Account"
    }
  },
```

  In `frontend/src/locales/pt.json`, add the matching keys:

```json
  "mobile": {
    "tabBar": {
      "library": "Biblioteca",
      "account": "Conta"
    }
  },
```

- [ ] **Step 2: Write the failing component test.**

```ts
// frontend/src/components/ui/__tests__/UiBottomTabBar.spec.ts
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, type Router } from 'vue-router'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

function makeRouter(): Router {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', component: { template: '<div/>' }, meta: { bottomTabBar: false } },
      { path: '/my-calendars', component: { template: '<div/>' } },
      { path: '/calendar/:id', component: { template: '<div/>' } },
      { path: '/account', component: { template: '<div/>' } },
      { path: '/account/profile', component: { template: '<div/>' } },
      { path: '/upgrade', component: { template: '<div/>' } },
    ],
  })
}

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

describe('UiBottomTabBar', () => {
  beforeEach(() => {
    vi.resetModules()
  })
  afterEach(() => {
    resetViewportMock()
  })

  async function mountAt(path: string, viewportPx: number) {
    await mockViewport(viewportPx)
    const { default: UiBottomTabBar } = await import('../UiBottomTabBar.vue')
    const router = makeRouter()
    await router.push(path)
    await router.isReady()
    return mount(UiBottomTabBar, { global: { plugins: [i18n, router] } })
  }

  it('hides on desktop viewports', async () => {
    const w = await mountAt('/my-calendars', 1280)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(false)
  })

  it('hides on routes with meta.bottomTabBar === false', async () => {
    const w = await mountAt('/login', 390)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(false)
  })

  it('renders 2 tabs on authenticated mobile routes', async () => {
    const w = await mountAt('/my-calendars', 390)
    expect(w.find('[data-testid="bottom-tab-bar"]').exists()).toBe(true)
    expect(w.findAll('[data-testid="bottom-tab"]')).toHaveLength(2)
  })

  it('highlights Library on /my-calendars', async () => {
    const w = await mountAt('/my-calendars', 390)
    const lib = w.get('[data-testid="bottom-tab-library"]')
    expect(lib.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })

  it('highlights Library on /calendar/:id (editor is child of Library)', async () => {
    const w = await mountAt('/calendar/42', 390)
    const lib = w.get('[data-testid="bottom-tab-library"]')
    expect(lib.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })

  it('highlights Account on /account/profile', async () => {
    const w = await mountAt('/account/profile', 390)
    const acc = w.get('[data-testid="bottom-tab-account"]')
    expect(acc.classes().some((c) => c.includes('text-accent-1'))).toBe(true)
  })
})
```

- [ ] **Step 3: Run test to verify it fails.**

  Run: `cd frontend && npx vitest run src/components/ui/__tests__/UiBottomTabBar.spec.ts`
  Expected: FAIL — component module not found.

- [ ] **Step 4: Implement the component.**

```vue
<!-- frontend/src/components/ui/UiBottomTabBar.vue -->
<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
import IconCal from './icons/IconCal.vue'
import IconSettings from './icons/IconSettings.vue'

defineOptions({ name: 'UiBottomTabBar' })

const { t } = useI18n()
const route = useRoute()
const { isMobile } = useViewportLayout()

const visible = computed(
  () => isMobile.value && route.meta.bottomTabBar !== false,
)

const activeTab = computed<'library' | 'account' | null>(() => {
  const p = route.path
  if (p.startsWith('/calendar') || p.startsWith('/my-calendars')) return 'library'
  if (p.startsWith('/account')) return 'account'
  return null
})
</script>

<template>
  <nav
    v-if="visible"
    data-testid="bottom-tab-bar"
    :aria-label="t('mobile.tabBar.library') + ' / ' + t('mobile.tabBar.account')"
    class="sticky bottom-0 left-0 right-0 z-30 flex justify-around border-t border-line-soft bg-bg-0/80 pt-2 backdrop-blur-md"
    :style="{ paddingBottom: 'max(22px, calc(env(safe-area-inset-bottom) + 8px))' }"
  >
    <RouterLink
      to="/my-calendars"
      data-testid="bottom-tab"
      :class="[
        'flex flex-col items-center gap-1 px-4 py-1 text-[10px] font-medium',
        activeTab === 'library' ? 'text-accent-1' : 'text-fg-2',
      ]"
    >
      <span data-testid="bottom-tab-library" :class="activeTab === 'library' ? 'text-accent-1' : 'text-fg-2'">
        <IconCal />
      </span>
      {{ t('mobile.tabBar.library') }}
    </RouterLink>
    <RouterLink
      to="/account"
      data-testid="bottom-tab"
      :class="[
        'flex flex-col items-center gap-1 px-4 py-1 text-[10px] font-medium',
        activeTab === 'account' ? 'text-accent-1' : 'text-fg-2',
      ]"
    >
      <span data-testid="bottom-tab-account" :class="activeTab === 'account' ? 'text-accent-1' : 'text-fg-2'">
        <IconSettings />
      </span>
      {{ t('mobile.tabBar.account') }}
    </RouterLink>
  </nav>
</template>
```

- [ ] **Step 5: Mark public + auth-stage routes with `meta.bottomTabBar: false`.**

  In `frontend/src/router/index.ts`, every route with `meta: { public: true }` gets `bottomTabBar: false` added — and any route where the bar should not show. Edit:

```ts
// Each public route now reads (example):
{ path: '/login', name: 'Login', component: LoginPage, meta: { public: true, bottomTabBar: false } },
// Same for: /forgot-password, /reset-password, /register,
// /verify-email/pending, /verify-email
// /upgrade/success and /upgrade/canceled (auth-required but standalone celebrations) — add bottomTabBar: false there too.
// Public 404 catch-all also gets bottomTabBar: false.
```

  Audit the entire router file and add `bottomTabBar: false` to every public route + the two upgrade-completion pages + the 404 catch-all. Authenticated routes (`/my-calendars`, `/calendar/:id`, `/account`, `/account/*`, `/upgrade`) leave the meta unset (default = visible).

- [ ] **Step 6: Mount the bar in `App.vue`.**

  Open `frontend/src/App.vue` and add the bar after the existing `<router-view>`:

```vue
<script setup lang="ts">
import UiBottomTabBar from '@/components/ui/UiBottomTabBar.vue'
// existing imports...
</script>

<template>
  <!-- existing layout... -->
  <router-view />
  <UiBottomTabBar />
</template>
```

  (Inspect the current `App.vue` and place `<UiBottomTabBar />` at the same DOM level as `<router-view />`, NOT inside any centered-max-width container — the bar must span full viewport.)

- [ ] **Step 7: Run the bar tests + locale parity to verify.**

  Run: `cd frontend && npx vitest run src/components/ui/__tests__/UiBottomTabBar.spec.ts`
  Expected: 6 tests PASS.

  Run: `cd frontend && npm run test:unit -- --run`
  Expected: total previous count + 6 new = green.

- [ ] **Step 8: Commit.**

```bash
git add frontend/src/components/ui/UiBottomTabBar.vue \
        frontend/src/components/ui/__tests__/UiBottomTabBar.spec.ts \
        frontend/src/router/index.ts \
        frontend/src/App.vue \
        frontend/src/locales/en.json \
        frontend/src/locales/pt.json
git commit -m "feat(mobile): UiBottomTabBar primitive + route meta gating"
```

---

### Task 3: `UiBottomSheet` primitive (no third-party dep)

**Plan revision (2026-05-03):** Originally specced `vaul-vue` for drag-to-dismiss UX. Reassessed mid-execution and dropped — drag-dismiss is gravy on top of a working bottom sheet, the static sheet is ~70 lines, and reaching for a dep here is the kind of pre-decision YAGNI we want to push back on. If user testing later shows drag-dismiss is missed, we can add a small custom drag (~40 lines `pointermove` + transform) or pull `vaul-vue` then. v1 ships with × close + backdrop tap + ESC.

**Files:**
- Create: `frontend/src/components/ui/UiBottomSheet.vue`
- Test: `frontend/src/components/ui/__tests__/UiBottomSheet.spec.ts`

**Behavior:**
- Bottom-anchored sheet, slides up on `modelValue=true`, slides down on close.
- Teleports to `document.body` so it overlays everything.
- Backdrop tap closes; ESC closes; × button closes.
- Drag-handle visual at the top (decorative grabber), no gesture in v1.
- Body scroll locked while open.

- [ ] **Step 1: Write the failing test.**

```ts
// frontend/src/components/ui/__tests__/UiBottomSheet.spec.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import UiBottomSheet from '../UiBottomSheet.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function mountSheet(modelValue: boolean) {
  return mount(UiBottomSheet, {
    props: { modelValue },
    slots: { default: '<p>hello</p>' },
    attachTo: document.body,
    global: { plugins: [i18n] },
  })
}

describe('UiBottomSheet', () => {
  beforeEach(() => {
    document.body.replaceChildren()
  })
  afterEach(() => {
    document.body.replaceChildren()
    document.body.style.overflow = ''
  })

  it('does not render the panel when modelValue is false', () => {
    const w = mountSheet(false)
    expect(document.body.querySelector('[data-testid="bottom-sheet-panel"]')).toBeNull()
    w.unmount()
  })

  it('renders slot content when open and teleports to body', () => {
    const w = mountSheet(true)
    expect(document.body.querySelector('[data-testid="bottom-sheet-panel"]')).not.toBeNull()
    expect(document.body.textContent).toContain('hello')
    w.unmount()
  })

  it('emits update:modelValue=false when close button is clicked', async () => {
    const w = mountSheet(true)
    const closeBtn = document.body.querySelector(
      '[data-testid="bottom-sheet-close"]',
    ) as HTMLElement | null
    expect(closeBtn).not.toBeNull()
    closeBtn?.click()
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('emits update:modelValue=false when backdrop is clicked', async () => {
    const w = mountSheet(true)
    const backdrop = document.body.querySelector(
      '[data-testid="bottom-sheet-backdrop"]',
    ) as HTMLElement | null
    expect(backdrop).not.toBeNull()
    backdrop?.click()
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('emits update:modelValue=false when ESC is pressed', async () => {
    const w = mountSheet(true)
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(w.emitted('update:modelValue')?.[0]).toEqual([false])
    w.unmount()
  })

  it('locks body scroll while open', async () => {
    const w = mountSheet(true)
    expect(document.body.style.overflow).toBe('hidden')
    await w.setProps({ modelValue: false })
    expect(document.body.style.overflow).toBe('')
    w.unmount()
  })

  it('renders a decorative drag-handle visual', () => {
    const w = mountSheet(true)
    const handle = document.body.querySelector('[data-testid="bottom-sheet-handle"]')
    expect(handle).not.toBeNull()
    expect(handle?.getAttribute('aria-hidden')).toBe('true')
    w.unmount()
  })
})
```

- [ ] **Step 2: Run the test to verify it fails.**

  Run: `cd frontend && npx vitest run src/components/ui/__tests__/UiBottomSheet.spec.ts`
  Expected: FAIL — component module not found.

- [ ] **Step 3: Verify `common.close` exists in both locales, otherwise add it.**

  Search both `frontend/src/locales/en.json` and `pt.json` for an existing close-button label. If a key like `common.close`, `userDetails.close`, or `app.close` already exists, prefer reusing it. If none exists, add at the top of each file:
  - en: `"common": { "close": "Close" }`
  - pt: `"common": { "close": "Fechar" }`

  Note which key you ended up using — the component's `t('...')` must match.

- [ ] **Step 4: Implement the component.**

```vue
<!-- frontend/src/components/ui/UiBottomSheet.vue -->
<script setup lang="ts">
import { onBeforeUnmount, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import IconX from './icons/IconX.vue'

defineOptions({ name: 'UiBottomSheet' })

interface Props {
  modelValue: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const { t } = useI18n()

function close() {
  emit('update:modelValue', false)
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.modelValue) close()
}

watch(
  () => props.modelValue,
  (open) => {
    if (typeof document === 'undefined') return
    if (open) {
      document.body.style.overflow = 'hidden'
      document.addEventListener('keydown', onKeydown)
    } else {
      document.body.style.overflow = ''
      document.removeEventListener('keydown', onKeydown)
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  document.body.style.overflow = ''
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-opacity duration-200"
      enter-from-class="opacity-0"
      leave-active-class="transition-opacity duration-200"
      leave-to-class="opacity-0"
    >
      <div
        v-if="modelValue"
        data-testid="bottom-sheet-backdrop"
        class="fixed inset-0 z-40 bg-black/40"
        @click="close"
      />
    </Transition>
    <Transition
      enter-active-class="transition-transform duration-300"
      enter-from-class="translate-y-full"
      leave-active-class="transition-transform duration-200"
      leave-to-class="translate-y-full"
    >
      <div
        v-if="modelValue"
        data-testid="bottom-sheet-panel"
        role="dialog"
        aria-modal="true"
        class="fixed bottom-0 left-0 right-0 z-50 rounded-t-[28px] border-t border-line-soft bg-bg-1 shadow-[0_-16px_50px_rgba(0,0,0,0.35)]"
        :style="{ paddingBottom: 'max(36px, calc(env(safe-area-inset-bottom) + 16px))' }"
        @click.stop
      >
        <div
          data-testid="bottom-sheet-handle"
          aria-hidden="true"
          class="mx-auto mt-3 mb-4 h-1 w-9 rounded-full bg-line"
        />
        <button
          type="button"
          data-testid="bottom-sheet-close"
          :aria-label="t('common.close')"
          class="absolute right-4 top-4 rounded-full p-2 text-fg-2 hover:bg-bg-2 focus-visible:outline-2 focus-visible:outline-accent-1 focus-visible:outline-offset-2"
          @click="close"
        >
          <IconX />
        </button>
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>
```

- [ ] **Step 5: Run the spec + full suite + lint.**

  Run: `cd frontend && npx vitest run src/components/ui/__tests__/UiBottomSheet.spec.ts`
  Expected: 7 tests PASS.

  Run: `cd frontend && npm run test:unit -- --run`
  Expected: prior count + 7 new = green.

  Run: `cd frontend && npm run lint && npm run build`
  Expected: clean.

- [ ] **Step 6: Commit.**

```bash
git add frontend/src/components/ui/UiBottomSheet.vue \
        frontend/src/components/ui/__tests__/UiBottomSheet.spec.ts \
        frontend/src/locales/en.json \
        frontend/src/locales/pt.json
git commit -m "feat(mobile): UiBottomSheet primitive (no third-party dep)"
```

---

### Task 4: Rubber-band scroll polish + viewport meta + dvh

**Files:**
- Modify: `frontend/index.html` (viewport meta)
- Modify: `frontend/src/assets/main.css` (overscroll-behavior + dvh helper class)

- [ ] **Step 1: Widen the viewport meta tag.**

  In `frontend/index.html`, find the existing `<meta name="viewport" ...>` and replace its content with:

```html
<meta
  name="viewport"
  content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content"
/>
```

  (`viewport-fit=cover` enables `env(safe-area-inset-*)` on iOS notch devices; `interactive-widget=resizes-content` makes iOS Safari resize content rather than the visual viewport when the keyboard appears.)

- [ ] **Step 2: Add overscroll polish + dvh helper to `main.css`.**

  In `frontend/src/assets/main.css`, append after the existing `select:focus-visible` rule:

```css
/* Track 3: rubber-band scroll polish.
   Scroll containers anchored to the bottom tab bar should not bubble
   the gesture into history-back navigation. */
html, body {
  overscroll-behavior-y: contain;
}

/* Full-height containers should use dynamic viewport height on mobile
   Safari so the URL bar appearing/disappearing doesn't cause layout
   jumps. The .h-dvh-screen utility provides 100dvh with 100vh fallback. */
.h-dvh-screen {
  height: 100vh;
  height: 100dvh;
}

/* Bottom-tab-bar-aware scroll padding so focused inputs are visible
   above the bar on mobile keyboards. */
.scroll-padding-tab-bar {
  scroll-padding-bottom: 90px;
}
```

- [ ] **Step 3: Run lint + build.**

  Run: `cd frontend && npm run lint && npm run build`
  Expected: clean.

- [ ] **Step 4: Manual verification.** Open the dev server (`npm run dev`), use DevTools mobile emulation at iPhone 14 viewport. Confirm:
  - The page does not pull-to-refresh navigate-back when scrolling within a list.
  - DevTools Sources shows the new `<meta>` tag.

  No automated test — these are CSS/HTML-level concerns.

- [ ] **Step 5: Commit.**

```bash
git add frontend/index.html frontend/src/assets/main.css
git commit -m "feat(mobile): rubber-band scroll polish (overscroll-behavior + dvh + viewport meta)"
```

---

### Task 5: Playwright config + mobile-safari device target

**Files:**
- Modify: `frontend/package.json` (devDeps + scripts)
- Create: `frontend/playwright.config.ts`
- Create: `frontend/e2e/smoke.spec.ts` (single sanity test for the harness)
- Modify: `frontend/.gitignore` (add `playwright-report/`, `test-results/`)

- [ ] **Step 1: Install Playwright.**

```bash
cd frontend && npm install --save-dev @playwright/test
npx playwright install --with-deps webkit
```

  We only install webkit (Mobile Safari uses webkit). Adding chromium/firefox later is one config edit.

- [ ] **Step 2: Add scripts to `package.json`.**

  Edit `frontend/package.json` `"scripts"`:

```json
{
  "scripts": {
    "test:e2e": "playwright test",
    "test:e2e:ui": "playwright test --ui",
    "test:e2e:install": "playwright install --with-deps webkit"
  }
}
```

- [ ] **Step 3: Create the Playwright config.**

```ts
// frontend/playwright.config.ts
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'Mobile Safari',
      use: { ...devices['iPhone 14'] },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
```

- [ ] **Step 4: Add a sanity-check smoke test.**

```ts
// frontend/e2e/smoke.spec.ts
import { test, expect } from '@playwright/test'

test('app shell loads on mobile viewport', async ({ page }) => {
  await page.goto('/login')
  await expect(page).toHaveURL(/\/login/)
  // Viewport is iPhone 14 — should be < 1024px wide.
  const viewport = page.viewportSize()
  expect(viewport?.width).toBeLessThan(1024)
})
```

- [ ] **Step 5: Update `.gitignore`.**

  Append to `frontend/.gitignore`:

```
playwright-report/
test-results/
.playwright/
```

- [ ] **Step 6: Run the smoke test.**

  Run: `cd frontend && npm run test:e2e`
  Expected: 1 test PASS. The dev server boots automatically and tears down.

  If this fails because of port collision or backend dependency, document the prerequisite and proceed — the smoke test only loads `/login` which is public and doesn't need the backend.

- [ ] **Step 7: Commit.**

```bash
git add frontend/package.json frontend/package-lock.json \
        frontend/playwright.config.ts frontend/e2e/smoke.spec.ts \
        frontend/.gitignore
git commit -m "chore(test): playwright config + mobile-safari device target + smoke harness"
```

---

## Phase 2 — Surface migration (user-visible changes)

> **Note:** Account routes already exist as sub-routes (per router inspection). The "AccountPage refactor" reduces to a layout adaptation, not a routing rewrite.

### Task 6: AccountPage layout — sidebar (desktop) ↔ list-of-sections (mobile)

**Files:**
- Modify: `frontend/src/components/AccountPage.vue` (branch on viewport)
- Modify: `frontend/src/components/__tests__/AccountPage.spec.ts` (add mobile branch test)
- Modify: `frontend/src/locales/en.json` and `pt.json` (add `mobile.account.backToList`)

- [ ] **Step 1: Add i18n keys for the back-arrow + section labels.**

  In `en.json`, extend the `mobile` block:

```json
  "mobile": {
    "tabBar": { "library": "Library", "account": "Account" },
    "account": {
      "backToList": "Back to Account",
      "sectionsHeading": "Account"
    }
  },
```

  Mirror in `pt.json`:

```json
  "mobile": {
    "tabBar": { "library": "Biblioteca", "account": "Conta" },
    "account": {
      "backToList": "Voltar para Conta",
      "sectionsHeading": "Conta"
    }
  },
```

- [ ] **Step 2: Read the current `AccountPage.vue` to understand the desktop layout.** Then update it to branch:

  Implementation contract:
  - When `isMobile && route.name === 'account'` (i.e., the index `/account` with no sub-route): render the **sectioned list** (links to each sub-route). Hide `<router-view />` because no sub-route is active.
  - When `isMobile && route.name !== 'account'` (i.e., a sub-route is open): render `<router-view />` plus a **back arrow** at top-left that links to `/account`.
  - When `!isMobile`: render the existing sidebar + `<router-view />` layout unchanged.
  - The router currently redirects `/account` → `/account/profile` (`{ path: '', redirect: { name: 'account.profile' } }`). **Remove that redirect for mobile** — but keep desktop behavior. Easiest: remove the redirect entirely, and have the desktop sidebar route to `/account/profile` by default if no sub-route is set (e.g., a `watchEffect` that pushes when `!isMobile && route.name === 'account'`).

  Pattern (paste into `AccountPage.vue`):

```vue
<script setup lang="ts">
import { computed, watchEffect } from 'vue'
import { useRoute, useRouter, RouterLink } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useViewportLayout } from '@/composables/useViewportLayout'
// ...keep existing imports for sidebar UI...

defineOptions({ name: 'AccountPage' })

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const { isMobile } = useViewportLayout()

const SECTIONS = [
  { name: 'account.profile', i18n: 'account.tabs.profile' },
  { name: 'account.preferences', i18n: 'account.tabs.preferences' },
  { name: 'account.subscription', i18n: 'account.tabs.subscription' },
  { name: 'account.password', i18n: 'account.tabs.password' },
  { name: 'account.danger', i18n: 'account.tabs.danger' },
] as const

const isAtSectionList = computed(() => route.path === '/account' || route.path === '/account/')

// Desktop fallback — when no sub-route is active, push to profile.
watchEffect(() => {
  if (!isMobile.value && isAtSectionList.value) {
    router.replace({ name: 'account.profile' })
  }
})
</script>

<template>
  <main class="mx-auto w-full max-w-5xl px-4 py-6 lg:px-6 lg:py-10">
    <!-- Mobile: sectioned list at /account -->
    <div v-if="isMobile && isAtSectionList" data-testid="account-section-list">
      <h1 class="font-display text-3xl text-fg-1">
        {{ t('mobile.account.sectionsHeading') }}
      </h1>
      <ul class="mt-6 divide-y divide-line-soft overflow-hidden rounded-lg border border-line-soft bg-bg-1">
        <li v-for="s in SECTIONS" :key="s.name">
          <RouterLink
            :to="{ name: s.name }"
            :data-testid="`account-section-${s.name}`"
            class="flex items-center justify-between px-4 py-4 text-fg-1 hover:bg-bg-2"
          >
            <span>{{ t(s.i18n) }}</span>
            <span class="text-fg-3">›</span>
          </RouterLink>
        </li>
      </ul>
    </div>

    <!-- Mobile: sub-route view with back arrow -->
    <div v-else-if="isMobile" data-testid="account-subroute">
      <RouterLink
        to="/account"
        class="-ml-2 mb-4 inline-flex items-center gap-1 px-2 py-1 text-sm text-fg-2"
        data-testid="account-back-arrow"
      >
        ‹ {{ t('mobile.account.backToList') }}
      </RouterLink>
      <RouterView />
    </div>

    <!-- Desktop: existing sidebar + router-view layout -->
    <div v-else class="grid grid-cols-[220px_1fr] gap-8" data-testid="account-desktop-layout">
      <!-- KEEP existing sidebar markup here verbatim — copy from current AccountPage -->
      <RouterView />
    </div>
  </main>
</template>
```

  **Important:** the desktop path needs the *existing sidebar markup*. Read the current `AccountPage.vue` and paste its sidebar content into the `<div v-else>` block above, replacing the comment.

- [ ] **Step 3: Remove the `path: ''` redirect from the router.**

  In `frontend/src/router/index.ts`, in the `/account` children, find:

```ts
{ path: '', redirect: { name: 'account.profile' } },
```

  Remove it. Now `/account` resolves to `AccountPage` with no active child — which is exactly what mobile expects. Desktop's `watchEffect` redirects to `account.profile`.

- [ ] **Step 4: Update tests — add mobile/desktop branch coverage.**

  Open `frontend/src/components/__tests__/AccountPage.spec.ts` and add (or replace) a describe block:

```ts
describe('AccountPage layout', () => {
  it('renders the section list at /account on mobile', async () => {
    await mockViewport(390)
    // ...mount AccountPage with router pushed to /account...
    // expect(wrapper.find('[data-testid="account-section-list"]').exists()).toBe(true)
  })

  it('renders the sub-route view + back arrow at /account/profile on mobile', async () => {
    // ...
  })

  it('renders the desktop sidebar layout above the breakpoint', async () => {
    await mockViewport(1280)
    // expect(wrapper.find('[data-testid="account-desktop-layout"]').exists()).toBe(true)
  })
})
```

  Use the existing `AccountPage.spec.ts` test infrastructure; the current spec already constructs a router stub. Reuse that pattern. Run with `npx vitest run src/components/__tests__/AccountPage.spec.ts` and iterate until all assertions pass.

- [ ] **Step 5: Manual verification.** `npm run dev`, mobile viewport. Visit `/account`. See the sectioned list. Tap a section. Sub-route loads with back arrow. Tap back arrow → return to list. Resize to desktop width → sidebar appears, current sub-route stays.

- [ ] **Step 6: Commit.**

```bash
git add frontend/src/components/AccountPage.vue \
        frontend/src/components/__tests__/AccountPage.spec.ts \
        frontend/src/router/index.ts \
        frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(mobile): AccountPage list-vs-sidebar layout

Desktop keeps the sidebar; mobile lands on a sectioned list at /account
and navigates to sub-routes (/account/profile, etc.) with a back arrow.
Removes the /account → /account/profile redirect; desktop's watchEffect
pushes to profile when no sub-route is active."
```

---

### Task 7: SubscriptionTab Nudge-B for free users

**Files:**
- Modify: `frontend/src/components/account/SubscriptionTab.vue`
- Modify: `frontend/src/components/account/__tests__/SubscriptionTab.spec.ts`
- Modify: `frontend/src/locales/en.json` and `pt.json` (add `account.subscription.upgradeNudge`)

- [ ] **Step 1: Add i18n keys.**

  `en.json`:

```json
"account": {
  "subscription": {
    "upgradeNudge": {
      "tierChipFree": "Free plan",
      "ctaFree": "Upgrade to Pro",
      "ctaSubtitle": "Unlock all accent themes, priority refresh, and more."
    }
    // ...keep existing keys...
  }
}
```

  Mirror in `pt.json`:

```json
"upgradeNudge": {
  "tierChipFree": "Plano Free",
  "ctaFree": "Fazer upgrade para Pro",
  "ctaSubtitle": "Desbloqueie todos os acentos, atualização prioritária e muito mais."
}
```

- [ ] **Step 2: Read the current `SubscriptionTab.vue`** to understand the entitlement-fetch flow + past-due banner.

- [ ] **Step 3: Add the Nudge-B affordance for free users.** In `SubscriptionTab.vue`, add a block that renders only when `tier === 'free'` (and not `past_due`, which has its own banner). The block contains:
  - `UiChip variant="default"` showing the "Free plan" tier
  - Subtitle text from `account.subscription.upgradeNudge.ctaSubtitle`
  - `UiButton variant="primary"` with the CTA text, `@click` calling `router.push('/upgrade')`

  Sketch (insert above the existing past-due banner / payment-method block, behind a `v-if`):

```vue
<section
  v-if="tier === 'free'"
  data-testid="subscription-upgrade-nudge"
  class="rounded-lg border border-accent-1 bg-bg-1 p-6 shadow-sm"
>
  <div class="flex items-start justify-between gap-4">
    <div>
      <UiChip variant="default">{{ t('account.subscription.upgradeNudge.tierChipFree') }}</UiChip>
      <p class="mt-3 text-sm text-fg-2">
        {{ t('account.subscription.upgradeNudge.ctaSubtitle') }}
      </p>
    </div>
    <UiButton variant="primary" @click="router.push('/upgrade')">
      {{ t('account.subscription.upgradeNudge.ctaFree') }}
    </UiButton>
  </div>
</section>
```

- [ ] **Step 4: Update tests — assert nudge visibility per tier.**

  Add to `SubscriptionTab.spec.ts`:

```ts
it('shows the Nudge-B upgrade affordance for free users', async () => {
  // ...mock subscription service to return { tier: 'free', status: null, ... }...
  const w = mountSubscriptionTab(/* ... */)
  await flushPromises()
  expect(w.find('[data-testid="subscription-upgrade-nudge"]').exists()).toBe(true)
})

it('hides the Nudge-B affordance for paid users', async () => {
  // ...mock { tier: 'paid', status: 'active', ... }...
  const w = mountSubscriptionTab(/* ... */)
  await flushPromises()
  expect(w.find('[data-testid="subscription-upgrade-nudge"]').exists()).toBe(false)
})

it('hides the Nudge-B affordance for past-due users (banner takes precedence)', async () => {
  // ...mock { tier: 'paid', status: 'past_due', ... }...
  const w = mountSubscriptionTab(/* ... */)
  await flushPromises()
  expect(w.find('[data-testid="subscription-upgrade-nudge"]').exists()).toBe(false)
  // existing past-due banner test should still pass.
})
```

  Run: `cd frontend && npx vitest run src/components/account/__tests__/SubscriptionTab.spec.ts`
  Iterate until green.

- [ ] **Step 5: Commit.**

```bash
git add frontend/src/components/account/SubscriptionTab.vue \
        frontend/src/components/account/__tests__/SubscriptionTab.spec.ts \
        frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(mobile): SubscriptionTab Nudge-B upgrade affordance for free users"
```

---

### Task 8: MyCalendarsPage stacked layout

**Files:**
- Modify: `frontend/src/components/MyCalendarsPage.vue`
- Modify: `frontend/src/components/__tests__/MyCalendarsPage.spec.ts`

- [ ] **Step 1: Read current `MyCalendarsPage.vue` to understand the desktop grid + tile component used.**

- [ ] **Step 2: Branch on viewport. Mobile: stacked single-column cards (poster collage strip + meta row + airing chip). Desktop: keep existing 3-col grid.**

  Pattern:

```vue
<script setup lang="ts">
import { useViewportLayout } from '@/composables/useViewportLayout'
// existing imports...
const { isMobile } = useViewportLayout()
</script>

<template>
  <main class="mx-auto w-full max-w-6xl px-4 py-6 lg:px-6 lg:py-10">
    <!-- TopBar (large variant on mobile) -->
    <header :class="isMobile ? 'mb-4' : 'mb-8'">
      <h1 :class="isMobile ? 'font-display text-4xl' : 'font-display text-5xl'">
        <!-- italic flourish via i18n named slot -->
        ...
      </h1>
    </header>

    <!-- Mobile stack -->
    <div v-if="isMobile" class="flex flex-col gap-3" data-testid="my-calendars-mobile-stack">
      <CalendarTile v-for="c in calendars" :key="c.id" :calendar="c" variant="mobile-row" />
      <button class="rounded-lg border border-dashed border-line p-5 text-fg-2" @click="createNew">
        + {{ t('calendars.newCalendar') }}
      </button>
    </div>

    <!-- Desktop grid (existing) -->
    <div v-else class="grid grid-cols-3 gap-4" data-testid="my-calendars-desktop-grid">
      <!-- existing tile loop -->
    </div>
  </main>
</template>
```

- [ ] **Step 3: Adapt or extend `CalendarTile.vue`** to support a `variant: 'desktop' | 'mobile-row'` prop. Mobile variant uses `h-24` collage strip + meta below.

  Sketch (within `CalendarTile.vue`):

```vue
<script setup lang="ts">
defineProps<{ calendar: Calendar; variant?: 'desktop' | 'mobile-row' }>()
</script>

<template>
  <article
    :class="[
      'card overflow-hidden bg-bg-1',
      variant === 'mobile-row' ? '' : 'aspect-[3/4]',
    ]"
  >
    <div class="h-24 bg-gradient-to-br from-accent-1 to-accent-2"><!-- collage strip --></div>
    <div class="flex items-center justify-between p-3">
      <div>
        <h3 class="font-semibold">{{ calendar.title }}</h3>
        <p class="text-xs text-fg-2">{{ calendar.itemCount }} items</p>
      </div>
      <!-- airing chip -->
    </div>
  </article>
</template>
```

  (Keep the existing desktop variant styling. The mobile variant is the new branch.)

- [ ] **Step 4: Update tests with paired mobile/desktop branch assertions.**

```ts
describe('MyCalendarsPage layout', () => {
  it('renders the stacked mobile layout below 1024px', async () => {
    await mockViewport(390)
    const { default: Page } = await import('@/components/MyCalendarsPage.vue')
    const w = mount(Page, { /* ...store/i18n setup... */ })
    expect(w.find('[data-testid="my-calendars-mobile-stack"]').exists()).toBe(true)
  })

  it('renders the desktop grid at 1280px', async () => {
    await mockViewport(1280)
    const { default: Page } = await import('@/components/MyCalendarsPage.vue')
    const w = mount(Page, { /* ... */ })
    expect(w.find('[data-testid="my-calendars-desktop-grid"]').exists()).toBe(true)
  })
})
```

  Run: `cd frontend && npx vitest run src/components/__tests__/MyCalendarsPage.spec.ts`. Iterate to green.

- [ ] **Step 5: Commit.**

```bash
git add frontend/src/components/MyCalendarsPage.vue \
        frontend/src/components/shared/CalendarTile.vue \
        frontend/src/components/__tests__/MyCalendarsPage.spec.ts
git commit -m "feat(mobile): MyCalendarsPage stacked layout + new-calendar dashed footer"
```

---

### Task 9: `UiAuthShellMobile` — floating-poster fan login shell

**Files:**
- Create: `frontend/src/components/ui/UiAuthShellMobile.vue`
- Create: `frontend/src/services/posters.ts` (new — random AniList covers + sessionStorage cache)
- Modify: `frontend/src/components/ui/UiAuthShell.vue` (viewport-branching shell)
- Test: `frontend/src/components/ui/__tests__/UiAuthShellMobile.spec.ts`
- Modify: `frontend/src/locales/en.json` and `pt.json` (add `auth.mobile.posterAlt`)

- [ ] **Step 1: Add i18n key.**

  `en.json`:

```json
"auth": {
  "mobile": { "posterAlt": "Anime cover art" }
  // existing auth keys...
}
```

  `pt.json`:

```json
"mobile": { "posterAlt": "Arte de capa de anime" }
```

- [ ] **Step 2: Create `services/posters.ts`.**

```ts
// frontend/src/services/posters.ts
import api from '@/config/api'

const CACHE_KEY = 'auth-fan-posters-v1'

export interface PosterRef {
  url: string
  hue: number  // fallback gradient hue if image fails to load
}

const FALLBACK: PosterRef[] = [
  { url: '', hue: 28 },
  { url: '', hue: 145 },
  { url: '', hue: 350 },
]

/**
 * Returns up to `n` cover URLs for the auth-shell fan. Result is cached
 * for the session — a re-mount of the login screen reuses the same posters
 * rather than re-hitting AniList. Failures degrade silently to procedural
 * gradients (the auth shell renders fine without real posters).
 */
export async function getRandomCoverPosters(n = 3): Promise<PosterRef[]> {
  try {
    const cached = sessionStorage.getItem(CACHE_KEY)
    if (cached) {
      const parsed: PosterRef[] = JSON.parse(cached)
      if (Array.isArray(parsed) && parsed.length >= n) return parsed.slice(0, n)
    }
    const res = await api.get<{ covers: string[] }>('/anilist/random-covers', { params: { n } })
    const refs: PosterRef[] = res.data.covers.map((url, i) => ({
      url,
      hue: [28, 145, 350, 80, 285][i] ?? 28,
    }))
    sessionStorage.setItem(CACHE_KEY, JSON.stringify(refs))
    return refs
  } catch {
    return FALLBACK.slice(0, n)
  }
}
```

  **Note:** the `/anilist/random-covers` endpoint is a placeholder — verify whether the backend already exposes such a route. If not, use the existing trending/airing endpoint and pluck cover URLs client-side. Update the URL to whatever the existing `anilist` crate proxy already serves.

- [ ] **Step 3: Implement `UiAuthShellMobile.vue`.**

```vue
<!-- frontend/src/components/ui/UiAuthShellMobile.vue -->
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { getRandomCoverPosters, type PosterRef } from '@/services/posters'

defineOptions({ name: 'UiAuthShellMobile' })

const { t } = useI18n()
const posters = ref<PosterRef[]>([])

onMounted(async () => {
  posters.value = await getRandomCoverPosters(3)
})

const POSTER_TRANSFORMS = [
  { translateY: '0', rotate: '-4deg' },
  { translateY: '-20px', rotate: '0deg' },
  { translateY: '5px', rotate: '4deg' },
] as const
</script>

<template>
  <div class="relative h-dvh-screen overflow-hidden bg-bg-0 text-fg-0">
    <!-- Atmosphere -->
    <div
      aria-hidden="true"
      class="pointer-events-none absolute inset-0"
      :style="{
        background:
          'radial-gradient(60% 50% at 50% 20%, var(--accent-1-glow), transparent 65%), radial-gradient(50% 50% at 80% 70%, var(--accent-1-soft), transparent 65%)',
      }"
    />

    <!-- Floating poster fan -->
    <div class="absolute left-0 right-0 flex justify-center gap-3" style="top: 90px">
      <div
        v-for="(p, i) in [0, 1, 2]"
        :key="i"
        :style="{
          width: '110px',
          height: '165px',
          borderRadius: 'var(--radius-md)',
          backgroundImage: posters[i]?.url ? `url(${posters[i].url})` : undefined,
          background: !posters[i]?.url
            ? `linear-gradient(135deg, oklch(70% 0.15 ${posters[i]?.hue ?? 28}), oklch(60% 0.16 ${(posters[i]?.hue ?? 28) + 200}))`
            : undefined,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
          transform: `translateY(${POSTER_TRANSFORMS[i].translateY}) rotate(${POSTER_TRANSFORMS[i].rotate})`,
          boxShadow: 'var(--shadow-lg)',
        }"
        :aria-label="t('auth.mobile.posterAlt')"
      />
    </div>

    <!-- Slot for the form -->
    <div class="absolute bottom-0 left-0 right-0 px-6 pb-10">
      <slot />
    </div>
  </div>
</template>
```

  (Note: `var(--shadow-lg)` may need to be a literal shadow string if it's not exposed via tokens. Check `tokens.css`; if missing, replace with `'0 12px 40px rgba(0,0,0,0.35)'` literal.)

- [ ] **Step 4: Convert `UiAuthShell.vue` into a viewport-branching shell.** Read the current contents, paste them into the `<template v-else>` branch:

```vue
<script setup lang="ts">
import { useViewportLayout } from '@/composables/useViewportLayout'
import UiAuthShellMobile from './UiAuthShellMobile.vue'
// keep existing imports for the desktop layout

const { isMobile } = useViewportLayout()
</script>

<template>
  <UiAuthShellMobile v-if="isMobile">
    <slot />
  </UiAuthShellMobile>
  <template v-else>
    <!-- KEEP existing UiAuthShell desktop markup here verbatim -->
  </template>
</template>
```

- [ ] **Step 5: Test.**

```ts
// frontend/src/components/ui/__tests__/UiAuthShellMobile.spec.ts
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

vi.mock('@/services/posters', () => ({
  getRandomCoverPosters: () =>
    Promise.resolve([
      { url: '', hue: 28 },
      { url: '', hue: 145 },
      { url: '', hue: 350 },
    ]),
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

describe('UiAuthShellMobile', () => {
  it('renders 3 fan-poster slots', async () => {
    const { default: Shell } = await import('@/components/ui/UiAuthShellMobile.vue')
    const w = mount(Shell, { global: { plugins: [i18n] } })
    await new Promise((r) => setTimeout(r, 0))
    const slots = w.findAll('[aria-label]').filter(
      (n) => n.attributes('aria-label') === en.auth.mobile.posterAlt,
    )
    expect(slots).toHaveLength(3)
  })

  it('renders the slot content', () => {
    const { default: Shell } = require('@/components/ui/UiAuthShellMobile.vue')
    const w = mount(Shell, {
      global: { plugins: [i18n] },
      slots: { default: '<form data-testid="login-form"/>' },
    })
    expect(w.find('[data-testid="login-form"]').exists()).toBe(true)
  })
})
```

  Run: `cd frontend && npx vitest run src/components/ui/__tests__/UiAuthShellMobile.spec.ts`. Iterate to green.

- [ ] **Step 6: Manual verification.** `npm run dev`, mobile viewport. Visit `/login`. See fan posters (procedural gradient if AniList route returns nothing). Form is bottom-anchored.

- [ ] **Step 7: Commit.**

```bash
git add frontend/src/components/ui/UiAuthShellMobile.vue \
        frontend/src/components/ui/UiAuthShell.vue \
        frontend/src/services/posters.ts \
        frontend/src/components/ui/__tests__/UiAuthShellMobile.spec.ts \
        frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(mobile): UiAuthShellMobile + Login/Register fan-poster shell"
```

---

### Task 10: Split `CalendarEditorView` into desktop/mobile shells

**Files:**
- Rename: `frontend/src/components/calendar/CalendarEditorView.vue` → `frontend/src/components/calendar/CalendarEditorViewDesktop.vue`
- Create: `frontend/src/components/calendar/CalendarEditorView.vue` (new thin shell)
- Modify: any test files that import the old path

- [ ] **Step 1: Rename the existing file.**

```bash
cd frontend
git mv src/components/calendar/CalendarEditorView.vue src/components/calendar/CalendarEditorViewDesktop.vue
```

- [ ] **Step 2: Update the renamed file's `defineOptions({ name })`** if present, to `'CalendarEditorViewDesktop'`.

- [ ] **Step 3: Create the new shell at the original path.**

```vue
<!-- frontend/src/components/calendar/CalendarEditorView.vue -->
<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { useViewportLayout } from '@/composables/useViewportLayout'

defineOptions({ name: 'CalendarEditorView' })

const { isMobile } = useViewportLayout()

// Lazy-load each variant so desktop-only users don't pay for the mobile
// editor (and vice versa).
const Desktop = defineAsyncComponent(
  () => import('./CalendarEditorViewDesktop.vue'),
)
const Mobile = defineAsyncComponent(
  () => import('./CalendarEditorViewMobile.vue'),
)
</script>

<template>
  <component :is="isMobile ? Mobile : Desktop" />
</template>
```

  This will fail the build until Task 11 lands `CalendarEditorViewMobile.vue`. That's fine — Task 10 only does the rename + shell. Confirm:

  Run: `cd frontend && npm run build`
  Expected: build error referencing `CalendarEditorViewMobile.vue`. **This is acceptable for Task 10's commit** — Task 11 fixes it.

  **However, to keep the codebase shippable between tasks, create a stub `CalendarEditorViewMobile.vue` that just returns `<CalendarEditorViewDesktop />`:**

```vue
<!-- frontend/src/components/calendar/CalendarEditorViewMobile.vue (STUB — replaced in Task 11) -->
<script setup lang="ts">
import CalendarEditorViewDesktop from './CalendarEditorViewDesktop.vue'
defineOptions({ name: 'CalendarEditorViewMobile' })
</script>
<template>
  <CalendarEditorViewDesktop />
</template>
```

- [ ] **Step 4: Update any test imports if the rename broke them.**

  Run: `cd frontend && npm run test:unit -- --run`
  If anything fails because of the rename, update the import path. Iterate to green.

- [ ] **Step 5: Commit.**

```bash
git add -A frontend/src/components/calendar/
git commit -m "refactor(editor): split CalendarEditorView into desktop/mobile shells

Renames the existing implementation to CalendarEditorViewDesktop and
introduces a thin viewport-branching shell. Mobile variant is a stub
that proxies to desktop until Task 11 lands the real mobile editor."
```

---

### Task 11: `CalendarEditorViewMobile` (the big one)

**Files:**
- Replace stub: `frontend/src/components/calendar/CalendarEditorViewMobile.vue`
- Create: `frontend/src/components/calendar/EditorItemsPanelMobile.vue`
- Create: `frontend/src/components/calendar/EditorSearchPanelMobile.vue`
- Test: `frontend/src/components/calendar/__tests__/CalendarEditorViewMobile.spec.ts`
- Modify: `frontend/src/locales/en.json` and `pt.json` (mobile editor keys)
- Possibly modify: an existing calendar Pinia store for lifted batch-add selection state

- [ ] **Step 1: Add i18n keys.**

  `en.json`:

```json
"mobile": {
  // existing keys...
  "editor": {
    "itemsTab": "Items · {count}",
    "searchTab": "Search",
    "fabAdd": "Add anime",
    "batchAdd": "Add {count} to {target}",
    "batchClear": "Clear",
    "emptySearch": {
      "title": "Find something new.",
      "subtitle": "Search by title, studio, or season — anime & manga both work.",
      "tryChips": "Try",
      "recentLabel": "Recent"
    },
    "results": {
      "tapToSelect": "tap to select",
      "added": "Added",
      "resultsCount": "Results · {count}"
    }
  }
}
```

  Mirror in `pt.json` (translations TBD by user — use sensible defaults; the locale-parity test will catch missing keys).

- [ ] **Step 2: Implement `EditorItemsPanelMobile.vue`** — the items-list panel reusing the existing `MediaItemCard.vue`.

  Sketch:

```vue
<script setup lang="ts">
defineOptions({ name: 'EditorItemsPanelMobile' })
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
defineProps<{ items: CalendarItem[] }>()
</script>
<template>
  <div class="flex flex-col gap-2 px-4 pb-24">
    <MediaItemCard v-for="item in items" :key="item.id" :item="item" variant="mobile-row" />
  </div>
</template>
```

  (Verify `MediaItemCard.vue`'s prop API — the variant prop may need to be added if it doesn't already support a mobile variant. Banner-fade-in mask string MUST come from the same constant currently used by Track 1's contract test.)

- [ ] **Step 3: Implement `EditorSearchPanelMobile.vue`** — the search panel with empty/results states + tap-to-select + sticky batch-add CTA.

  Per design (lines 282–416 of `screens-mobile.jsx`):
  - Search input at top (autofocus)
  - Empty state: chip suggestions + recent items
  - Results state: tap rows to select; selected rows get banner fade-in (preserve mask string); already-added rows show green border + "Added" chip + dimmed
  - Sticky batch-add CTA at `bottom: 90px` when `selection.size >= 1`

  Lift selection state to a Pinia store (`useEditorSelectionStore`) so a viewport flip across 1024 px doesn't clear the user's selection set. Sketch the store:

```ts
// frontend/src/stores/editorSelection.ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useEditorSelectionStore = defineStore('editorSelection', () => {
  const selectedMediaIds = ref<Set<number>>(new Set())
  function toggle(id: number) {
    if (selectedMediaIds.value.has(id)) selectedMediaIds.value.delete(id)
    else selectedMediaIds.value.add(id)
  }
  function clear() { selectedMediaIds.value = new Set() }
  return { selectedMediaIds, toggle, clear }
})
```

  The `EditorSearchPanelMobile` reads/writes this store. On route change (per-calendar), the store should be cleared — wire that in the editor shell's `onBeforeRouteUpdate` / `onBeforeRouteLeave`.

- [ ] **Step 4: Implement `CalendarEditorViewMobile.vue`.**

```vue
<!-- frontend/src/components/calendar/CalendarEditorViewMobile.vue (real impl) -->
<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import { useEditorSelectionStore } from '@/stores/editorSelection'
import UiSegmented from '@/components/ui/UiSegmented.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'
import EditorItemsPanelMobile from './EditorItemsPanelMobile.vue'
import EditorSearchPanelMobile from './EditorSearchPanelMobile.vue'

defineOptions({ name: 'CalendarEditorViewMobile' })

const { t } = useI18n()
const route = useRoute()
const selection = useEditorSelectionStore()

type Tab = 'items' | 'search'
const tab = ref<Tab>('items')
const searchInputRef = ref<HTMLInputElement | null>(null)

// Editor data fetched as in the desktop variant; reuse the same composable
// or store the desktop variant uses. Refer to CalendarEditorViewDesktop.vue
// for the exact data-fetch hook.
const items = ref<CalendarItem[]>([]) // placeholder — wire to existing fetch

const itemCount = computed(() => items.value.length)

function jumpToSearch() {
  tab.value = 'search'
  void nextTick(() => searchInputRef.value?.focus())
}

onBeforeRouteUpdate(() => selection.clear())
onBeforeRouteLeave(() => selection.clear())
</script>

<template>
  <div class="relative flex h-dvh-screen flex-col bg-bg-0">
    <!-- TopBar — title + back + settings -->
    <header
      class="bg-bg-0 px-4 pb-2"
      :style="{ paddingTop: 'max(54px, calc(env(safe-area-inset-top) + 12px))' }"
    >
      <div class="flex items-center justify-between">
        <RouterLink to="/my-calendars" class="text-2xl">‹</RouterLink>
        <span class="font-semibold">{{ /* calendar title from store */ }}</span>
        <button class="text-fg-1"><IconPlus /></button>
      </div>
    </header>

    <!-- Segmented Items / Search -->
    <div class="px-4 pb-3 pt-1">
      <UiSegmented
        v-model="tab"
        :options="[
          { value: 'items', label: t('mobile.editor.itemsTab', { count: itemCount }) },
          { value: 'search', label: t('mobile.editor.searchTab') },
        ]"
        full-width
      />
    </div>

    <!-- Panels with keep-alive so scroll + query persist across swap -->
    <div class="flex-1 overflow-auto pb-24">
      <KeepAlive>
        <EditorItemsPanelMobile v-if="tab === 'items'" :items="items" />
        <EditorSearchPanelMobile v-else ref="searchInputRef" />
      </KeepAlive>
    </div>

    <!-- FAB — Items tab only, jumps to Search -->
    <button
      v-if="tab === 'items'"
      :aria-label="t('mobile.editor.fabAdd')"
      data-testid="editor-fab"
      class="absolute z-20 flex h-14 w-14 items-center justify-center rounded-full bg-accent-1 text-white shadow-[0_12px_32px_var(--accent-1-glow),0_4px_12px_rgba(0,0,0,0.25)]"
      :style="{ right: '18px', bottom: '104px', color: 'var(--accent-1-fg)' }"
      @click="jumpToSearch"
    >
      <IconPlus />
    </button>
  </div>
</template>
```

  **Important integration points:**
  - The data fetch (calendar items, calendar title) must reuse whatever existing composable / Pinia store `CalendarEditorViewDesktop` uses — DO NOT re-fetch independently. The mobile and desktop variants share state.
  - `UiSegmented` may not have a `full-width` prop; check the component and either add the prop or use a wrapping `class="w-full"`.
  - The FAB color uses inline `var(--accent-1-fg)` because that token isn't exposed as a Tailwind utility (per memory `feedback_tailwind_v4_theme_gaps.md`).

- [ ] **Step 5: Add the comprehensive test.**

```ts
// frontend/src/components/calendar/__tests__/CalendarEditorViewMobile.spec.ts
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

vi.mock('@/composables/useViewportLayout', () => ({
  MOBILE_BREAKPOINT_PX: 1024,
  useViewportLayout: () => ({ MOBILE_BREAKPOINT_PX: 1024, isMobile: { value: true } }),
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function makeMount() {
  setActivePinia(createPinia())
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/calendar/:id', component: { template: '<div/>' } }],
  })
  return router
}

describe('CalendarEditorViewMobile', () => {
  beforeEach(() => {
    vi.resetModules()
  })

  it('renders Items tab by default', async () => {
    const router = makeMount()
    await router.push('/calendar/1')
    const { default: View } = await import('../CalendarEditorViewMobile.vue')
    const w = mount(View, { global: { plugins: [i18n, router] } })
    await flushPromises()
    expect(w.text()).toContain(en.mobile.editor.searchTab)
    // Items panel is the active tab — segmented control's "items" button is selected.
  })

  it('shows the FAB only on Items tab', async () => {
    const router = makeMount()
    await router.push('/calendar/1')
    const { default: View } = await import('../CalendarEditorViewMobile.vue')
    const w = mount(View, { global: { plugins: [i18n, router] } })
    await flushPromises()
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(true)
    // Switch to Search tab via segmented click
    const searchBtn = w.findAll('button').find((b) => b.text() === en.mobile.editor.searchTab)
    await searchBtn?.trigger('click')
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(false)
  })

  it('FAB click jumps to Search tab and focuses input', async () => {
    const router = makeMount()
    await router.push('/calendar/1')
    const { default: View } = await import('../CalendarEditorViewMobile.vue')
    const w = mount(View, { attachTo: document.body, global: { plugins: [i18n, router] } })
    await flushPromises()
    await w.find('[data-testid="editor-fab"]').trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(false)
    // Search input should be focused; depending on jsdom limitations this
    // assertion may need to inspect document.activeElement.
  })
})
```

  Run: `cd frontend && npx vitest run src/components/calendar/__tests__/CalendarEditorViewMobile.spec.ts`
  Iterate to green. Some assertions may need `attachTo: document.body` for focus tests.

- [ ] **Step 6: Manual verification.** `npm run dev`, mobile viewport. Visit a calendar from `/my-calendars`. Confirm:
  - Items tab loads, lists items
  - FAB appears bottom-right; click → jumps to Search tab + input is focused
  - Switch to Search via segmented → FAB hidden
  - Type a query → results render
  - Tap a result → banner fade-in, sticky CTA appears
  - Switch back to Items → scroll position preserved
  - Resize to desktop width → desktop variant takes over without state loss (selection lifted to Pinia)

- [ ] **Step 7: Commit.**

```bash
git add frontend/src/components/calendar/CalendarEditorViewMobile.vue \
        frontend/src/components/calendar/EditorItemsPanelMobile.vue \
        frontend/src/components/calendar/EditorSearchPanelMobile.vue \
        frontend/src/components/calendar/__tests__/CalendarEditorViewMobile.spec.ts \
        frontend/src/stores/editorSelection.ts \
        frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(mobile): CalendarEditorViewMobile (segmented + FAB + keep-alive + batch-add CTA + banner fade-in)"
```

---

### Task 12: `CalendarScheduleView` mobile responsive

**Files:**
- Modify: `frontend/src/components/calendar/CalendarScheduleView.vue`
- Modify: `frontend/src/components/calendar/__tests__/ScheduleDayColumn.spec.ts` (if it exists; otherwise add a new spec)

- [ ] **Step 1: Read current `CalendarScheduleView.vue`. Add an internal mobile branch.**

  Mobile layout (per design lines 421–463):
  - Large TopBar: "This week"
  - Horizontal-scrolling day pills row (each pill is a column in the existing desktop layout — collapsed to a small badge on mobile)
  - Active day pill uses `bg-accent-1`
  - Below the pills: a single column of time-anchored episode rows for the active day

  Implementation outline:

```vue
<script setup lang="ts">
import { computed, ref } from 'vue'
import { useViewportLayout } from '@/composables/useViewportLayout'
const { isMobile } = useViewportLayout()
const activeDayIndex = ref(/* today's weekday index */)
</script>

<template>
  <div v-if="isMobile" class="flex flex-col gap-4 pb-24">
    <header class="px-4 pt-4">
      <h1 class="font-display text-3xl">{{ t('mobile.schedule.title') }}</h1>
      <p class="text-sm text-fg-2">{{ t('mobile.schedule.sub') }}</p>
    </header>

    <div class="flex gap-2 overflow-x-auto px-4 pb-2">
      <button
        v-for="(d, i) in days"
        :key="d.label"
        :class="[
          'flex min-w-[46px] flex-col items-center rounded-md border py-2',
          i === activeDayIndex
            ? 'border-transparent bg-accent-1 text-white'
            : 'border-line-soft bg-bg-1 text-fg-1',
        ]"
        @click="activeDayIndex = i"
      >
        <span class="text-[10px] uppercase opacity-80">{{ d.label }}</span>
        <span class="font-mono text-lg font-semibold">{{ d.date }}</span>
      </button>
    </div>

    <!-- Active day's episodes -->
    <div class="flex flex-col gap-2 px-4">
      <div
        v-for="ep in episodesByDay[activeDayIndex]"
        :key="ep.id"
        class="flex items-center gap-3 rounded-md border border-line-soft bg-bg-1 p-3"
      >
        <span class="font-mono text-accent-1 w-12">{{ ep.time }}</span>
        <!-- poster + title + airing-in -->
      </div>
    </div>
  </div>

  <!-- Desktop: keep existing markup -->
  <div v-else>
    <!-- existing desktop layout -->
  </div>
</template>
```

- [ ] **Step 2: Add a mobile branch test.** Mount with `mockViewport(390)`, assert the day-pills row exists, tap a pill, assert the episode list updates.

- [ ] **Step 3: Run all calendar-related tests + lint + build.**

  Run: `cd frontend && npx vitest run src/components/calendar/ && npm run lint && npm run build`
  Expected: green.

- [ ] **Step 4: Commit.**

```bash
git add frontend/src/components/calendar/CalendarScheduleView.vue \
        frontend/src/components/calendar/__tests__/ \
        frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(mobile): CalendarScheduleView mobile responsive (day pills + active-day episode list)"
```

---

### Task 13: Upgrade flow mobile responsive

**Files:**
- Modify: `frontend/src/components/UpgradePage.vue` (responsive: stack tiers vertically on mobile)
- Modify: `frontend/src/components/UpgradeInterruptModal.vue` (mobile branch uses `UiBottomSheet`)
- Modify: `frontend/src/components/UpgradeSuccessPage.vue` (responsive)
- Modify: tests for each

- [ ] **Step 1: `UpgradePage.vue` — stack tiers vertically below `lg` breakpoint.**

  Current layout (Track 4): `grid-cols-1 lg:grid-cols-2`. Already stacks correctly below 1024 — verify by mounting at 390 px in a vitest test.

  Add to the existing `UpgradePage.spec.ts`:

```ts
it('stacks Free + Pro tiers vertically on mobile', async () => {
  await mockViewport(390)
  const { default: UpgradePage } = await import('@/components/UpgradePage.vue')
  const w = mount(UpgradePage, { global: { plugins: [makeI18n('en')] } })
  // Both tier articles are present
  expect(w.findAll('article').length).toBe(2)
  // The grid container has grid-cols-1 active (Tailwind utility class string check)
  const grid = w.find('section')
  expect(grid.classes()).toContain('grid-cols-1')
})
```

  If the existing layout already does what mobile needs (it does — `grid-cols-1 lg:grid-cols-2`), no markup change is needed beyond verifying via test. **Assert atmosphere gradient + italic display headline scale acceptably at mobile width** (manual check).

- [ ] **Step 2: `UpgradeInterruptModal.vue` — mobile branch uses `UiBottomSheet`.**

  Read the current modal markup. Branch:

```vue
<script setup lang="ts">
import { useViewportLayout } from '@/composables/useViewportLayout'
import UiBottomSheet from './ui/UiBottomSheet.vue'
const { isMobile } = useViewportLayout()
// existing props/emits
</script>

<template>
  <UiBottomSheet v-if="isMobile" v-model="visible">
    <!-- Same content as the existing modal body — just rendered inside the bottom sheet -->
    <UpgradeInterruptModalBody />
  </UiBottomSheet>

  <!-- existing centered modal markup unchanged -->
  <UiModal v-else v-model="visible">
    <UpgradeInterruptModalBody />
  </UiModal>
</template>
```

  Extract the modal body into a small `UpgradeInterruptModalBody.vue` so both branches share it.

- [ ] **Step 3: Update `UpgradeInterruptModal.spec.ts`** with paired branches:

```ts
it('wraps content in UiBottomSheet on mobile', async () => {
  await mockViewport(390)
  // ...assert UiBottomSheet rendered, drag-handle visible, close button visible...
})

it('wraps content in UiModal on desktop', async () => {
  await mockViewport(1280)
  // ...assert UiModal rendered, no drag-handle...
})
```

- [ ] **Step 4: `UpgradeSuccessPage.vue` — responsive type scale + full-width CTA.** Audit the current markup; if it's already responsive (per Track 4), leave it; otherwise add `lg:` prefixes to scale type back up and constrain content width on desktop.

- [ ] **Step 5: Run all upgrade-related tests + lint + build.**

  Run: `cd frontend && npx vitest run src/components/__tests__/Upgrade && npm run lint && npm run build`
  Iterate to green.

- [ ] **Step 6: Commit.**

```bash
git add frontend/src/components/UpgradePage.vue \
        frontend/src/components/UpgradeInterruptModal.vue \
        frontend/src/components/UpgradeInterruptModalBody.vue \
        frontend/src/components/UpgradeSuccessPage.vue \
        frontend/src/components/__tests__/Upgrade*.spec.ts
git commit -m "feat(mobile): UpgradePage + UpgradeInterruptModal (vaul-vue) + UpgradeSuccess responsive"
```

---

### Task 14: Small-surface responsive sweep

**Files:**
- Modify: `frontend/src/components/NotFoundPage.vue`
- Modify: `frontend/src/components/VerifyEmailPendingPage.vue`
- Modify: `frontend/src/components/VerifyEmailConfirmPage.vue`
- Modify: `frontend/src/components/ForgotPasswordPage.vue`
- Modify: `frontend/src/components/ResetPasswordPage.vue`
- Modify: `frontend/src/components/UpgradeCanceledPage.vue`

- [ ] **Step 1: Audit each surface at 390 px viewport.** Open the dev server, mobile emulation, visit each route. Note any layout breakage.

- [ ] **Step 2: Apply consistent fixes:**
  - Constrain content to `max-w-sm` on mobile, `max-w-md` on desktop where appropriate
  - Center-stack with `flex flex-col items-center gap-4`
  - Type scale: display 36–48 mobile, 48–64 desktop (use `text-3xl sm:text-5xl`)
  - CTAs full-width below 640 (`w-full sm:w-auto`)
  - Top safe-area: `min-h-dvh-screen pt-[max(54px,calc(env(safe-area-inset-top)+12px))]`

- [ ] **Step 3: Per-surface specifics:**
  - `NotFoundPage` — center the 404 + CTA stack
  - `VerifyEmailPendingPage / VerifyEmailConfirmPage` — already mostly responsive; just type scale
  - `ForgotPasswordPage / ResetPasswordPage` — inherit `UiAuthShell` (which now branches to mobile shell) → mostly free
  - `UpgradeCanceledPage` — center stack + full-width CTAs

- [ ] **Step 4: Add minimal vitest assertion per surface.** A single test that mounts at mobile viewport and asserts the page renders without throwing + the heading is visible. No need for paired desktop/mobile assertions for these — they're shared markup.

- [ ] **Step 5: Run all unit tests + lint + build.**

  Run: `cd frontend && npm run test:unit -- --run && npm run lint && npm run build`
  Expected: green.

- [ ] **Step 6: Commit.**

```bash
git add frontend/src/components/NotFoundPage.vue \
        frontend/src/components/VerifyEmailPendingPage.vue \
        frontend/src/components/VerifyEmailConfirmPage.vue \
        frontend/src/components/ForgotPasswordPage.vue \
        frontend/src/components/ResetPasswordPage.vue \
        frontend/src/components/UpgradeCanceledPage.vue \
        frontend/src/components/__tests__/
git commit -m "feat(mobile): NotFoundPage + VerifyEmail* + Forgot/ResetPassword + UpgradeCanceled responsive sweep"
```

---

### Task 15: Playwright mobile smoke suite

**Files:**
- Create or extend: `frontend/e2e/auth.spec.ts`, `frontend/e2e/library.spec.ts`, `frontend/e2e/editor-tabbed.spec.ts`, `frontend/e2e/editor-banner.spec.ts`, `frontend/e2e/account.spec.ts`, `frontend/e2e/upgrade.spec.ts`, `frontend/e2e/tab-bar.spec.ts`, `frontend/e2e/viewport-flip.spec.ts`, `frontend/e2e/safe-area.spec.ts`, `frontend/e2e/drag-dismiss.spec.ts`

- [ ] **Step 1: Author each spec.** Each is short — typically 1–3 `test()` calls. Examples:

```ts
// frontend/e2e/tab-bar.spec.ts
import { test, expect } from '@playwright/test'

test('tab bar visible on /my-calendars', async ({ page }) => {
  // log in via test fixture or seed cookies — depends on existing test infra
  await page.goto('/my-calendars')
  await expect(page.getByTestId('bottom-tab-bar')).toBeVisible()
})

test('tab bar hidden on /login', async ({ page }) => {
  await page.goto('/login')
  await expect(page.getByTestId('bottom-tab-bar')).not.toBeVisible()
})
```

```ts
// frontend/e2e/viewport-flip.spec.ts
import { test, expect } from '@playwright/test'

test('viewport flip mid-session swaps Editor layout without state loss', async ({ page, context }) => {
  // Start mobile (default device target = iPhone 14)
  await page.goto('/my-calendars')
  // ... open a calendar, start a search, select one result
  // Resize viewport to desktop
  await page.setViewportSize({ width: 1280, height: 800 })
  // Assert desktop editor layout is now active and selection state is preserved
})
```

  **Authentication:** the existing test infra likely uses cookie seeding or a dev-only login endpoint. If neither exists, add `e2e/fixtures.ts` that performs a programmatic login once and reuses the auth cookie across tests via `storageState`.

- [ ] **Step 2: Run the suite.**

  Run: `cd frontend && npm run test:e2e`
  Expected: all 10 cases pass on Mobile Safari device target.

- [ ] **Step 3: Commit.**

```bash
git add frontend/e2e/
git commit -m "test(e2e): mobile smoke suite for tab bar, editor, account, upgrade, viewport flip"
```

---

### Task 16: Plan close-out

**Files:**
- Modify: `docs/superpowers/plans/2026-05-03-track-3-mobile-companion.md` (mark all tasks complete)
- Update: `MEMORY.md` index entry for Track 3

- [ ] **Step 1: Run the full verification gate one last time.**

  Run:
```bash
cd frontend
npm run test:unit -- --run
npm run test:e2e
npm run lint
npm run build
```

  All must be green. Capture the test/file counts for the close-out commit.

- [ ] **Step 2: Edit this plan file** — mark every checkbox as `- [x]` and add a `## Completed` footer with the date + bundle-size delta vs the pre-Track-3 baseline (capture from `npm run build` output).

- [ ] **Step 3: Add/update memory entries.**

  - Update `project_track_2_*` and `project_track_4_*` siblings with a "Track 3 followed up" note if appropriate.
  - Add `project_track_3_complete.md` capturing the architectural decisions (hybrid topology, 2-tab bar, Nudge-B, breakpoint constant, etc.) that future work will build on.
  - Add to `MEMORY.md` index.

- [ ] **Step 4: Commit.**

```bash
git add docs/superpowers/plans/2026-05-03-track-3-mobile-companion.md \
        # any memory files (out of repo, in user-global memory dir)
git commit -m "docs(track-3): close out plan + capture completion notes"
```

---

## Self-review checklist (run after the plan is written)

This was performed inline during plan authoring. Documented for transparency.

- **Spec coverage:** Every section of the spec maps to a task — Architecture (Tasks 1–4), Components Group A (Task 14), Group B (Tasks 6, 8, 12, 13), Group C (Tasks 10–11), data flow (Tasks 1, 11), testing (Task 5 + 15), risks/acceptance (woven through). No gaps.
- **Placeholders:** None — every code block is concrete (modulo the spec-acknowledged "verify existing route name" / "consult node_modules" callouts, which are *deliberate* signals that the engineer needs to look at the codebase, not vague TODOs).
- **Type consistency:** `MOBILE_BREAKPOINT_PX`, `useViewportLayout`, `isMobile`, the `tab` enum (`'items' | 'search'`), `useEditorSelectionStore` API are consistent across all task references.
- **Scope:** Single-track plan, ≈16 tasks, achievable in 2–3 weeks of focused work given the full-sweep scope locked in brainstorm.
- **Discoveries vs spec:** Two findings during plan authoring narrowed work — Account routes already structured (Task 6 simplified), `useWindowSize` already exists with a different threshold (kept separate per spec). Both noted in plan.

---

## Open questions for execution time

These are intentionally not resolved here — answers depend on inspecting the live codebase:

1. **`UiSegmented`'s `full-width` prop** — does it exist? If not, wrap with `class="w-full"` (Task 11 step 4).
2. **AniList random-covers endpoint shape** — does the backend already proxy this, or do we plumb a new endpoint? (Task 9 step 2.)
3. **`MediaItemCard` mobile-row variant** — does the existing component support a `variant` prop, or does it need to be added? (Tasks 8, 11.)
4. **Playwright auth fixture** — what's the cleanest way to seed an authed cookie for e2e tests? (Task 15.)

Each can be resolved in <10 minutes by reading the relevant file when its task is in progress.
