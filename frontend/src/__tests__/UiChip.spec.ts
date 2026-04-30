import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiChip from '@/components/ui/UiChip.vue'

describe('UiChip', () => {
  it('renders default variant', () => {
    const w = mount(UiChip, { slots: { default: 'tag' } })
    expect(w.text()).toBe('tag')
  })
  it('pro variant uses accent class', () => {
    const w = mount(UiChip, { props: { variant: 'pro' } })
    expect(w.classes().some(c => c.includes('accent'))).toBe(true)
  })
  it('success variant uses success class', () => {
    const w = mount(UiChip, { props: { variant: 'success' } })
    expect(w.classes().some(c => c.includes('success'))).toBe(true)
  })
})
