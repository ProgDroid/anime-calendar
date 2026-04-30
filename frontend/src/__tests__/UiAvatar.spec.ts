import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiAvatar from '@/components/ui/UiAvatar.vue'

describe('UiAvatar', () => {
  it('renders img when src provided', () => {
    const w = mount(UiAvatar, { props: { src: '/p.png', alt: 'Pat' } })
    expect(w.find('img').exists()).toBe(true)
    expect(w.find('img').attributes('alt')).toBe('Pat')
  })
  it('renders fallback initials when no src', () => {
    const w = mount(UiAvatar, { props: { alt: 'Pat', fallback: 'PA' } })
    expect(w.find('img').exists()).toBe(false)
    expect(w.text()).toContain('PA')
  })
})
