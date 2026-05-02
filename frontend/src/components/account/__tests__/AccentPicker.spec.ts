import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import AccentPicker from '../AccentPicker.vue'

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: {
    en: {
      account: {
        preferences: {
          accents: {
            coral: 'Coral',
            iris: 'Iris',
            matcha: 'Matcha',
            sakura: 'Sakura',
            citron: 'Citron',
          },
          proChip: 'Pro',
          proNote: 'Free during early access',
        },
      },
    },
  },
})

function mountPicker(
  modelValue: 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron' = 'coral',
  isPaid = false,
) {
  return mount(AccentPicker, {
    props: { modelValue, isPaid },
    global: { plugins: [i18n] },
  })
}

describe('AccentPicker', () => {
  it('renders all 5 accent swatches', () => {
    const wrapper = mountPicker('coral')
    expect(wrapper.findAll('[data-testid^="accent-swatch-"]')).toHaveLength(5)
  })

  it('shows Pro chip on pro accents', () => {
    const wrapper = mountPicker('coral')
    expect(wrapper.find('[data-testid="accent-pro-matcha"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="accent-pro-sakura"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="accent-pro-citron"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="accent-pro-coral"]').exists()).toBe(false)
  })

  it('emits update:modelValue on free-accent click for any tier', async () => {
    const wrapper = mountPicker('coral', false)
    await wrapper.find('[data-testid="accent-swatch-iris"]').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['iris'])
    expect(wrapper.emitted('interrupt')).toBeUndefined()
  })

  it('emits interrupt (NOT update:modelValue) when free user clicks Pro accent', async () => {
    const wrapper = mountPicker('coral', false)
    await wrapper.find('[data-testid="accent-swatch-matcha"]').trigger('click')
    expect(wrapper.emitted('interrupt')?.[0]).toEqual(['matcha'])
    expect(wrapper.emitted('update:modelValue')).toBeUndefined()
  })

  it('emits update:modelValue (NOT interrupt) when paid user clicks Pro accent', async () => {
    const wrapper = mountPicker('coral', true)
    await wrapper.find('[data-testid="accent-swatch-sakura"]').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['sakura'])
    expect(wrapper.emitted('interrupt')).toBeUndefined()
  })
})
