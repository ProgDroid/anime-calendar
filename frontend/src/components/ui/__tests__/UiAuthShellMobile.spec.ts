import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import UiAuthShellMobile from '@/components/ui/UiAuthShellMobile.vue'
import en from '@/locales/en.json'
import pt from '@/locales/pt.json'
import type { PosterRef } from '@/services/posters'

// Mock the posters service so no real sessionStorage/fetch happens in tests.
vi.mock('@/services/posters', () => ({
  getRandomCoverPosters: vi.fn(
    async (n = 3): Promise<PosterRef[]> =>
      Array.from({ length: n }, (_, i) => ({
        background: `linear-gradient(${i * 60}deg, #111, #222)`,
        glyph: ['◐', '✦', '▲'][i] ?? '◐',
      })),
  ),
}))

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: { en, pt },
})

const mountShell = (options: Parameters<typeof mount>[1] = {}) =>
  mount(UiAuthShellMobile, {
    ...options,
    global: { ...options.global, plugins: [i18n] },
  })

describe('UiAuthShellMobile', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders 3 poster elements with aria-label from i18n key', async () => {
    const wrapper = mountShell()
    await flushPromises()

    const posters = wrapper.findAll('[data-testid^="auth-mobile-poster-"]')
    expect(posters).toHaveLength(3)

    const expectedLabel = en.auth.mobile.posterAlt
    for (const poster of posters) {
      expect(poster.attributes('aria-label')).toBe(expectedLabel)
    }
  })

  it('renders slot content inside the bottom form area', async () => {
    const wrapper = mountShell({
      slots: { default: '<form data-testid="login-form" />' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="login-form"]').exists()).toBe(true)
  })
})
