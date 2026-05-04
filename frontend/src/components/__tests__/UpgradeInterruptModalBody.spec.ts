import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'

import en from '@/locales/en.json'
import UpgradeInterruptModalBody from '@/components/UpgradeInterruptModalBody.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: defineComponent({ render: () => h('div') }) },
      { path: '/upgrade', component: defineComponent({ render: () => h('div') }) },
    ],
  })
}

function mountBody(props: Record<string, unknown> = {}) {
  return mount(UpgradeInterruptModalBody, {
    props,
    global: { plugins: [i18n, makeRouter()] },
  })
}

describe('UpgradeInterruptModalBody — reason-routed copy', () => {
  it('renders cap_calendars heading + description for reason=cap_calendars', () => {
    const wrapper = mountBody({ reason: 'cap_calendars' })
    expect(wrapper.find('[data-testid="upgrade-interrupt-heading"]').text())
      .toBe(en.interrupt.heading.cap_calendars)
    expect(wrapper.find('[data-testid="upgrade-interrupt-description"]').text())
      .toBe(en.interrupt.description.cap_calendars)
    wrapper.unmount()
  })

  it('renders cap_shows heading + description for reason=cap_shows', () => {
    const wrapper = mountBody({ reason: 'cap_shows' })
    expect(wrapper.find('[data-testid="upgrade-interrupt-heading"]').text())
      .toBe(en.interrupt.heading.cap_shows)
    expect(wrapper.find('[data-testid="upgrade-interrupt-description"]').text())
      .toBe(en.interrupt.description.cap_shows)
    wrapper.unmount()
  })

  it('renders pro_accent heading + description for reason=pro_accent', () => {
    const wrapper = mountBody({ reason: 'pro_accent' })
    expect(wrapper.find('[data-testid="upgrade-interrupt-heading"]').text())
      .toBe(en.interrupt.heading.pro_accent)
    expect(wrapper.find('[data-testid="upgrade-interrupt-description"]').text())
      .toBe(en.interrupt.description.pro_accent)
    wrapper.unmount()
  })

  it('falls back to pro_accent copy when no reason is provided', () => {
    const wrapper = mountBody({})
    expect(wrapper.find('[data-testid="upgrade-interrupt-heading"]').text())
      .toBe(en.interrupt.heading.pro_accent)
    expect(wrapper.find('[data-testid="upgrade-interrupt-description"]').text())
      .toBe(en.interrupt.description.pro_accent)
    wrapper.unmount()
  })

  it('keeps the shared 3-bullet feature list across all reasons', () => {
    for (const reason of ['cap_calendars', 'cap_shows', 'pro_accent'] as const) {
      const wrapper = mountBody({ reason })
      expect(wrapper.text()).toContain(en.interrupt.features.accents)
      expect(wrapper.text()).toContain(en.interrupt.features.priority)
      expect(wrapper.text()).toContain(en.interrupt.features.fullSet)
      wrapper.unmount()
    }
  })
})
