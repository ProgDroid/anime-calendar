import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import ItemSearchPanel from '@/components/calendar/ItemSearchPanel.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const mockItem: Item = {
  id: 1, id_mal: null,
  title: { english: 'Naruto', native: 'ナルト', romaji: 'Naruto' },
  media_type: 'ANIME', episode_duration: 23, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
}

describe('ItemSearchPanel', () => {
  it('renders the search input', () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.find('input[type="text"]').exists()).toBe(true)
  })

  it('emits search event with name and media type when button clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('input[type="text"]').setValue('Naruto')
    await wrapper.find('button[type="button"]').trigger('click')
    const emitted = wrapper.emitted('search')
    expect(emitted).toBeTruthy()
    expect(emitted?.[0]?.[0]).toMatchObject({ name: 'Naruto' })
  })

  it('renders fetched items', () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Naruto')
  })

  it('emits toggle-selection when item is clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="item-card-1"]').trigger('click')
    expect(wrapper.emitted('toggle-selection')).toBeTruthy()
  })

  it('emits add-selected when add button clicked', async () => {
    const wrapper = mount(ItemSearchPanel, {
      props: { fetchedItems: [mockItem], selectedItems: [1], itemsInCalendar: [], loading: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    await wrapper.find('[data-testid="add-selected-btn"]').trigger('click')
    expect(wrapper.emitted('add-selected')).toBeTruthy()
  })
})
