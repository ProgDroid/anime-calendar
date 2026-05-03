// Ref-counted body scroll lock. Multiple components (modal, bottom sheet)
// can call lock()/unlock() and the body style is only restored when the
// last caller unlocks. Also captures the prior inline overflow value so
// we don't clobber whatever the host page had before us.
let count = 0
let prevOverflow = ''

export function lockBodyScroll() {
  if (typeof document === 'undefined') return
  if (count++ === 0) {
    prevOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
}

export function unlockBodyScroll() {
  if (typeof document === 'undefined') return
  if (count > 0 && --count === 0) {
    document.body.style.overflow = prevOverflow
  }
}

// Test helper — resets the singleton state. Tests that flip lock/unlock
// across describe blocks should call this in beforeEach to avoid leaks.
export function _resetBodyScrollLockForTests() {
  count = 0
  prevOverflow = ''
}
