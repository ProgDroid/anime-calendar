import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import Wordmark from '@/components/shared/Wordmark.vue'

describe('Wordmark', () => {
  it('renders the app name', () => {
    const wrapper = mount(Wordmark)
    expect(wrapper.text()).toContain('Anime Calendar')
  })

  it('applies size variant', () => {
    const wrapper = mount(Wordmark, { props: { size: 'lg' } })
    expect(wrapper.attributes('data-size')).toBe('lg')
  })
})
