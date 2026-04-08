import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import CalendarItemsList from '@/components/calendar/CalendarItemsList.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const mockItem: Item = {
  id: 1, id_mal: null,
  title: { english: 'Bleach', native: 'ブリーチ', romaji: 'Bleach' },
  media_type: 'ANIME', episode_duration: 24, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
}

describe('CalendarItemsList', () => {
  it('renders item titles', () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Bleach')
  })

  it('emits remove when remove button is clicked', async () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="remove-item-1"]').trigger('click')
    expect(wrapper.emitted('remove')?.[0]).toEqual([1])
  })

  it('emits clear when clear button is clicked', async () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [mockItem], calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="clear-btn"]').trigger('click')
    expect(wrapper.emitted('clear')).toBeTruthy()
  })

  it('disables clear button when no items', () => {
    const wrapper = mount(CalendarItemsList, {
      props: { items: [], calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="clear-btn"]').attributes('disabled')).toBeDefined()
  })
})
