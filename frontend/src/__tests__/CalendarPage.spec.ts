import { describe, it, expect } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia } from 'pinia'
import CalendarPage from '@/components/CalendarPage.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      {
        path: '/calendar/:id',
        component: CalendarPage,
        children: [
          { path: '', name: 'calendar.editor', component: { template: '<div data-testid="editor-stub" />' } },
          { path: 'schedule', name: 'calendar.schedule', component: { template: '<div data-testid="schedule-stub" />' } },
        ],
      },
    ],
  })
}

describe('CalendarPage (shell)', () => {
  it('renders the segmented tabs as RouterLinks with aria-current', async () => {
    const router = makeRouter()
    router.push('/calendar/42')
    await router.isReady()
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    expect(wrapper.find('[data-testid="calendar-tabs"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Editor')
    expect(wrapper.text()).toContain('Schedule')
    const links = wrapper.find('[data-testid="calendar-tabs"]').findAll('a')
    expect(links).toHaveLength(2)
    expect(links[0]!.attributes('aria-current')).toBe('page')
    expect(links[1]!.attributes('aria-current')).toBeUndefined()
  })

  it('starts on editor tab and renders the editor outlet', async () => {
    const router = makeRouter()
    router.push('/calendar/42')
    await router.isReady()
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    expect(wrapper.find('[data-testid="editor-stub"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="schedule-stub"]').exists()).toBe(false)
  })

  it('clicking schedule link navigates to /calendar/:id/schedule', async () => {
    const router = makeRouter()
    router.push('/calendar/42')
    await router.isReady()
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    const links = wrapper.find('[data-testid="calendar-tabs"]').findAll('a')
    await links[1]!.trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/calendar/42/schedule')
  })

  it('aria-current swaps to schedule link when on schedule route', async () => {
    const router = makeRouter()
    router.push('/calendar/42/schedule')
    await router.isReady()
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    const links = wrapper.find('[data-testid="calendar-tabs"]').findAll('a')
    expect(links[0]!.attributes('aria-current')).toBeUndefined()
    expect(links[1]!.attributes('aria-current')).toBe('page')
  })
})
