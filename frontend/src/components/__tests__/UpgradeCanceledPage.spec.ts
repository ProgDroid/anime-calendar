import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/upgrade', component: defineComponent({ render: () => h('div') }) },
      { path: '/my-calendars', component: defineComponent({ render: () => h('div') }) },
      { path: '/upgrade/canceled', component: defineComponent({ render: () => h('div') }) },
    ],
  })
}

describe('UpgradeCanceledPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the canceled heading', async () => {
    const { default: UpgradeCanceledPage } = await import('../UpgradeCanceledPage.vue')
    const wrapper = mount(UpgradeCanceledPage, {
      global: { plugins: [i18n, makeRouter()] },
    })
    expect(wrapper.text()).toContain(en.upgrade.canceled.heading)
    expect(wrapper.find('[data-testid="upgrade-canceled-retry"]').exists()).toBe(true)
  })
})
