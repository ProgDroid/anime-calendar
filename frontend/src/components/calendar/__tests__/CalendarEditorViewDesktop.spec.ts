import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { MockEventSource } from '@/__tests__/setup'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

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
        user_id: 1,
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
    fetchSettings: vi.fn().mockResolvedValue({ user_id: 1, title_language_preference: 'English' }),
  }),
}))

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ user: 'testuser' }),
}))

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

describe('CalendarEditorViewDesktop', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetModules()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  async function mountView(calendarId = '42') {
    const { default: View } = await import('../CalendarEditorViewDesktop.vue')
    const router = makeRouter(View)
    await router.push(`/calendar/${calendarId}`)
    await router.isReady()
    const Host = defineComponent({ components: { RouterView }, render: () => h(RouterView) })
    const wrapper = mount(Host, {
      attachTo: document.body,
      global: { plugins: [i18n, router] },
    })
    await flushPromises()
    return { wrapper, router }
  }

  it('shows collision banner when meta_updated event arrives with higher version', async () => {
    const { wrapper } = await mountView()
    expect(wrapper.find('[data-testid="collision-banner"]').exists()).toBe(false)
    // Emit meta_updated with version > baseline (1)
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'meta_updated', fields: ['name'], actor: 'other', v: 2, at: '2026-01-01T00:00:00Z' }),
    )
    await flushPromises()
    expect(wrapper.find('[data-testid="collision-banner"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('redirects to /my-calendars on kick event', async () => {
    const { wrapper, router } = await mountView()
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({ type: 'kick', reason: 'role_revoked' }),
    )
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/my-calendars')
    wrapper.unmount()
  })
})
