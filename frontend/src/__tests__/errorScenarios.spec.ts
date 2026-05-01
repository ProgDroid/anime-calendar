/**
 * Error-scenario tests for major pages.
 *
 * Focuses on paths not already covered in component-specific spec files:
 *  - Status-code-specific error messages (401 vs 5xx)
 *  - CalendarPage load and save failures
 *  - MyCalendarsPage delete failure
 */

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory, createMemoryHistory } from 'vue-router'
import en from '@/locales/en.json'

// ── Global mocks ────────────────────────────────────────────────────────────

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), put: vi.fn(), post: vi.fn(), delete: vi.fn() }
}))
vi.mock('@/services/applySettings', () => ({ applySettings: vi.fn() }))
vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: () => ({
    fetchSettings: vi.fn().mockResolvedValue(null),
    getDefaultSettings: vi.fn().mockReturnValue({}),
  })
}))

import api from '@/config/api'
import axios from 'axios'

// Axios error factory
function axiosError(status: number) {
  return Object.assign(new Error('Request failed'), {
    isAxiosError: true,
    response: { status, data: { error: `HTTP ${status}` } }
  })
}

// ── UserDetailsPage — password update status-code branching ─────────────────

describe('UserDetailsPage — password update error paths', () => {
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/:p*', component: { template: '<div/>' } }]
  })
  let pinia: ReturnType<typeof createPinia>

  beforeEach(async () => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.clearAllMocks()

    const { default: UserDetailsPage } = await import('@/components/UserDetailsPage.vue')
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: { username: 'u', email: 'u@u.com', is_oauth: false }
    })

    pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }

    return { UserDetailsPage }
  })

  async function mountAndOpenPasswordForm() {
    const { default: UserDetailsPage } = await import('@/components/UserDetailsPage.vue')
    const wrapper = mount(UserDetailsPage, {
      global: { plugins: [i18n, router, pinia] }
    })
    await flushPromises()
    // Open password-change section
    const toggle = wrapper.findAll('button').find(b => b.text().includes(en.userDetails.changePassword))
    await toggle?.trigger('click')
    await wrapper.vm.$nextTick()
    return wrapper
  }

  it('401 on password update shows "current password incorrect" message', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(401))
    vi.spyOn(axios, 'isAxiosError').mockReturnValue(true)

    const wrapper = await mountAndOpenPasswordForm()
    const inputs = wrapper.findAll('input[type="password"]')
    // current / new / confirm
    await inputs[0]?.setValue('OldPass1!')
    await inputs[1]?.setValue('NewPass123!@')
    await inputs[2]?.setValue('NewPass123!@')

    const forms = wrapper.findAll('form')
    await forms[forms.length - 1]?.trigger('submit')
    await flushPromises()

    expect(wrapper.text()).toContain(en.userDetails.currentPasswordIncorrect)
    expect(wrapper.text()).not.toContain(en.userDetails.passwordUpdateFailed)
  })

  it('500 on password update shows generic "update failed" message', async () => {
    ;(api.post as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(500))
    vi.spyOn(axios, 'isAxiosError').mockReturnValue(false)

    const wrapper = await mountAndOpenPasswordForm()
    const inputs = wrapper.findAll('input[type="password"]')
    await inputs[0]?.setValue('OldPass1!')
    await inputs[1]?.setValue('NewPass123!@')
    await inputs[2]?.setValue('NewPass123!@')

    const forms = wrapper.findAll('form')
    await forms[forms.length - 1]?.trigger('submit')
    await flushPromises()

    expect(wrapper.text()).toContain(en.userDetails.passwordUpdateFailed)
    expect(wrapper.text()).not.toContain(en.userDetails.currentPasswordIncorrect)
  })
})

// ── CalendarPage — load and save error paths ────────────────────────────────

describe('CalendarPage — error paths', () => {
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  let pinia: ReturnType<typeof createPinia>

  beforeEach(async () => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.clearAllMocks()
    pinia.state.value['auth'] = { user: 'u', name: '', user_avatar: '' }
  })

  it('shows loadFailed when GET /calendars/:id returns an error', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(500))

    const CalendarEditorView = (await import('@/components/calendar/CalendarEditorView.vue')).default
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/calendar/:id', component: CalendarEditorView }]
    })
    await router.push('/calendar/42')

    const wrapper = mount(CalendarEditorView, {
      global: { plugins: [i18n, router, pinia] }
    })
    await flushPromises()

    expect(wrapper.text()).toContain(en.calendar.loadFailed)
  })

  it('shows loadFailed when GET /calendars/:id returns 404', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(404))

    const CalendarEditorView = (await import('@/components/calendar/CalendarEditorView.vue')).default
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/calendar/:id', component: CalendarEditorView }]
    })
    await router.push('/calendar/99')

    const wrapper = mount(CalendarEditorView, {
      global: { plugins: [i18n, router, pinia] }
    })
    await flushPromises()

    expect(wrapper.text()).toContain(en.calendar.loadFailed)
  })

  it('shows updateFailed when PUT /calendar fails', async () => {
    // items must be non-empty so submitCalendar reaches the PUT call
    ;(api.get as ReturnType<typeof vi.fn>).mockResolvedValue({
      data: {
        id: 1,
        name: 'My Cal',
        language: 'English',
        items: [{ id: 1, title: 'Test Anime', image: '', episodes: 12, status: 'RELEASING', next_airing_episode: null, schedule: [] }],
        subscription_token: 'tok'
      }
    })
    ;(api.put as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(500))

    const CalendarEditorView = (await import('@/components/calendar/CalendarEditorView.vue')).default
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [{ path: '/calendar/:id', component: CalendarEditorView }]
    })
    await router.push('/calendar/1')

    const wrapper = mount(CalendarEditorView, {
      global: { plugins: [i18n, router, pinia] }
    })
    await flushPromises()

    // CalendarSettingsForm has no <form> tag — submit is triggered via a button
    // with data-testid="submit-btn" that emits the 'submit' event to the parent.
    await wrapper.find('[data-testid="submit-btn"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain(en.calendar.updateFailed)
  })
})

// ── MyCalendarsPage — delete failure ───────────────────────────────────────

describe('MyCalendarsPage — delete failure', () => {
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/:p*', component: { template: '<div/>' } }]
  })
  let pinia: ReturnType<typeof createPinia>

  beforeEach(async () => {
    pinia = createPinia()
    setActivePinia(pinia)
    vi.clearAllMocks()
    pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }
  })

  it('shows deleteFailed error when DELETE /calendars/:id fails', async () => {
    ;(api.get as ReturnType<typeof vi.fn>).mockImplementation(async (url: string) => {
      if (typeof url === 'string' && url.startsWith('/items')) return { data: [] }
      return {
        data: {
          data: [
            {
              id: 1,
              name: 'Cal 1',
              item_count: 0,
              subscription_token: 'tok',
              created_at: '2026-01-01T00:00:00Z',
              updated_at: '2026-01-01T00:00:00Z',
              recent_item_ids: [],
            },
          ],
          pagination: { page: 1, page_size: 10, total: 1, total_pages: 1 },
        },
      }
    })
    ;(api.delete as ReturnType<typeof vi.fn>).mockRejectedValue(axiosError(500))

    const MyCalendarsPage = (await import('@/components/MyCalendarsPage.vue')).default
    const wrapper = mount(MyCalendarsPage, {
      global: { plugins: [i18n, router, pinia] }
    })
    await flushPromises()

    // Open the confirm modal for the first calendar via the tile delete button
    await wrapper.findAll('[data-testid="calendar-tile-delete"]')[0]?.trigger('click')
    await wrapper.vm.$nextTick()

    // Confirm the delete
    const confirmBtn = wrapper.find('[data-testid="confirm-btn"]')
    await confirmBtn?.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain(en.calendars.deleteFailed)
  })
})
