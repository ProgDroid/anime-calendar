import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import LoginPage from '@/components/LoginPage.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn(), get: vi.fn(), delete: vi.fn(), put: vi.fn() }
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createWebHistory(), routes: [{ path: '/:p*', component: LoginPage }] })

function mountPage() {
  return mount(LoginPage, {
    global: {
      plugins: [i18n, router, createPinia()],
      stubs: { GoogleLoginButton: true }
    }
  })
}

// UiInput wraps the <input> in a <div>; data-testid lands on the wrapper, so
// we drill into the inner input for value setting.
const inputAt = (wrapper: ReturnType<typeof mountPage>, testid: string) =>
  wrapper.find(`[data-testid="${testid}"] input`)

describe('LoginPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders in login mode by default', () => {
    const wrapper = mountPage()
    expect(wrapper.text()).toContain(en.auth.login.title)
    expect(wrapper.find('[data-testid="login-username"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="login-email"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-password"]').exists()).toBe(true)
  })

  it('renders the new design surfaces (tagline, poster collage, submit, google)', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="login-tagline"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-poster-collage"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-submit"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-google"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-forgot-link"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-register-link"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-form"]').exists()).toBe(true)
  })

  it('toggles to register mode when register link clicked', async () => {
    const wrapper = mountPage()
    await wrapper.find('[data-testid="login-register-link"]').trigger('click')
    expect(wrapper.text()).toContain(en.auth.register.title)
    expect(wrapper.find('[data-testid="login-username"]').exists()).toBe(true)
  })

  it('clears error when toggling mode', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'fail' } } })
    const wrapper = mountPage()
    await inputAt(wrapper, 'login-email').setValue('x@x.com')
    await inputAt(wrapper, 'login-password').setValue('pass')
    await wrapper.find('[data-testid="login-form"]').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[data-testid="login-error"]').exists()).toBe(true)

    await wrapper.find('[data-testid="login-register-link"]').trigger('click')
    expect(wrapper.find('[data-testid="login-error"]').exists()).toBe(false)
  })

  it('calls api.post /login with correct credentials on submit', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: { username: 'user' } })
    const wrapper = mountPage()
    await inputAt(wrapper, 'login-email').setValue('test@example.com')
    await inputAt(wrapper, 'login-password').setValue('secret')
    await wrapper.find('[data-testid="login-form"]').trigger('submit')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/login', { email: 'test@example.com', password: 'secret' })
  })

  it('calls api.post /register with credentials in register mode', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: { username: 'user' } })
    const wrapper = mountPage()
    await wrapper.find('[data-testid="login-register-link"]').trigger('click')
    await inputAt(wrapper, 'login-username').setValue('newuser')
    await inputAt(wrapper, 'login-email').setValue('test@example.com')
    await inputAt(wrapper, 'login-password').setValue('secret')
    await wrapper.find('[data-testid="login-form"]').trigger('submit')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/register', {
      username: 'newuser',
      email: 'test@example.com',
      password: 'secret'
    })
  })

  it('shows error on login failure', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'bad creds' } } })
    const wrapper = mountPage()
    await inputAt(wrapper, 'login-email').setValue('test@example.com')
    await inputAt(wrapper, 'login-password').setValue('wrong')
    await wrapper.find('[data-testid="login-form"]').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[data-testid="login-error"]').exists()).toBe(true)
  })

  it('disables submit button while loading', async () => {
    let resolve!: (v: unknown) => void
    vi.mocked(api.post).mockReturnValue(new Promise(r => { resolve = r }))
    const wrapper = mountPage()
    await inputAt(wrapper, 'login-email').setValue('test@example.com')
    await inputAt(wrapper, 'login-password').setValue('pass')
    wrapper.find('[data-testid="login-form"]').trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="login-submit"]').attributes('disabled')).toBeDefined()
    resolve!({ data: { username: 'u' } })
  })
})
