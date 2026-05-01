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
  it('default radio variant: selected option marked aria-checked', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'b', options: opts, ariaLabel: 'group' } })
    const btns = w.findAll('button')
    expect(w.find('[role="radiogroup"]').exists()).toBe(true)
    expect(btns[1]!.attributes('role')).toBe('radio')
    expect(btns[1]!.attributes('aria-checked')).toBe('true')
    expect(btns[0]!.attributes('aria-checked')).toBe('false')
    expect(btns[1]!.attributes('tabindex')).toBe('0')
    expect(btns[0]!.attributes('tabindex')).toBe('-1')
  })
  it('tab variant: selected option marked aria-selected', () => {
    const w = mount(UiSegmented, { props: { modelValue: 'b', options: opts, variant: 'tab' } })
    const btns = w.findAll('button')
    expect(w.find('[role="tablist"]').exists()).toBe(true)
    expect(btns[1]!.attributes('role')).toBe('tab')
    expect(btns[1]!.attributes('aria-selected')).toBe('true')
    expect(btns[0]!.attributes('aria-selected')).toBe('false')
  })
})
