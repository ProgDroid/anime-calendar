import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
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
  },
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

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/calendar/:id', component: { template: '<div/>' } },
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
    const router = makeRouter()
    await router.push(`/calendar/${calendarId}`)
    await router.isReady()
    const { default: View } = await import('../CalendarEditorViewMobile.vue')
    const wrapper = mount(View, {
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
})
