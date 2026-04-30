import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiAuthShell from '@/components/ui/UiAuthShell.vue'

describe('UiAuthShell', () => {
  it('renders default slot content', () => {
    const wrapper = mount(UiAuthShell, {
      slots: { default: '<p data-testid="slot-content">hello</p>' },
    })
    expect(wrapper.find('[data-testid="slot-content"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('hello')
  })

  it('renders the poster pane with the provided testid', () => {
    const wrapper = mount(UiAuthShell, {
      props: { posterTestid: 'login-poster-collage' },
    })
    const poster = wrapper.find('[data-testid="login-poster-collage"]')
    expect(poster.exists()).toBe(true)
    expect(poster.classes()).toContain('hidden')
    expect(poster.classes()).toContain('lg:block')
    expect(poster.classes()).toContain('flex-1')
    expect(poster.classes()).toContain('bg-bg-2')
  })

  it('omits the poster pane when posterTestid is not provided', () => {
    const wrapper = mount(UiAuthShell)
    expect(wrapper.find('.bg-bg-2').exists()).toBe(false)
  })

  it('applies the outer wrapper layout classes', () => {
    const wrapper = mount(UiAuthShell)
    const root = wrapper.element as HTMLElement
    expect(root.classList.contains('flex')).toBe(true)
    expect(root.classList.contains('bg-bg-1')).toBe(true)
    expect(root.classList.contains('min-h-[calc(100vh-6rem)]')).toBe(true)
  })

  it('applies the card test id when cardTestid is provided', () => {
    const wrapper = mount(UiAuthShell, {
      props: { cardTestid: 'verify-pending' },
      slots: { default: '<p>x</p>' },
    })
    expect(wrapper.find('[data-testid="verify-pending"]').exists()).toBe(true)
  })
})
