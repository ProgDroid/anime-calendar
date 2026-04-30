import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import VerifyEmailConfirmPage from '@/components/VerifyEmailConfirmPage.vue'
import en from '@/locales/en.json'

const verifyEmailMock = vi.fn()
vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ verifyEmail: verifyEmailMock }),
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter(query: Record<string, string> = {}) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/verify-email', component: VerifyEmailConfirmPage },
      { path: '/login', name: 'Login', component: { template: '<div />' } },
      { path: '/my-calendars', name: 'MyCalendars', component: { template: '<div />' } },
    ],
  })
  const qs = new URLSearchParams(query).toString()
  router.push(`/verify-email${qs ? `?${qs}` : ''}`)
  return router
}

async function mountPage(query: Record<string, string> = {}) {
  const router = makeRouter(query)
  await router.isReady()
  return mount(VerifyEmailConfirmPage, {
    global: {
      plugins: [i18n, router, createPinia()],
    },
  })
}

describe('VerifyEmailConfirmPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.useFakeTimers()
  })

  it('shows the spinner during verifying state', async () => {
    verifyEmailMock.mockReturnValue(new Promise(() => {}))
    const wrapper = await mountPage({ token: 'abc' })
    expect(wrapper.find('[data-testid="verify-spinner"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="verify-confirm-status"]').exists()).toBe(false)
  })

  it('shows success status when verifyEmail resolves', async () => {
    verifyEmailMock.mockResolvedValue('alice')
    const wrapper = await mountPage({ token: 'abc' })
    await flushPromises()
    expect(wrapper.find('[data-testid="verify-spinner"]').exists()).toBe(false)
    const status = wrapper.find('[data-testid="verify-confirm-status"]')
    expect(status.exists()).toBe(true)
    expect(status.text()).toContain(en.auth.verifyEmail.confirm.success)
    expect(wrapper.find('[data-testid="verify-go-to-calendars"]').exists()).toBe(true)
  })

  it('shows error status when verifyEmail rejects', async () => {
    verifyEmailMock.mockRejectedValue(new Error('bad token'))
    const wrapper = await mountPage({ token: 'abc' })
    await flushPromises()
    const status = wrapper.find('[data-testid="verify-confirm-status"]')
    expect(status.exists()).toBe(true)
    expect(status.text()).toContain(en.auth.verifyEmail.confirm.error)
    expect(wrapper.find('[data-testid="verify-back-to-login"]').exists()).toBe(true)
  })

  it('shows error status when token is missing from query', async () => {
    const wrapper = await mountPage({})
    await flushPromises()
    expect(verifyEmailMock).not.toHaveBeenCalled()
    const status = wrapper.find('[data-testid="verify-confirm-status"]')
    expect(status.exists()).toBe(true)
    expect(status.text()).toContain(en.auth.verifyEmail.confirm.error)
  })
})
