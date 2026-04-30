import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UiEmptyState from '@/components/ui/UiEmptyState.vue'

describe('UiEmptyState', () => {
  it('renders title, body, and action slot', () => {
    const wrapper = mount(UiEmptyState, {
      props: { title: 'No items', body: 'Add one to begin' },
      slots: { action: '<button data-testid="cta">Add</button>' },
    })
    expect(wrapper.find('[data-testid="empty-state-title"]').text()).toBe('No items')
    expect(wrapper.find('[data-testid="empty-state-body"]').text()).toBe('Add one to begin')
    expect(wrapper.find('[data-testid="cta"]').exists()).toBe(true)
  })

  it('renders illustration slot when provided', () => {
    const wrapper = mount(UiEmptyState, {
      props: { title: 't', body: 'b' },
      slots: { illustration: '<svg data-testid="illo"/>' },
    })
    expect(wrapper.find('[data-testid="illo"]').exists()).toBe(true)
  })
})
