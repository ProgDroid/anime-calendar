import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { nextTick } from 'vue'
import { useWindowSize } from '@/composables/useWindowSize'

describe('useWindowSize', () => {
  beforeEach(() => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 1024 })
    Object.defineProperty(window, 'innerHeight', { writable: true, configurable: true, value: 768 })
  })

  it('returns current window dimensions', () => {
    const { width, height } = useWindowSize()
    expect(width.value).toBe(1024)
    expect(height.value).toBe(768)
  })

  it('isMobile is false when width >= 768', () => {
    const { isMobile } = useWindowSize()
    expect(isMobile.value).toBe(false)
  })

  it('isMobile is true when width < 768', () => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: 375 })
    const { isMobile } = useWindowSize()
    expect(isMobile.value).toBe(true)
  })
})
