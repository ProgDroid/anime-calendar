import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import MediaItemCard from '@/components/shared/MediaItemCard.vue'
import type { Item } from '@/types/item'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

const item: Item = {
  id: 1, id_mal: null,
  title: { english: 'Attack on Titan', native: '進撃の巨人', romaji: 'Shingeki no Kyojin' },
  media_type: 'ANIME', episode_duration: 24, airing_schedule: [],
  cover_image: { extraLarge: '', large: '', medium: 'http://img.test/cover.jpg', color: '' },
  banner_image: '', recommendations: []
}

describe('MediaItemCard', () => {
  const mountOpts = { global: { plugins: [i18n] } }

  it('renders the title', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('Attack on Titan')
  })

  it('renders cover image when available', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false },
      ...mountOpts
    })
    expect(wrapper.find('img').attributes('src')).toBe('http://img.test/cover.jpg')
  })

  it('shows media type badge', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false },
      ...mountOpts
    })
    expect(wrapper.text()).toContain('ANIME')
  })

  it('applies border-accent-1 class when selected', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: true, isInCalendar: false },
      ...mountOpts
    })
    expect(wrapper.classes()).toContain('border-accent-1')
  })

  it('applies border-success class when in calendar', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: true },
      ...mountOpts
    })
    expect(wrapper.classes()).toContain('border-success')
  })

  it('hides episode count when compact', () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false, compact: true },
      ...mountOpts
    })
    // episode count text should not be present in compact mode
    expect(wrapper.find('p.text-xs').exists()).toBe(false)
  })

  it('emits click when card is clicked', async () => {
    const wrapper = mount(MediaItemCard, {
      props: { item, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false },
      ...mountOpts
    })
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toBeTruthy()
  })

  it('renders no-image placeholder when cover_image is absent', () => {
    const noImageItem = { ...item, cover_image: undefined }
    const wrapper = mount(MediaItemCard, {
      props: { item: noImageItem, displayTitle: 'Attack on Titan', isSelected: false, isInCalendar: false },
      ...mountOpts
    })
    expect(wrapper.find('img').exists()).toBe(false)
  })

  it('shows banner overlay when selected with banner image', () => {
    const bannerItem = { ...item, banner_image: 'http://img.test/banner.jpg' }
    const wrapper = mount(MediaItemCard, {
      props: { item: bannerItem, displayTitle: 'Attack on Titan', isSelected: true, isInCalendar: false },
      ...mountOpts
    })
    const banner = wrapper.find('[data-testid="banner"]')
    expect(banner.exists()).toBe(true)
    expect(banner.classes()).toContain('opacity-100')
  })
})
