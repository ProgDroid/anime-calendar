import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { makeSmokeRouter } from '@/__tests__/test-utils/router'

import en from '@/locales/en.json'

vi.mock('axios', () => ({
  default: { post: vi.fn() },
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('ForgotPasswordPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the heading', async () => {
    const { default: ForgotPasswordPage } = await import('../ForgotPasswordPage.vue')
    const wrapper = mount(ForgotPasswordPage, {
      global: { plugins: [i18n, makeSmokeRouter('/forgot-password')] },
    })
    expect(wrapper.text()).toContain(en.auth.forgotPassword.heading)
  })
})
