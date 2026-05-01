import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { h } from 'vue'
import UiInput from '../UiInput.vue'

describe('UiInput', () => {
  it('renders without icons by default and does not add icon padding', () => {
    const wrapper = mount(UiInput, { props: { modelValue: '' } })
    const input = wrapper.get('input')
    expect(input.classes()).not.toContain('pl-9')
    expect(input.classes()).not.toContain('pr-9')
    expect(wrapper.find('span.absolute.left-3').exists()).toBe(false)
    expect(wrapper.find('span.absolute.right-3').exists()).toBe(false)
  })

  it('renders iconLeft slot and applies pl-9 to the input', () => {
    const wrapper = mount(UiInput, {
      props: { modelValue: '' },
      slots: {
        iconLeft: () => h('svg', { 'data-testid': 'left-icon' }),
      },
    })
    expect(wrapper.find('[data-testid="left-icon"]').exists()).toBe(true)
    const input = wrapper.get('input')
    expect(input.classes()).toContain('pl-9')
    expect(input.classes()).not.toContain('pr-9')
  })

  it('renders iconRight slot with an interactive button and applies pr-9', async () => {
    let clicked = 0
    const wrapper = mount(UiInput, {
      props: { modelValue: '' },
      slots: {
        iconRight: () =>
          h(
            'button',
            {
              type: 'button',
              'data-testid': 'right-action',
              onClick: () => {
                clicked++
              },
            },
            'eye',
          ),
      },
    })
    const btn = wrapper.get('[data-testid="right-action"]')
    const input = wrapper.get('input')
    expect(input.classes()).toContain('pr-9')
    await btn.trigger('click')
    expect(clicked).toBe(1)
  })

  it('supports both icons together', () => {
    const wrapper = mount(UiInput, {
      props: { modelValue: '' },
      slots: {
        iconLeft: () => h('svg', { 'data-testid': 'l' }),
        iconRight: () => h('svg', { 'data-testid': 'r' }),
      },
    })
    const input = wrapper.get('input')
    expect(input.classes()).toContain('pl-9')
    expect(input.classes()).toContain('pr-9')
  })
})
