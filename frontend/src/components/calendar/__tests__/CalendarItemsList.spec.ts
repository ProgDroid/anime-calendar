import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'

import en from '@/locales/en.json'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function mountList(overrides: Record<string, unknown> = {}) {
  return mount(CalendarItemsList, {
    props: {
      items: [],
      calendarLanguage: 'english' as const,
      ...overrides,
    },
    global: { plugins: [i18n] },
  })
}

describe('CalendarItemsList — free-tier counter + warning banner', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('shows the show counter chip when showCount is provided (free user)', () => {
    const wrapper = mountList({ showCount: 7 })
    const chip = wrapper.find('[data-testid="show-counter-chip"]')
    expect(chip.exists()).toBe(true)
    expect(chip.text()).toContain('7')
    expect(chip.text()).toContain('25')
    wrapper.unmount()
  })

  it('hides the counter chip when showCount is null (pro user)', () => {
    const wrapper = mountList({ showCount: null })
    expect(wrapper.find('[data-testid="show-counter-chip"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('renders the warning banner at the 80% threshold', () => {
    const wrapper = mountList({ showCount: 21 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('does not render the warning banner below the threshold', () => {
    const wrapper = mountList({ showCount: 4 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('dismisses the warning banner per session', async () => {
    const wrapper = mountList({ showCount: 22 })
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(true)
    await wrapper.find('[data-testid="show-warning-banner-dismiss"]').trigger('click')
    expect(wrapper.find('[data-testid="show-warning-banner"]').exists()).toBe(false)
    wrapper.unmount()
  })
})
