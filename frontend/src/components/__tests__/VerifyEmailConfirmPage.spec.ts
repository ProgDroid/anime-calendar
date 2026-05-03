import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'
import { createPinia, setActivePinia } from 'pinia'

import en from '@/locales/en.json'

const verifyEmailMock = vi.fn()
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    verifyEmail: (...args: unknown[]) => verifyEmailMock(...args),
  }),
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/my-calendars', component: defineComponent({ render: () => h('div') }) },
      { path: '/verify', component: defineComponent({ render: () => h('div') }) },
    ],
  })
}

describe('VerifyEmailConfirmPage — mobile smoke test', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    verifyEmailMock.mockReset()
    verifyEmailMock.mockResolvedValue(undefined)
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the eyebrow', async () => {
    const { default: VerifyEmailConfirmPage } = await import('../VerifyEmailConfirmPage.vue')
    const wrapper = mount(VerifyEmailConfirmPage, {
      global: { plugins: [i18n, makeRouter(), createPinia()] },
    })
    expect(wrapper.text()).toContain(en.auth.verifyEmail.confirm.eyebrow)
  })
})
