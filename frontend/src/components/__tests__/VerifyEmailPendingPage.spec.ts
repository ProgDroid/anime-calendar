import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { makeSmokeRouter } from '@/__tests__/test-utils/router'

import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn(), get: vi.fn() },
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

describe('VerifyEmailPendingPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the pending heading', async () => {
    const { default: VerifyEmailPendingPage } = await import('../VerifyEmailPendingPage.vue')
    const wrapper = mount(VerifyEmailPendingPage, {
      global: { plugins: [i18n, makeSmokeRouter('/verify-email/pending')] },
    })
    expect(wrapper.text()).toContain(en.auth.verifyEmail.pending.title)
  })
})
