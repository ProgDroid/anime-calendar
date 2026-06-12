import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { makeSmokeRouter } from '@/__tests__/test-utils/router'

import en from '@/locales/en.json'

vi.mock('axios', () => {
  const post = vi.fn()
  const get = vi.fn()
  // Unified shape across all axios-mocking specs so a cross-file mock-registry
  // leak can't strip `axios.post` (→ "mockResolvedValue is not a function") or
  // swap the error predicate. Recognises either marker the specs use (F2-32).
  const isAxiosError = (
    err: unknown,
  ): err is { response?: { status?: number; data?: Record<string, unknown> } } =>
    typeof err === 'object' && err !== null && ('isAxiosError' in err || '__isAxiosError' in err)
  return { default: { post, get, isAxiosError }, isAxiosError }
})

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('ResetPasswordPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the heading', async () => {
    const { default: ResetPasswordPage } = await import('../ResetPasswordPage.vue')
    // Navigate with a token query param so onMounted doesn't redirect.
    const router = makeSmokeRouter('/reset-password?token=test-token-abc')
    await router.isReady()
    const wrapper = mount(ResetPasswordPage, {
      global: { plugins: [i18n, router] },
    })
    expect(wrapper.text()).toContain(en.auth.resetPassword.heading)
  })
})
