import { vi } from 'vitest'
import { ref } from 'vue'

let originalInnerWidth: number | null = null

/**
 * Stubs `useViewportLayout` to return a fixed `isMobile` value AND sets
 * `window.innerWidth` so the real composable (if not mocked) also reads
 * the right viewport. Call BEFORE importing the component under test:
 *
 *   await mockViewport(390)
 *   const { default: Comp } = await import('@/components/X.vue')
 *
 * The window.innerWidth side-effect makes the helper compatible with
 * specs that mix static and dynamic imports of components consuming
 * `useViewportLayout`. Restored in `resetViewportMock()`.
 */
export async function mockViewport(width: number) {
  if (originalInnerWidth === null) {
    originalInnerWidth = window.innerWidth
  }
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: width,
  })
  vi.doMock('@/composables/useViewportLayout', () => ({
    MOBILE_BREAKPOINT_PX: 1024,
    useViewportLayout: () => ({
      MOBILE_BREAKPOINT_PX: 1024,
      isMobile: ref(width < 1024),
    }),
  }))
}

export function resetViewportMock() {
  vi.doUnmock('@/composables/useViewportLayout')
  vi.resetModules()
  if (originalInnerWidth !== null) {
    Object.defineProperty(window, 'innerWidth', {
      configurable: true,
      writable: true,
      value: originalInnerWidth,
    })
    originalInnerWidth = null
  }
}
