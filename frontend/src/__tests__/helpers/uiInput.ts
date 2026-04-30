import type { VueWrapper } from '@vue/test-utils'

/**
 * UiInput renders the testid on its wrapper <div>; this drills into the
 * inner <input> so callers can assert/setValue against the native element.
 */
export function inputAt(wrapper: VueWrapper, testid: string) {
  return wrapper.get(`[data-testid="${testid}"] input`)
}
