import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'

import en from '@/locales/en.json'
import EditorItemsPanelMobile from '@/components/calendar/EditorItemsPanelMobile.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: defineComponent({ render: () => h('div') }) }],
  })
}

const baseProps = {
  items: [],
  calendarLanguage: 'english' as const,
  recommendations: [],
}

function mountPanel(overrides: Record<string, unknown> = {}) {
  return mount(EditorItemsPanelMobile, {
    props: { ...baseProps, ...overrides },
    global: { plugins: [i18n, makeRouter()] },
  })
}

describe('EditorItemsPanelMobile — free-tier counter + warning banner', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows the show counter chip when showCount is provided (free user)', () => {
    const wrapper = mountPanel({ showCount: 5 })
    const chip = wrapper.find('[data-testid="show-counter-chip"]')
    expect(chip.exists()).toBe(true)
    expect(chip.text()).toContain('5')
    expect(chip.text()).toContain('25')
    wrapper.unmount()
  })

  it('hides the counter chip when showCount is null (pro user)', () => {
    const wrapper = mountPanel({ showCount: null })
    expect(wrapper.find('[data-testid="show-counter-chip"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('hides the counter chip when showCount is undefined (pro user)', () => {
    const wrapper = mountPanel({})
    expect(wrapper.find('[data-testid="show-counter-chip"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('renders the 80% warning banner when showCount is at or above the threshold', () => {
    // FREE_SHOW_CAP = 25, threshold 0.8 → 20+
    const wrapper = mountPanel({ showCount: 20 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('does not render the warning banner below the threshold', () => {
    const wrapper = mountPanel({ showCount: 5 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('hides the warning banner once the cap is fully reached (chip becomes the cap state)', () => {
    const wrapper = mountPanel({ showCount: 25 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('dismisses the warning banner when dismiss is clicked, and keeps it dismissed', async () => {
    const wrapper = mountPanel({ showCount: 22 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(true)
    await wrapper.find('[data-testid="show-warning-banner-dismiss"]').trigger('click')
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('hides the banner for pro users (no showCount)', () => {
    const wrapper = mountPanel({})
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })
})
