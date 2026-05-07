/**
 * Regression test for C-2: SSE actor filter must compare frame.actor
 * against authStore.userId (numeric id string), not authStore.user (username).
 *
 * Prior to this fix frame.actor was compared to the username so self-echoed
 * SSE events were never filtered, causing duplicate toasts and clobbered
 * optimistic state.
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { MockEventSource } from '@/__tests__/setup'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

// ── Mocks ──────────────────────────────────────────────────────────────────

vi.mock('@/config/api', () => ({
  default: {
    get: vi.fn().mockResolvedValue({
      data: {
        id: 42,
        name: 'Test Cal',
        language: 'english',
        event_style: 'timed',
        meta_version: 1,
        items: [],
        user_id: 42,
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
    }),
    put: vi.fn().mockResolvedValue({ data: { name: 'Test Cal' } }),
    post: vi.fn().mockResolvedValue({}),
    delete: vi.fn().mockResolvedValue({}),
  },
}))

vi.mock('@/services/calendars', () => ({
  addItem: vi.fn().mockResolvedValue({ affected: true }),
  removeItem: vi.fn().mockResolvedValue(undefined),
  fetchScheduleForCalendar: vi.fn().mockResolvedValue({}),
}))

vi.mock('@/services/subscription', () => ({
  getMySubscription: vi.fn().mockResolvedValue({ tier: 'free' }),
}))

vi.mock('@/stores/usageStore', () => ({
  useUsageStore: () => ({ loaded: true, showCount: null, refresh: vi.fn() }),
}))

vi.mock('@/services/toastService', () => ({
  toastService: { success: vi.fn(), error: vi.fn() },
}))

vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: () => ({
    fetchSettings: vi.fn().mockResolvedValue({ user_id: 42, title_language_preference: 'English' }),
  }),
}))

// authStore.userId = 42, authStore.user = 'testuser'
// frame.actor will be "42" (numeric id string) after T1/T2.
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ user: 'testuser', userId: 42 }),
}))

// ── Helpers ────────────────────────────────────────────────────────────────

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function makeRouter(viewComponent: unknown) {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/calendar/:id', component: viewComponent as never },
      { path: '/my-calendars', component: { template: '<div/>' } },
    ],
  })
}

async function mountDesktop(calendarId = '42') {
  const { default: View } = await import('@/components/calendar/CalendarEditorViewDesktop.vue')
  const router = makeRouter(View)
  await router.push(`/calendar/${calendarId}`)
  await router.isReady()
  const Host = defineComponent({ components: { RouterView }, render: () => h(RouterView) })
  const wrapper = mount(Host, {
    attachTo: document.body,
    global: { plugins: [i18n, router] },
  })
  await flushPromises()
  return wrapper
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('SSE actor filter (C-2 regression)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetModules()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('suppresses item_added toast when frame.actor matches own userId string', async () => {
    const { toastService } = await import('@/services/toastService')
    const w = await mountDesktop()

    // actor === String(authStore.userId) === "42" → should be filtered
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_added', media_id: 99, actor: '42' }),
    )
    await flushPromises()

    expect(toastService.success).not.toHaveBeenCalled()
    w.unmount()
  })

  it('shows item_added toast when frame.actor is a different user id', async () => {
    const { toastService } = await import('@/services/toastService')
    const w = await mountDesktop()

    // actor === "99" !== "42" → should fire toast
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_added', media_id: 99, actor: '99' }),
    )
    await flushPromises()

    expect(toastService.success).toHaveBeenCalledTimes(1)
    w.unmount()
  })

  it('suppresses item_removed toast when frame.actor matches own userId string', async () => {
    const { toastService } = await import('@/services/toastService')
    const w = await mountDesktop()

    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_removed', media_id: 5, actor: '42' }),
    )
    await flushPromises()

    expect(toastService.success).not.toHaveBeenCalled()
    w.unmount()
  })

  it('shows item_removed toast when frame.actor is a different user id', async () => {
    const { toastService } = await import('@/services/toastService')
    const w = await mountDesktop()

    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_removed', media_id: 5, actor: '99' }),
    )
    await flushPromises()

    expect(toastService.success).toHaveBeenCalledTimes(1)
    w.unmount()
  })

  it('does NOT filter based on username (C-2 regression guard)', async () => {
    // Before the fix the comparison was frame.actor !== authStore.user ("testuser").
    // A frame with actor "testuser" would have been suppressed incorrectly — but
    // after T1 actors are numeric ids, so "testuser" should NOT be filtered.
    const { toastService } = await import('@/services/toastService')
    const w = await mountDesktop()

    // "testuser" !== "42" → toast must fire (username is no longer the filter key)
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'item_added', media_id: 99, actor: 'testuser' }),
    )
    await flushPromises()

    expect(toastService.success).toHaveBeenCalledTimes(1)
    w.unmount()
  })

  // Documented safe default: when authStore.userId is null (uninitialized auth state),
  // String(null) === "null" — which never equals a real numeric actor string. The
  // filter therefore lets events through, surfacing the toast rather than silently
  // dropping it. We don't unit-test this branch because:
  //   1. It's a 1-line JavaScript invariant (String(null) === "null") that won't
  //      realistically regress.
  //   2. The 5 tests above already cover both sides of the comparison contract.
  //   3. Swapping the auth store mid-test (vi.doMock after top-level vi.mock) proved
  //      unstable in Vitest watch mode and added no coverage value.
})
