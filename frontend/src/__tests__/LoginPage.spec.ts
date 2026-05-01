import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import LoginPage from '@/components/LoginPage.vue'
import en from '@/locales/en.json'
import { inputAt } from './helpers/uiInput'

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

  it('toggles password visibility when eye button is clicked', async () => {
    const wrapper = mountPage()
    const pwd = inputAt(wrapper, 'login-password')
    expect(pwd.attributes('type')).toBe('password')
    await wrapper.find('[data-testid="login-password-toggle"]').trigger('click')
    expect(inputAt(wrapper, 'login-password').attributes('type')).toBe('text')
    await wrapper.find('[data-testid="login-password-toggle"]').trigger('click')
    expect(inputAt(wrapper, 'login-password').attributes('type')).toBe('password')
  })

  it('remember-me checkbox toggles its checked state', async () => {
    const wrapper = mountPage()
    const cb = wrapper.find('[data-testid="login-remember"]')
    expect(cb.exists()).toBe(true)
    expect((cb.element as HTMLInputElement).checked).toBe(false)
    await cb.setValue(true)
    expect((cb.element as HTMLInputElement).checked).toBe(true)
  })

  it('renders Google CTA before the form (Google-first ordering)', () => {
    const wrapper = mountPage()
    const google = wrapper.find('[data-testid="login-google"]').element
    const form = wrapper.find('[data-testid="login-form"]').element
    // DOCUMENT_POSITION_FOLLOWING (4): form follows google
    expect(google.compareDocumentPosition(form) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
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
