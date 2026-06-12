import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import axios from 'axios'
import ForgotPasswordPage from '@/components/ForgotPasswordPage.vue'
import en from '@/locales/en.json'
import { inputAt } from './helpers/uiInput'
import { makeSmokeRouter } from './test-utils/router'

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
const router = makeSmokeRouter('/forgot-password')

function mountPage() {
  return mount(ForgotPasswordPage, {
    global: {
      plugins: [i18n, router, createPinia()],
    },
  })
}

describe('ForgotPasswordPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders all required testids', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="forgot-form"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="forgot-email"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="forgot-submit"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="forgot-tagline"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.auth.forgotPassword.title)
  })

  it('calls axios.post /api/auth/forgot-password on submit and shows success', async () => {
    vi.mocked(axios.post).mockResolvedValue({ data: {} })
    const wrapper = mountPage()
    await inputAt(wrapper, 'forgot-email').setValue('test@example.com')
    await wrapper.find('[data-testid="forgot-form"]').trigger('submit')
    await flushPromises()
    expect(axios.post).toHaveBeenCalledWith('/api/auth/forgot-password', { email: 'test@example.com' })
    expect(wrapper.find('[data-testid="forgot-success"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="forgot-form"]').exists()).toBe(false)
  })

  it('shows the same success state on error (anti-enumeration)', async () => {
    vi.mocked(axios.post).mockRejectedValue(new Error('boom'))
    const wrapper = mountPage()
    await inputAt(wrapper, 'forgot-email').setValue('test@example.com')
    await wrapper.find('[data-testid="forgot-form"]').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[data-testid="forgot-success"]').exists()).toBe(true)
  })

  it('disables submit while loading', async () => {
    let resolve!: (v: unknown) => void
    vi.mocked(axios.post).mockReturnValue(new Promise(r => { resolve = r }))
    const wrapper = mountPage()
    await inputAt(wrapper, 'forgot-email').setValue('test@example.com')
    wrapper.find('[data-testid="forgot-form"]').trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="forgot-submit"]').attributes('disabled')).toBeDefined()
    resolve!({ data: {} })
  })

  it('back-to-login link points to /login', () => {
    const wrapper = mountPage()
    const link = wrapper.find('[data-testid="forgot-login-link"]')
    expect(link.exists()).toBe(true)
    expect(link.attributes('href')).toBe('/login')
  })
})
