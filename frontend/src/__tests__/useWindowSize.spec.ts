import { describe, it, expect, beforeEach } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { useWindowSize } from '@/composables/useWindowSize'

describe('useWindowSize', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 1024 })
    Object.defineProperty(window, 'innerHeight', { writable: true, configurable: true, value: 768 })
  })

  it('returns current window dimensions', () => {
    const TestComponent = defineComponent({
      setup() {
        return useWindowSize()
      },
      template: '<div />'
    })

    const wrapper = mount(TestComponent)
    const vm = wrapper.vm as any

    expect(vm.width).toBe(1024)
    expect(vm.height).toBe(768)

    wrapper.unmount()
  })

  it('isMobile is false when width >= 768', () => {
    const TestComponent = defineComponent({
      setup() {
        return useWindowSize()
      },
      template: '<div />'
    })

    const wrapper = mount(TestComponent)
    const vm = wrapper.vm as any

    expect(vm.isMobile).toBe(false)

    wrapper.unmount()
  })

  it('isMobile is true when width < 768', () => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 375 })
    const TestComponent = defineComponent({
      setup() {
        return useWindowSize()
      },
      template: '<div />'
    })

    const wrapper = mount(TestComponent)
    const vm = wrapper.vm as any

    expect(vm.isMobile).toBe(true)

    wrapper.unmount()
  })

  it('updates width and isMobile reactively on window resize', async () => {
    const TestComponent = defineComponent({
      setup() {
        return useWindowSize()
      },
      template: '<div />'
    })

    const wrapper = mount(TestComponent)
    const vm = wrapper.vm as any

    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 375 })
    window.dispatchEvent(new Event('resize'))
    await nextTick()

    expect(vm.width).toBe(375)
    expect(vm.isMobile).toBe(true)

    wrapper.unmount()
  })
})
