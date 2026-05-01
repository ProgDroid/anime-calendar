import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import NotFoundPage from '@/components/NotFoundPage.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    { path: '/', component: { template: '<div />' } },
    { path: '/:pathMatch(.*)*', component: NotFoundPage },
  ],
})

function mountPage() {
  return mount(NotFoundPage, {
    global: {
      plugins: [i18n, router, createPinia()],
    },
  })
}

describe('NotFoundPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders the not-found testid and i18n strings', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="not-found"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="not-found-numeral"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.errors.notFound.title)
    expect(wrapper.text()).toContain(en.errors.notFound.body)
    expect(wrapper.text()).toContain(en.errors.notFound.cta)
  })

  it('navigates to / when the action button is clicked', async () => {
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mountPage()
    await wrapper.find('[data-testid="not-found"] button').trigger('click')
    await flushPromises()
    expect(pushSpy).toHaveBeenCalledWith('/')
  })
})
