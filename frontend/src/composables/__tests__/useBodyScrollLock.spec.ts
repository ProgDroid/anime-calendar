import { describe, it, expect, beforeEach } from 'vitest'
import {
  lockBodyScroll,
  unlockBodyScroll,
  _resetBodyScrollLockForTests,
} from '@/composables/useBodyScrollLock'

describe('useBodyScrollLock', () => {
  beforeEach(() => {
    _resetBodyScrollLockForTests()
    document.body.style.overflow = ''
  })

  it('locks and unlocks for a single caller', () => {
    expect(document.body.style.overflow).toBe('')
    lockBodyScroll()
    expect(document.body.style.overflow).toBe('hidden')
    unlockBodyScroll()
    expect(document.body.style.overflow).toBe('')
  })

  it('keeps lock held across overlapping callers', () => {
    lockBodyScroll()
    lockBodyScroll()
    expect(document.body.style.overflow).toBe('hidden')
    unlockBodyScroll()
    expect(document.body.style.overflow).toBe('hidden') // still locked
    unlockBodyScroll()
    expect(document.body.style.overflow).toBe('')
  })

  it('restores the host page overflow value on final unlock', () => {
    document.body.style.overflow = 'scroll'
    lockBodyScroll()
    expect(document.body.style.overflow).toBe('hidden')
    unlockBodyScroll()
    expect(document.body.style.overflow).toBe('scroll')
  })

  it('extra unlocks are no-ops', () => {
    unlockBodyScroll()
    unlockBodyScroll()
    expect(document.body.style.overflow).toBe('')
    lockBodyScroll()
    expect(document.body.style.overflow).toBe('hidden')
  })
})
