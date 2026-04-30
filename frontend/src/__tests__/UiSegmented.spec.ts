import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiSegmented from '@/components/ui/UiSegmented.vue'

const opts = [{ value: 'a', label: 'A' }, { value: 'b', label: 'B' }]

describe('UiSegmented', () => {
  it('renders all options', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'a', options: opts } })
    expect(w.findAll('button')).toHaveLength(2)
  })
  it('clicking emits update:modelValue', async () => {
    const w = mount(UiSegmented, { props: { modelValue: 'a', options: opts } })
    await w.findAll('button')[1]!.trigger('click')
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['b'])
  })
  it('selected option marked aria-selected', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'b', options: opts } })
    expect(w.findAll('button')[1]!.attributes('aria-selected')).toBe('true')
    expect(w.findAll('button')[0]!.attributes('aria-selected')).toBe('false')
  })
})
