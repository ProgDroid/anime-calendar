import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { MockEventSource } from '@/__tests__/setup'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import CalendarSettingsForm from '../CalendarSettingsForm.vue'

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
  useAuthStore: () => ({ user: 'testuser', userId: 1 }),
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
    sessionStorage.clear()
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

  it('item_added toast shows the actor display name, not the numeric id', async () => {
    const { wrapper } = await mountView()
    const { toastService } = await import('@/services/toastService')
    MockEventSource.lastInstance.emit(
      'message',
      JSON.stringify({
        type: 'item_added',
        media_id: 7,
        actor: '2',
        display: 'Alice',
        v: 1,
        at: '2026-01-01T00:00:00Z',
      }),
    )
    await flushPromises()
    expect(toastService.success).toHaveBeenCalledWith('Alice added an item')
    wrapper.unmount()
  })

  it('overlays a saved draft for the matching calendar id', async () => {
    sessionStorage.setItem(
      'calendarDraft:42',
      JSON.stringify({
        calendarName: 'Draft Name',
        calendarLanguage: 'english',
        calendarEventStyle: 'timed',
        itemsInCalendar: [],
      }),
    )
    const { wrapper } = await mountView('42')
    const form = wrapper.findComponent(CalendarSettingsForm)
    expect(form.props('name')).toBe('Draft Name')
    wrapper.unmount()
  })

  it('ignores a draft saved under a different calendar id', async () => {
    sessionStorage.setItem(
      'calendarDraft:99',
      JSON.stringify({
        calendarName: 'Other Draft',
        calendarLanguage: 'english',
        calendarEventStyle: 'timed',
        itemsInCalendar: [],
      }),
    )
    const { wrapper } = await mountView('42')
    const form = wrapper.findComponent(CalendarSettingsForm)
    // Server value ('Test Cal' from the api mock) — the 99-scoped draft is not applied.
    expect(form.props('name')).toBe('Test Cal')
    wrapper.unmount()
  })
})
