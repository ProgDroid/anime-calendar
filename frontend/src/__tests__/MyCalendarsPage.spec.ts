import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import MyCalendarsPage from '@/components/MyCalendarsPage.vue'
import en from '@/locales/en.json'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), delete: vi.fn(), post: vi.fn(), put: vi.fn() },
  getApiUrl: (path: string) => `http://localhost/api${path}`,
}))

const openUpgradeModalMock = vi.fn()
vi.mock('@/composables/useUpgradeInterrupt', () => ({
  useUpgradeInterrupt: () => ({ openUpgradeModal: openUpgradeModalMock }),
}))

vi.mock('@/services/subscription', () => ({
  getMySubscription: vi.fn(),
}))

import api from '@/config/api'
import { getMySubscription } from '@/services/subscription'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: '/:p*', component: MyCalendarsPage }],
})

const mockCalendars = [
  {
    id: 1,
    name: 'My Anime Calendar',
    item_count: 5,
    airing_count: 0,
    subscription_token: 'token123',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z',
    recent_item_ids: [],
  },
  {
    id: 2,
    name: 'Second Calendar',
    item_count: 2,
    airing_count: 0,
    subscription_token: 'token456',
    created_at: '2026-01-03T00:00:00Z',
    updated_at: '2026-01-03T00:00:00Z',
    recent_item_ids: [],
  },
]

const mockPagination = { page: 1, page_size: 6, total: 2, total_pages: 1 }

function mountPage() {
  return mount(MyCalendarsPage, {
    global: { plugins: [i18n, router, createPinia()] },
    attachTo: document.body,
  })
}

describe('MyCalendarsPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage')) return { data: { shows: 0, calendars: 2 } }
      return { data: { data: mockCalendars, pagination: mockPagination } }
    })
    vi.mocked(getMySubscription).mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    openUpgradeModalMock.mockClear()
  })

  it('renders the page title', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain(en.calendars.title)
    wrapper.unmount()
  })

  it('shows calendars after successful load', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain('My Anime Calendar')
    expect(wrapper.text()).toContain('Second Calendar')
    wrapper.unmount()
  })

  it('shows empty state when no calendars returned', async () => {
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage')) return { data: { shows: 0, calendars: 0 } }
      return { data: { data: [], pagination: { ...mockPagination, total: 0 } } }
    })
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="my-calendars-empty"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.calendars.notFound)
    wrapper.unmount()
  })

  it('shows error message when API call fails', async () => {
    vi.mocked(api.get).mockRejectedValue(new Error('Network error'))
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="my-calendars-error"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.calendars.loadFailed)
    wrapper.unmount()
  })

  it('opens confirm modal when tile delete is clicked', async () => {
    const wrapper = mountPage()
    await flushPromises()
    const tiles = wrapper.findAll('[data-testid="calendar-tile-delete"]')
    expect(tiles.length).toBeGreaterThan(0)
    await tiles[0]!.trigger('click')
    expect(document.body.querySelector('[data-testid="modal-box"]')).not.toBeNull()
    expect(document.body.textContent).toContain(en.calendars.deleteConfirmTitle)
    wrapper.unmount()
  })

  it('calls DELETE and reloads on confirm', async () => {
    vi.mocked(api.delete).mockResolvedValue({})
    const wrapper = mountPage()
    await flushPromises()

    await wrapper.findAll('[data-testid="calendar-tile-delete"]')[0]!.trigger('click')
    await wrapper.vm.$nextTick()
    ;(document.body.querySelector('[data-testid="confirm-btn"]') as HTMLElement).click()
    await flushPromises()

    expect(api.delete).toHaveBeenCalledWith('/calendars/1')
    // initial /calendars + reload /calendars; /items calls allowed
    const calendarCalls = vi.mocked(api.get).mock.calls.filter(([u]) =>
      typeof u === 'string' && u.startsWith('/calendars'),
    )
    expect(calendarCalls.length).toBe(2)
    wrapper.unmount()
  })

  it('navigates to /calendar/new on the new-calendar button', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('[data-testid="my-calendars-new"]').trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/new')
    wrapper.unmount()
  })

  it('navigates to /calendar/:id when tile body is clicked', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.findAll('[data-testid="calendar-tile-body"]')[0]!.trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/1')
    wrapper.unmount()
  })

  it('navigates to /calendar/:id on Edit button', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.findAll('[data-testid="calendar-tile-edit"]')[0]!.trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/1')
    wrapper.unmount()
  })

  it('hides the airing stat when no calendars are airing', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="my-calendars-airing"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('renders the summed airing stat when calendars have airing items', async () => {
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage')) return { data: { shows: 0, calendars: 2 } }
      return {
        data: {
          data: [
            { ...mockCalendars[0]!, airing_count: 2 },
            { ...mockCalendars[1]!, airing_count: 5 },
          ],
          pagination: mockPagination,
        },
      }
    })
    const wrapper = mountPage()
    await flushPromises()
    const stat = wrapper.find('[data-testid="my-calendars-airing"]')
    expect(stat.exists()).toBe(true)
    expect(stat.text()).toContain('7')
    wrapper.unmount()
  })
})

describe('MyCalendarsPage mobile layout', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage')) return { data: { shows: 0, calendars: 2 } }
      return { data: { data: mockCalendars, pagination: mockPagination } }
    })
    vi.mocked(getMySubscription).mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
    openUpgradeModalMock.mockClear()
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders the desktop grid above the breakpoint', async () => {
    await mockViewport(1280)
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="my-calendars"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="my-calendars-mobile-stack"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="my-calendars-mobile-create"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('renders the mobile stack below the breakpoint', async () => {
    await mockViewport(390)
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="my-calendars-mobile-stack"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="my-calendars-mobile-create"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="my-calendars"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('mobile stack lists every calendar from the store', async () => {
    await mockViewport(390)
    const wrapper = mountPage()
    await flushPromises()
    const tiles = wrapper
      .find('[data-testid="my-calendars-mobile-stack"]')
      .findAll('[data-testid="calendar-tile"]')
    expect(tiles).toHaveLength(mockCalendars.length)
    wrapper.unmount()
  })

  it('mobile create CTA navigates to /calendar/new', async () => {
    await mockViewport(390)
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('[data-testid="my-calendars-mobile-create"]').trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/new')
    wrapper.unmount()
  })
})

describe('MyCalendarsPage free-tier cap UX', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    openUpgradeModalMock.mockClear()
  })

  function withFree(calendarCount: number) {
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage'))
        return { data: { shows: 0, calendars: calendarCount } }
      return { data: { data: mockCalendars, pagination: mockPagination } }
    })
    vi.mocked(getMySubscription).mockResolvedValue({
      tier: 'free',
      status: null,
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
  }

  function withPaid() {
    vi.mocked(api.get).mockImplementation(async (url: string) => {
      if (url.startsWith('/items')) return { data: [] }
      if (url.startsWith('/account/usage')) return { data: { shows: 99, calendars: 99 } }
      return { data: { data: mockCalendars, pagination: mockPagination } }
    })
    vi.mocked(getMySubscription).mockResolvedValue({
      tier: 'paid',
      status: 'active',
      current_period_end: null,
      cancel_at_period_end: false,
      trial_end: null,
    })
  }

  it('shows the calendar counter chip for a free user', async () => {
    withFree(2)
    const wrapper = mountPage()
    await flushPromises()
    const chip = wrapper.find('[data-testid="calendar-counter-chip"]')
    expect(chip.exists()).toBe(true)
    expect(chip.text()).toContain('2')
    expect(chip.text()).toContain('3')
    wrapper.unmount()
  })

  it('hides the calendar counter chip for a paid user', async () => {
    withPaid()
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('[data-testid="calendar-counter-chip"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('marks the new-calendar button as aria-disabled when free user is at the calendar cap', async () => {
    withFree(3)
    const wrapper = mountPage()
    await flushPromises()
    const btn = wrapper.find('[data-testid="my-calendars-new"]')
    expect(btn.attributes('aria-disabled')).toBe('true')
    wrapper.unmount()
  })

  it('opens the upgrade modal with cap_calendars when the at-cap new-calendar button is clicked', async () => {
    withFree(3)
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('[data-testid="my-calendars-new"]').trigger('click')
    expect(openUpgradeModalMock).toHaveBeenCalledWith('cap_calendars')
    expect(pushSpy).not.toHaveBeenCalledWith('/calendar/new')
    wrapper.unmount()
  })

  it('does not flag the new-calendar button as aria-disabled below the cap', async () => {
    withFree(1)
    const wrapper = mountPage()
    await flushPromises()
    const btn = wrapper.find('[data-testid="my-calendars-new"]')
    expect(btn.attributes('aria-disabled')).toBe('false')
    wrapper.unmount()
  })
})
