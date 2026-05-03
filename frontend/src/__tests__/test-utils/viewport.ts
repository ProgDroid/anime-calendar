import { vi } from 'vitest'

/**
 * Stubs `useViewportLayout` to return a fixed `isMobile` value.
 * Call BEFORE importing the component under test:
 *
 *   await mockViewport(390)
 *   const { default: Comp } = await import('@/components/X.vue')
 */
export async function mockViewport(width: number) {
  vi.doMock('@/composables/useViewportLayout', () => ({
    MOBILE_BREAKPOINT_PX: 1024,
    useViewportLayout: () => ({
      MOBILE_BREAKPOINT_PX: 1024,
      isMobile: { value: width < 1024 },
    }),
  }))
}

export function resetViewportMock() {
  vi.doUnmock('@/composables/useViewportLayout')
  vi.resetModules()
}
