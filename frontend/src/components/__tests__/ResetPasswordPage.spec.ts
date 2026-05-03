import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import { defineComponent, h } from 'vue'
import { mockViewport, resetViewportMock } from '@/__tests__/test-utils/viewport'

import en from '@/locales/en.json'

vi.mock('axios', () => ({
  default: { post: vi.fn(), isAxiosError: vi.fn(() => false) },
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', component: defineComponent({ render: () => h('div') }) },
      {
        path: '/forgot-password',
        component: defineComponent({ render: () => h('div') }),
      },
      { path: '/reset', component: defineComponent({ render: () => h('div') }) },
    ],
  })
  // Navigate to a path that includes a token query param so onMounted doesn't redirect
  router.replace({ path: '/reset', query: { token: 'test-token-abc' } })
  return router
}

describe('ResetPasswordPage — mobile smoke test', () => {
  beforeEach(async () => {
    await mockViewport(390)
  })

  afterEach(() => {
    resetViewportMock()
  })

  it('renders at 390 px without throwing and shows the heading', async () => {
    const { default: ResetPasswordPage } = await import('../ResetPasswordPage.vue')
    const router = makeRouter()
    await router.isReady()
    const wrapper = mount(ResetPasswordPage, {
      global: { plugins: [i18n, router] },
    })
    expect(wrapper.text()).toContain(en.auth.resetPassword.heading)
  })
})
