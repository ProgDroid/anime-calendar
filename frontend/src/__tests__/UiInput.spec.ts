import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiInput from '@/components/ui/UiInput.vue'

describe('UiInput', () => {
  it('renders label linked via for/id', () => {
    const w = mount(UiInput, { props: { modelValue: '', label: 'Email' } })
    const label = w.find('label')
    const input = w.find('input')
    expect(label.attributes('for')).toBe(input.attributes('id'))
  })

  it('v-model round-trips', async () => {
    const w = mount(UiInput, { props: { modelValue: 'hi' } })
    const input = w.find('input').element as HTMLInputElement
    expect(input.value).toBe('hi')
    await w.find('input').setValue('hello')
    expect(w.emitted('update:modelValue')?.[0]).toEqual(['hello'])
  })

  it('error prop renders helper and danger class', () => {
    const w = mount(UiInput, { props: { modelValue: '', error: 'Required' } })
    expect(w.text()).toContain('Required')
    expect(w.find('input').classes().some(c => c.includes('danger'))).toBe(true)
  })
})
