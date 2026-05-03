import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { defineComponent, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import {
  useViewportLayout,
  MOBILE_BREAKPOINT_PX,
} from '@/composables/useViewportLayout'

function setWidth(px: number) {
  Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: px })
}

describe('useViewportLayout', () => {
  beforeEach(() => {
    setWidth(1280)
    vi.useFakeTimers()
  })
  afterEach(() => {
    vi.useRealTimers()
  })

  it('exposes the named breakpoint constant', () => {
    expect(MOBILE_BREAKPOINT_PX).toBe(1024)
  })

  it('returns isMobile=false above the breakpoint', () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    expect((w.vm as { isMobile: boolean }).isMobile).toBe(false)
    w.unmount()
  })

  it('returns isMobile=true below the breakpoint', () => {
    setWidth(390)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    expect((w.vm as { isMobile: boolean }).isMobile).toBe(true)
    w.unmount()
  })

  it('responds to resize events with debounce', async () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const w = mount(C)
    const vm = w.vm as { isMobile: boolean }
    expect(vm.isMobile).toBe(false)

    setWidth(390)
    window.dispatchEvent(new Event('resize'))
    // Before debounce window elapses, value is still old.
    expect(vm.isMobile).toBe(false)

    vi.advanceTimersByTime(120)
    await nextTick()
    expect(vm.isMobile).toBe(true)

    w.unmount()
  })

  it('shares state across multiple consumers (singleton)', () => {
    setWidth(1280)
    const C = defineComponent({ setup: () => useViewportLayout(), template: '<div/>' })
    const a = mount(C)
    const b = mount(C)
    expect((a.vm as { isMobile: boolean }).isMobile).toBe(false)
    expect((b.vm as { isMobile: boolean }).isMobile).toBe(false)
    a.unmount()
    b.unmount()
  })
})
