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

  it('id prop forwards to inner input and label for', () => {
    const w = mount(UiInput, { props: { modelValue: '', label: 'Email', id: 'email' } })
    expect(w.find('input').attributes('id')).toBe('email')
    expect(w.find('label').attributes('for')).toBe('email')
  })

  it('falls back to useId-generated id when id prop omitted', () => {
    const w = mount(UiInput, { props: { modelValue: '', label: 'Email' } })
    const inputId = w.find('input').attributes('id')
    expect(inputId).toBeTruthy()
    expect(w.find('label').attributes('for')).toBe(inputId)
  })

  it('required prop puts required attribute on inner input', () => {
    const w = mount(UiInput, { props: { modelValue: '', required: true } })
    expect(w.find('input').attributes('required')).toBeDefined()
  })

  it('omits required attribute by default', () => {
    const w = mount(UiInput, { props: { modelValue: '' } })
    expect(w.find('input').attributes('required')).toBeUndefined()
  })

  it('maxlength prop puts maxlength attribute on inner input', () => {
    const w = mount(UiInput, { props: { modelValue: '', maxlength: 50 } })
    expect(w.find('input').attributes('maxlength')).toBe('50')
  })

  it('name prop puts name attribute on inner input', () => {
    const w = mount(UiInput, { props: { modelValue: '', name: 'username' } })
    expect(w.find('input').attributes('name')).toBe('username')
  })

  it('autocomplete prop forwards to inner input', () => {
    const w = mount(UiInput, { props: { modelValue: '', autocomplete: 'current-password' } })
    expect(w.find('input').attributes('autocomplete')).toBe('current-password')
  })
})
