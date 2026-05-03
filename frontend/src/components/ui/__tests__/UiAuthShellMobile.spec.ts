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

  it('renders 3 decorative poster elements (aria-hidden via container)', async () => {
    const wrapper = mountShell()
    await flushPromises()

    const posters = wrapper.findAll('[data-testid^="auth-mobile-poster-"]')
    expect(posters).toHaveLength(3)
    // The fan is purely decorative — the wrapping container carries
    // aria-hidden="true" so screen readers skip the entire fan.
    expect(wrapper.find('[aria-hidden="true"]').exists()).toBe(true)
  })

  it('renders slot content inside the bottom form area', async () => {
    const wrapper = mountShell({
      slots: { default: '<form data-testid="login-form" />' },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="login-form"]').exists()).toBe(true)
  })

  it('renders eyebrow / heading / subtitle props above the form slot', async () => {
    const wrapper = mountShell({
      props: {
        eyebrow: 'Welcome back',
        heading: 'Sign in',
        subtitle: 'Pick up where you left off.',
      },
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="auth-eyebrow"]').text()).toBe('Welcome back')
    expect(wrapper.find('[data-testid="auth-heading"]').text()).toBe('Sign in')
    expect(wrapper.find('[data-testid="auth-subtitle"]').text()).toBe('Pick up where you left off.')
  })
})
