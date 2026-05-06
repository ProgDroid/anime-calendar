import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, h } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter, RouterView } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'

// Mock useViewportLayout so the component doesn't depend on window.innerWidth
vi.mock('@/composables/useViewportLayout', () => ({
  MOBILE_BREAKPOINT_PX: 1024,
  useViewportLayout: () => ({ MOBILE_BREAKPOINT_PX: 1024, isMobile: { value: true } }),
}))

// Mock api to avoid real HTTP calls
vi.mock('@/config/api', () => ({
  default: {
    get: vi.fn().mockResolvedValue({ data: { items: [] } }),
    put: vi.fn().mockResolvedValue({ data: { name: 'Test' } }),
    post: vi.fn().mockResolvedValue({ data: { affected: true } }),
    delete: vi.fn().mockResolvedValue({}),
  },
}))

// Mock calendars service so per-item calls don't hit the network
vi.mock('@/services/calendars', () => ({
  addItem: vi.fn().mockResolvedValue({ affected: true }),
  removeItem: vi.fn().mockResolvedValue(undefined),
  fetchScheduleForCalendar: vi.fn().mockResolvedValue({}),
}))

// Mock subscription service so getMySubscription doesn't consume api.get slots
vi.mock('@/services/subscription', () => ({
  getMySubscription: vi.fn().mockResolvedValue({ tier: 'free' }),
}))

// Mock usageStore so refresh() doesn't consume api.get slots
vi.mock('@/stores/usageStore', () => ({
  useUsageStore: () => ({
    loaded: true,
    showCount: null,
    refresh: vi.fn().mockResolvedValue(undefined),
  }),
}))

// Mock toastService
vi.mock('@/services/toastService', () => ({
  toastService: { success: vi.fn(), error: vi.fn() },
}))

// Mock userSettingsStore to avoid fetch
vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: () => ({
    fetchSettings: vi.fn().mockResolvedValue({ title_language_preference: 'English' }),
  }),
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en, pt } })

function makeRouter(viewComponent: unknown) {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      // Mount the View as the matched route's component. onBeforeRouteUpdate /
      // onBeforeRouteLeave only have an active route record when the component
      // is rendered as a child of <RouterView>, so we mount RouterView and
      // route to this entry rather than mounting the View directly.
      { path: '/calendar/:id', component: viewComponent as never },
      { path: '/my-calendars', component: { template: '<div/>' } },
    ],
  })
}

describe('CalendarEditorViewMobile', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetModules()
  })

  afterEach(() => {
    vi.clearAllMocks()
  })

  async function mountView(calendarId = 'new') {
    const { default: View } = await import('../CalendarEditorViewMobile.vue')
    const router = makeRouter(View)
    await router.push(`/calendar/${calendarId}`)
    await router.isReady()
    const Host = defineComponent({
      components: { RouterView },
      render: () => h(RouterView),
    })
    const wrapper = mount(Host, {
      attachTo: document.body,
      global: { plugins: [i18n, router] },
    })
    await flushPromises()
    return wrapper
  }

  it('renders the Items tab as the default active panel', async () => {
    const w = await mountView()
    // The segmented control should show both tabs
    expect(w.text()).toContain(en.mobile.editor.searchTab)
    // Items panel should be visible (no results empty state shown)
    expect(w.find('[data-testid="items-empty-state"]').exists()).toBe(true)
    w.unmount()
  })

  it('shows the FAB when on the Items tab', async () => {
    const w = await mountView()
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(true)
    w.unmount()
  })

  it('hides the FAB when on the Search tab', async () => {
    const w = await mountView()
    // Click the search tab button
    const tabBar = w.find('[data-testid="editor-tab-bar"]')
    const searchBtn = tabBar.findAll('button').find((b) => b.text() === en.mobile.editor.searchTab)
    expect(searchBtn).toBeDefined()
    await searchBtn!.trigger('click')
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(false)
    w.unmount()
  })

  it('FAB click switches to the Search tab', async () => {
    const w = await mountView()
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(true)
    await w.find('[data-testid="editor-fab"]').trigger('click')
    await flushPromises()
    // FAB should now be hidden (search tab active)
    expect(w.find('[data-testid="editor-fab"]').exists()).toBe(false)
    // Search empty state should be visible
    expect(w.find('[data-testid="search-empty-state"]').exists()).toBe(true)
    w.unmount()
  })

  it('FAB click focuses the search input', async () => {
    const w = await mountView()
    await w.find('[data-testid="editor-fab"]').trigger('click')
    await flushPromises()
    const input = w.find('[data-testid="search-input-mobile"] input').element as HTMLInputElement
    expect(document.activeElement).toBe(input)
    w.unmount()
  })

  it('renders the tab bar with Items and Search options', async () => {
    const w = await mountView()
    const tabBar = w.find('[data-testid="editor-tab-bar"]')
    expect(tabBar.exists()).toBe(true)
    const buttons = tabBar.findAll('button')
    expect(buttons.length).toBeGreaterThanOrEqual(2)
    const labels = buttons.map((b) => b.text())
    // Items tab has count interpolated: "Items · 0"
    expect(labels.some((l) => l.includes('Items'))).toBe(true)
    expect(labels.some((l) => l === en.mobile.editor.searchTab)).toBe(true)
    w.unmount()
  })

  it('shows CalendarSettingsForm at the top', async () => {
    const w = await mountView()
    // Settings form submit button should be present
    expect(w.find('[data-testid="submit-btn"]').exists()).toBe(true)
    w.unmount()
  })

  it('renders search input when on Search tab', async () => {
    const w = await mountView()
    const tabBar = w.find('[data-testid="editor-tab-bar"]')
    const searchBtn = tabBar.findAll('button').find((b) => b.text() === en.mobile.editor.searchTab)
    await searchBtn!.trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="search-input-mobile"]').exists()).toBe(true)
    w.unmount()
  })

  it('batch-add bar is hidden when no items are selected', async () => {
    const w = await mountView()
    const tabBar = w.find('[data-testid="editor-tab-bar"]')
    const searchBtn = tabBar.findAll('button').find((b) => b.text() === en.mobile.editor.searchTab)
    await searchBtn!.trigger('click')
    await flushPromises()
    expect(w.find('[data-testid="batch-add-bar"]').exists()).toBe(false)
    w.unmount()
  })

  it('submit with no item diff calls only api.put (no addItem/removeItem)', async () => {
    const { default: api } = await import('@/config/api')
    const { addItem, removeItem } = await import('@/services/calendars')

    const fakeItem = {
      id: 5,
      title: { english: 'Show A', romaji: '', native: '' },
      cover_image: null,
      airing_schedule: [],
    }
    vi.mocked(api.get).mockResolvedValueOnce({
      data: {
        id: 42,
        name: 'My Cal',
        language: 'english',
        event_style: 'timed',
        items: [fakeItem],
      },
    })

    const w = await mountView('42')

    // Fill required name field via the form input
    const nameInput = w.find('[data-testid="calendar-name-input"] input')
    if (nameInput.exists()) {
      await nameInput.setValue('My Cal')
    }
    // Trigger submit
    const submitBtn = w.find('[data-testid="submit-btn"]')
    await submitBtn.trigger('click')
    await flushPromises()

    // No diff: item 5 loaded and still present → no per-item calls
    expect(addItem).not.toHaveBeenCalled()
    expect(removeItem).not.toHaveBeenCalled()
    // Meta PUT should have been called once
    expect(api.put).toHaveBeenCalledTimes(1)
    w.unmount()
  })

  it('editor 403 on meta PUT after item ops shows success toast', async () => {
    const { default: api } = await import('@/config/api')
    const { addItem: mockAdd } = await import('@/services/calendars')
    const { toastService } = await import('@/services/toastService')

    // Load calendar with 1 item; load succeeds
    const fakeItem = {
      id: 5,
      title: { english: 'Show A', romaji: '', native: '' },
      cover_image: null,
      airing_schedule: [],
    }
    vi.mocked(api.get).mockResolvedValueOnce({
      data: {
        id: 42,
        name: 'My Cal',
        language: 'english',
        event_style: 'timed',
        items: [fakeItem],
      },
    })

    // PUT rejects with 403
    const err403 = Object.assign(new Error('Forbidden'), {
      isAxiosError: true,
      response: { status: 403 },
    })
    vi.mocked(api.put).mockRejectedValueOnce(err403)

    // addItem resolves (adding a new item id=7 to trigger the diff path)
    vi.mocked(mockAdd).mockResolvedValue({ affected: true })

    const w = await mountView('42')

    // Programmatically add item 7 so there is a diff to trigger per-item call
    // We do this by emitting add-selected on the search panel component
    // Since it's on the items tab, emit via the internal component chain.
    // Simpler: find the component instance and call its exposed handler.
    // addItemFromSearch is not exposed — use the @add-selected emit pathway.
    // Switch to search tab first to reach the panel
    const tabBar = w.find('[data-testid="editor-tab-bar"]')
    const searchBtn = tabBar.findAll('button').find((b) => b.text() === en.mobile.editor.searchTab)
    await searchBtn!.trigger('click')
    await flushPromises()

    const searchPanel = w.findComponent({ name: 'EditorSearchPanelMobile' })
    const newItem = {
      id: 7,
      title: { english: 'Show B', romaji: '', native: '' },
      cover_image: null,
      airing_schedule: [],
    }
    await searchPanel.vm.$emit('add-selected', [newItem])
    await flushPromises()

    // Now submit
    const submitBtn = w.find('[data-testid="submit-btn"]')
    await submitBtn.trigger('click')
    await flushPromises()

    // Toast success should be called despite 403 on PUT
    expect(toastService.success).toHaveBeenCalled()
    w.unmount()
  })
})
