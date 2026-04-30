import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiButton from '@/components/ui/UiButton.vue'

describe('UiButton', () => {
  it('renders default variant primary, size md', () => {
    const w = mount(UiButton, { slots: { default: 'Click' } })
    expect(w.text()).toBe('Click')
    expect(w.attributes('type')).toBe('button')
    expect(w.classes().some(c => c.includes('accent'))).toBe(true)
  })

  it('disabled blocks click emission', async () => {
    const w = mount(UiButton, { props: { disabled: true } })
    await w.trigger('click')
    expect(w.emitted('click')).toBeUndefined()
  })

  it('loading shows spinner and prevents click', async () => {
    const w = mount(UiButton, { props: { loading: true } })
    expect(w.find('[data-testid="spinner"]').exists()).toBe(true)
    await w.trigger('click')
    expect(w.emitted('click')).toBeUndefined()
  })

  it('danger variant applies danger color class', () => {
    const w = mount(UiButton, { props: { variant: 'danger' } })
    expect(w.classes().some(c => c.includes('danger'))).toBe(true)
  })

  it('size sm applies smaller padding class', () => {
    const w = mount(UiButton, { props: { size: 'sm' } })
    const small = mount(UiButton, { props: { size: 'lg' } })
    expect(w.classes().join(' ')).not.toBe(small.classes().join(' '))
  })
})
