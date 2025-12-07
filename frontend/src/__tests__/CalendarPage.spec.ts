import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CalendarPage from '@/components/CalendarPage.vue'

describe('CalendarPage', () => {
  it('renders correctly', () => {
    const wrapper = mount(CalendarPage)
    expect(wrapper.exists()).toBe(true)
  })
})
