import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import MyCalendarsPage from '@/components/MyCalendarsPage.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), delete: vi.fn(), post: vi.fn(), put: vi.fn() },
  getApiUrl: (path: string) => `http://localhost/api${path}`
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createWebHistory(), routes: [{ path: '/:p*', component: MyCalendarsPage }] })

const mockCalendars = [
  {
    id: 1,
    name: 'My Anime Calendar',
    item_count: 5,
    subscription_token: 'token123',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z'
  },
  {
    id: 2,
    name: 'Second Calendar',
    item_count: 2,
    subscription_token: 'token456',
    created_at: '2026-01-03T00:00:00Z',
    updated_at: '2026-01-03T00:00:00Z'
  }
]

const mockPagination = { page: 1, page_size: 6, total: 2, total_pages: 1 }

function mountPage() {
  return mount(MyCalendarsPage, {
    global: { plugins: [i18n, router, createPinia()] }
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
    vi.mocked(api.get).mockResolvedValue({
      data: { data: mockCalendars, pagination: mockPagination }
    })
  })

  it('renders the page title', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain(en.calendars.title)
  })

  it('shows calendar list after successful load', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain('My Anime Calendar')
    expect(wrapper.text()).toContain('Second Calendar')
  })

  it('shows empty state when no calendars returned', async () => {
    vi.mocked(api.get).mockResolvedValue({
      data: { data: [], pagination: { ...mockPagination, total: 0 } }
    })
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain(en.calendars.notFound)
  })

  it('shows error message when API call fails', async () => {
    vi.mocked(api.get).mockRejectedValue(new Error('Network error'))
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.calendars.loadFailed)
  })

  it('opens confirm modal when delete button is clicked', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.findAll('.btn-error')[0].trigger('click')
    expect(wrapper.find('[data-testid="modal-box"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.calendars.deleteConfirmTitle)
  })

  it('calls DELETE and reloads on confirm delete', async () => {
    vi.mocked(api.delete).mockResolvedValue({})
    const wrapper = mountPage()
    await flushPromises()

    // Open the confirm modal
    await wrapper.findAll('.btn-error')[0].trigger('click')
    await wrapper.vm.$nextTick()

    // Click the confirm button inside the modal
    await wrapper.find('[data-testid="confirm-btn"]').trigger('click')
    await flushPromises()

    expect(api.delete).toHaveBeenCalledWith('/calendars/1')
    expect(api.get).toHaveBeenCalledTimes(2) // initial load + reload after delete
  })

  it('navigates to /calendar/new on create button click', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    // First btn-primary is the "Create new" button at the top
    await wrapper.find('.btn.btn-primary').trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/new')
  })

  it('navigates to calendar edit page on edit button click', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await flushPromises()
    // Edit buttons are btn-sm btn-primary inside each card
    await wrapper.find('.btn-sm.btn-primary').trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/1')
  })
})
