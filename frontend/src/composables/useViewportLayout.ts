import { ref, computed, type ComputedRef } from 'vue'

export const MOBILE_BREAKPOINT_PX = 1024
const DEBOUNCE_MS = 100

// Singleton state — first call wires the resize listener; every subsequent
// call returns the same ref. This avoids N listeners across components and
// guarantees consistent isMobile across the entire app at any given moment.
const ssrSafeWidth = typeof window === 'undefined' ? 1280 : window.innerWidth
const width = ref(ssrSafeWidth)
let installed = false
let debounceHandle: ReturnType<typeof setTimeout> | null = null

function install() {
  if (installed || typeof window === 'undefined') return
  installed = true
  window.addEventListener('resize', () => {
    if (debounceHandle) clearTimeout(debounceHandle)
    debounceHandle = setTimeout(() => {
      width.value = window.innerWidth
      debounceHandle = null
    }, DEBOUNCE_MS)
  })
}

export interface ViewportLayout {
  isMobile: ComputedRef<boolean>
  MOBILE_BREAKPOINT_PX: number
}

export function useViewportLayout(): ViewportLayout {
  install()
  // Sync the singleton ref to current window width on each call. This keeps
  // consumers consistent when the component mounts after a resize that the
  // debounced listener has not yet flushed (or in tests that mutate
  // window.innerWidth without dispatching a resize event).
  if (typeof window !== 'undefined' && width.value !== window.innerWidth) {
    width.value = window.innerWidth
  }
  const isMobile = computed(() => width.value < MOBILE_BREAKPOINT_PX)
  return { isMobile, MOBILE_BREAKPOINT_PX }
}
