import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { makeSmokeRouter } from '@/__tests__/test-utils/router'

import en from '@/locales/en.json'

vi.mock('axios', () => ({
  default: { post: vi.fn(), isAxiosError: vi.fn(() => false) },
}))

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
