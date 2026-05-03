import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn(), get: vi.fn() },
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', component: defineComponent({ render: () => h('div') }) },
      { path: '/verify-pending', component: defineComponent({ render: () => h('div') }) },
    ],
  })
}

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
      global: { plugins: [i18n, makeRouter()] },
    })
    expect(wrapper.text()).toContain(en.auth.verifyEmail.pending.title)
  })
})
