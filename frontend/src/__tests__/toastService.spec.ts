import { describe, it, expect, beforeEach, vi } from 'vitest'
import { toastService } from '@/services/toastService'

// toastService mounts Vue components imperatively into the real DOM.
// Tests verify the public interface: id uniqueness, auto-hide timer, hide/hideAll.

describe('toastService', () => {
  beforeEach(() => {
    // Clean up any lingering toasts between tests
    toastService.hideAll()
    vi.clearAllMocks()
  })

  it('show returns a numeric id', () => {
    const id = toastService.show({ message: 'Hello', type: 'info', duration: 0 })
    expect(typeof id).toBe('number')
    toastService.hide(id)
  })

  it('show returns a unique id on each call', () => {
    const id1 = toastService.show({ message: 'A', type: 'info', duration: 0 })
    const id2 = toastService.show({ message: 'B', type: 'info', duration: 0 })
    expect(id1).not.toBe(id2)
    toastService.hide(id1)
    toastService.hide(id2)
  })

  it('success calls show with type "success"', () => {
    const showSpy = vi.spyOn(toastService, 'show').mockReturnValue(999)
    toastService.success('Done!', { duration: 0 })
    expect(showSpy).toHaveBeenCalledWith(expect.objectContaining({ type: 'success', message: 'Done!' }))
    vi.restoreAllMocks()
  })

  it('error calls show with type "error"', () => {
    const showSpy = vi.spyOn(toastService, 'show').mockReturnValue(999)
    toastService.error('Oops!', { duration: 0 })
    expect(showSpy).toHaveBeenCalledWith(expect.objectContaining({ type: 'error', message: 'Oops!' }))
    vi.restoreAllMocks()
  })

  it('warning calls show with type "warning"', () => {
    const showSpy = vi.spyOn(toastService, 'show').mockReturnValue(999)
    toastService.warning('Careful!', { duration: 0 })
    expect(showSpy).toHaveBeenCalledWith(expect.objectContaining({ type: 'warning' }))
    vi.restoreAllMocks()
  })

  it('info calls show with type "info"', () => {
    const showSpy = vi.spyOn(toastService, 'show').mockReturnValue(999)
    toastService.info('FYI', { duration: 0 })
    expect(showSpy).toHaveBeenCalledWith(expect.objectContaining({ type: 'info' }))
    vi.restoreAllMocks()
  })

  it('hide does not throw for an unknown id', () => {
    expect(() => toastService.hide(99999)).not.toThrow()
  })

  it('hideAll does not throw when no toasts are shown', () => {
    expect(() => toastService.hideAll()).not.toThrow()
  })

  it('auto-hides toast after the specified duration', async () => {
    vi.useFakeTimers()
    const hideSpy = vi.spyOn(toastService, 'hide')
    const id = toastService.show({ message: 'Bye', type: 'info', duration: 1000 })
    vi.advanceTimersByTime(1000)
    expect(hideSpy).toHaveBeenCalledWith(id)
    vi.useRealTimers()
  })

  it('does not auto-hide when duration is 0', async () => {
    vi.useFakeTimers()
    const hideSpy = vi.spyOn(toastService, 'hide')
    const id = toastService.show({ message: 'Sticky', type: 'info', duration: 0 })
    vi.advanceTimersByTime(60_000)
    expect(hideSpy).not.toHaveBeenCalledWith(id)
    toastService.hide(id)
    vi.useRealTimers()
  })
})
