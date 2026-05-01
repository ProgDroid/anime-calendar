import { describe, it, expect, vi } from 'vitest'
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
  it('renders the segmented tabs', async () => {
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

  it('switching tab pushes to /calendar/:id/schedule', async () => {
    const router = makeRouter()
    router.push('/calendar/42')
    await router.isReady()
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    const buttons = wrapper.find('[data-testid="calendar-tabs"]').findAll('button')
    // [editor, schedule]
    await buttons[1]!.trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/42/schedule')
  })

  it('switching back to editor from schedule pushes to /calendar/:id', async () => {
    const router = makeRouter()
    router.push('/calendar/42/schedule')
    await router.isReady()
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mount(CalendarPage, {
      global: { plugins: [i18n, router, createPinia()] },
    })
    await flushPromises()
    const buttons = wrapper.find('[data-testid="calendar-tabs"]').findAll('button')
    await buttons[0]!.trigger('click')
    expect(pushSpy).toHaveBeenCalledWith('/calendar/42')
  })
})
