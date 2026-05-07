/**
 * LoginPage — redirect validation (AUDIT H-9)
 *
 * Verifies that the post-login router.push uses a URL-origin check so that
 * protocol-relative (`//evil.com`) and absolute cross-origin URLs are rejected
 * and replaced with the safe default `/my-calendars`.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { defineComponent, h } from 'vue'
import { setActivePinia, createPinia } from 'pinia'

import en from '@/locales/en.json'

// --- service mocks -----------------------------------------------------------
const loginMock = vi.fn()
const registerMock = vi.fn()

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    login: (...args: unknown[]) => loginMock(...args),
    register: (...args: unknown[]) => registerMock(...args),
  }),
}))

// GoogleLoginButton has external deps not needed for these assertions.
vi.mock('../GoogleLoginButton.vue', () => ({
  default: defineComponent({ render: () => h('div') }),
}))

// -----------------------------------------------------------------------------

import LoginPage from '../LoginPage.vue'

const Stub = defineComponent({ render: () => h('div') })

function makeRouter(initialPath: string) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/login', component: LoginPage },
      { path: '/register', component: Stub },
      { path: '/my-calendars', component: Stub },
      { path: '/calendar/:id', component: Stub },
      { path: '/forgot-password', component: Stub },
    ],
  })
  void router.push(initialPath)
  return router
}

async function mountLogin(queryPath: string) {
  setActivePinia(createPinia())
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  const router = makeRouter(queryPath)
  await router.isReady()

  const wrapper = mount(LoginPage, {
    global: { plugins: [i18n, router] },
    attachTo: document.body,
  })

  // Spy on the router's push so we can assert what destination was requested.
  const pushSpy = vi.spyOn(router, 'push')

  return { wrapper, router, pushSpy }
}

describe('LoginPage — redirect validation (AUDIT H-9)', () => {
  beforeEach(() => {
    loginMock.mockReset()
    registerMock.mockReset()
    loginMock.mockResolvedValue(undefined)
  })

  it('rejects a protocol-relative URL (//evil.com) and falls back to /my-calendars', async () => {
    const { wrapper, pushSpy } = await mountLogin('/login?redirect=//evil.com')

    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith('/my-calendars')
    expect(pushSpy).not.toHaveBeenCalledWith(expect.stringContaining('evil.com'))
  })

  it('rejects an absolute cross-origin URL and falls back to /my-calendars', async () => {
    const { wrapper, pushSpy } = await mountLogin('/login?redirect=https://evil.com/steal')

    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith('/my-calendars')
  })

  it('accepts a same-origin path and pushes it directly', async () => {
    const { wrapper, pushSpy } = await mountLogin('/login?redirect=/calendar/42')

    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith('/calendar/42')
  })

  it('falls back to /my-calendars when no redirect query param is present', async () => {
    const { wrapper, pushSpy } = await mountLogin('/login')

    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith('/my-calendars')
  })
})
