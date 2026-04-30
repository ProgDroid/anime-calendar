import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import UiToast from '@/components/ui/UiToast.vue'

describe('UiToast', () => {
  beforeEach(() => vi.useFakeTimers())

  it('renders message', () => {
    const w = mount(UiToast, { props: { message: 'hi' } })
    expect(w.text()).toContain('hi')
  })

  it('auto-dismisses after duration', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 1000 } })
    await vi.advanceTimersByTimeAsync(1000)
    expect(w.emitted('dismiss')).toBeTruthy()
  })

  it('duration=0 stays sticky', async () => {
    const w = mount(UiToast, { props: { message: 'x', duration: 0 } })
    await vi.advanceTimersByTimeAsync(10000)
    expect(w.emitted('dismiss')).toBeUndefined()
  })

  it('danger variant uses danger class', () => {
    const w = mount(UiToast, { props: { message: 'x', variant: 'danger' } })
    expect(w.classes().some(c => c.includes('danger'))).toBe(true)
  })
})
