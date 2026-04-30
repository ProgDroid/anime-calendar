import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import IconCheck from '@/components/ui/icons/IconCheck.vue'
import IconPlus from '@/components/ui/icons/IconPlus.vue'
import IconSearch from '@/components/ui/icons/IconSearch.vue'

describe.each([
  ['IconCheck', IconCheck],
  ['IconPlus', IconPlus],
  ['IconSearch', IconSearch],
])('%s', (_name, Cmp) => {
  it('uses 24x24 viewBox and currentColor stroke', () => {
    const w = mount(Cmp)
    const svg = w.find('svg')
    expect(svg.attributes('viewBox')).toBe('0 0 24 24')
    expect(svg.attributes('stroke')).toBe('currentColor')
  })

  it('aria-hidden when no aria-label', () => {
    const w = mount(Cmp)
    expect(w.find('svg').attributes('aria-hidden')).toBe('true')
  })

  it('exposes aria-label and role=img when provided', () => {
    const w = mount(Cmp, { props: { ariaLabel: 'foo' } })
    expect(w.find('svg').attributes('aria-label')).toBe('foo')
    expect(w.find('svg').attributes('role')).toBe('img')
  })
})
