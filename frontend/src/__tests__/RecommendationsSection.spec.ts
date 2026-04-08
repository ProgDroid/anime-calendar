import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import RecommendationsSection from '@/components/calendar/RecommendationsSection.vue'
import en from '@/locales/en.json'
import type { Item } from '@/types/item'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const mountOpts = { global: { plugins: [i18n] } }

const makeItem = (id: number, title: string): Item => ({
  id, id_mal: null,
  title: { english: title, native: title, romaji: title },
  media_type: 'ANIME', episode_duration: 0, airing_schedule: [],
  cover_image: undefined, banner_image: '', recommendations: []
})

describe('RecommendationsSection', () => {
  it('shows empty state when no items in calendar', () => {
    const wrapper = mount(RecommendationsSection, {
      props: { recommendations: [], calendarHasItems: false, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain(en.calendar.addItemsToSeeRecommendations)
  })

  it('shows no-recommendations text when calendar has items but no recommendations', () => {
    const wrapper = mount(RecommendationsSection, {
      props: { recommendations: [], calendarHasItems: true, calendarLanguage: 'english' },
      ...mountOpts
    })
    expect(wrapper.text()).toContain(en.calendar.noRecommendations)
  })

  it('renders recommendation items', () => {
    const wrapper = mount(RecommendationsSection, {
      props: {
        recommendations: [makeItem(1, 'One Piece')],
        calendarHasItems: true,
        calendarLanguage: 'english'
      },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('One Piece')
  })

  it('emits add when add button is clicked', async () => {
    const wrapper = mount(RecommendationsSection, {
      props: {
        recommendations: [makeItem(1, 'One Piece')],
        calendarHasItems: true,
        calendarLanguage: 'english'
      },
      ...mountOpts
    })
    await wrapper.find('[data-testid="add-reco-1"]').trigger('click')
    expect(wrapper.emitted('add')?.[0][0]).toMatchObject({ id: 1 })
  })
})
