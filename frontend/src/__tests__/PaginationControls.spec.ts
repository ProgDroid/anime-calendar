import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import PaginationControls from '@/components/shared/PaginationControls.vue'
import en from '@/locales/en.json'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('PaginationControls', () => {
  const mountOpts = { global: { plugins: [i18n] } }

  it('does not render when total_pages <= 1', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 3, total_pages: 1 },
      ...mountOpts
    })
    expect(wrapper.find('[data-testid="pagination"]').exists()).toBe(false)
  })

  it('renders page buttons', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 2, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    expect(wrapper.findAll('button').length).toBeGreaterThan(2)
  })

  it('emits page-change when a page button is clicked', async () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    const buttons = wrapper.findAll('button')
    // Click page 2 (third button: prev, page1, page2...)
    await buttons[2]!.trigger('click')
    expect(wrapper.emitted('page-change')).toBeTruthy()
  })

  it('disables previous button on first page', () => {
    const wrapper = mount(PaginationControls, {
      props: { page: 1, page_size: 6, total: 18, total_pages: 3 },
      ...mountOpts
    })
    expect(wrapper.find('button:first-child').attributes('disabled')).toBeDefined()
  })
})
