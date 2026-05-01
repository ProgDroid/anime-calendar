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

function mountPicker(modelValue: 'coral' | 'iris' | 'matcha' | 'sakura' | 'citron' = 'coral') {
  return mount(AccentPicker, {
    props: { modelValue },
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

  it('emits update:modelValue on click', async () => {
    const wrapper = mountPicker('coral')
    await wrapper.find('[data-testid="accent-swatch-iris"]').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['iris'])
  })
})
