import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import MyCalendarsPage from '@/components/MyCalendarsPage.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), delete: vi.fn(), post: vi.fn(), put: vi.fn() },
  getApiUrl: (path: string) => `http://localhost/api${path}`,
}))

import api from '@/config/api'

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
      return { data: { data: mockCalendars, pagination: mockPagination } }
    })
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
