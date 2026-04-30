import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import VerifyEmailPendingPage from '@/components/VerifyEmailPendingPage.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn() },
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/verify-email-pending', component: VerifyEmailPendingPage },
    { path: '/login', name: 'Login', component: { template: '<div />' } },
  ],
})

function mountPage(email = 'user@example.com') {
  // Stub history.state which the component reads at setup
  Object.defineProperty(window.history, 'state', {
    configurable: true,
    get: () => ({ email }),
  })
  return mount(VerifyEmailPendingPage, {
    global: {
      plugins: [i18n, router, createPinia()],
    },
  })
}

describe('VerifyEmailPendingPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders all required testids', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="verify-pending"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="verify-resend"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="verify-poster-collage"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.auth.verifyEmail.pending.title)
    expect(wrapper.text()).toContain(en.auth.verifyEmail.pending.subtitle)
  })

  it('hides resend button when no email is provided', () => {
    const wrapper = mountPage('')
    expect(wrapper.find('[data-testid="verify-resend"]').exists()).toBe(false)
  })

  it('calls /auth/resend-verification on resend click and shows success', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: {} })
    const wrapper = mountPage()
    await wrapper.find('[data-testid="verify-resend"]').trigger('click')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/auth/resend-verification', {
      email: 'user@example.com',
    })
    expect(wrapper.find('[data-testid="verify-resend-success"]').exists()).toBe(true)
  })

  it('shows error message on resend failure', async () => {
    vi.mocked(api.post).mockRejectedValue(new Error('boom'))
    const wrapper = mountPage()
    await wrapper.find('[data-testid="verify-resend"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="verify-resend-error"]').exists()).toBe(true)
  })

  it('back-to-login link points to /login', () => {
    const wrapper = mountPage()
    const link = wrapper.find('[data-testid="verify-back-to-login"]')
    expect(link.exists()).toBe(true)
    expect(link.attributes('href')).toBe('/login')
  })
})
