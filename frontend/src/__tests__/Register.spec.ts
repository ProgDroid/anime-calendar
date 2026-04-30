import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import Register from '@/components/Register.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn(), get: vi.fn(), delete: vi.fn(), put: vi.fn() }
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/register', component: Register },
    { path: '/login', name: 'Login', component: { template: '<div />' } },
    { path: '/verify-email-pending', name: 'VerifyEmailPending', component: { template: '<div />' } },
  ],
})

function mountPage() {
  return mount(Register, {
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

describe('Register', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders all required testids', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="register-form"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-email"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-password"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-confirm"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-submit"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-login-link"]').exists()).toBe(true)
  })

  it('renders the new design surfaces (tagline, poster collage, google)', () => {
    const wrapper = mountPage()
    expect(wrapper.find('[data-testid="register-tagline"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-poster-collage"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-google"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.auth.register.title)
  })

  it('calls api.post /register with credentials on submit', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: { username: 'newuser' } })
    const wrapper = mountPage()
    await inputAt(wrapper, 'register-username').setValue('newuser')
    await inputAt(wrapper, 'register-email').setValue('test@example.com')
    await inputAt(wrapper, 'register-password').setValue('secretpass')
    await inputAt(wrapper, 'register-confirm').setValue('secretpass')
    await wrapper.find('[data-testid="register-form"]').trigger('submit')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/register', {
      username: 'newuser',
      email: 'test@example.com',
      password: 'secretpass',
    })
  })

  it('shows password mismatch error and does not call api when passwords differ', async () => {
    const wrapper = mountPage()
    await inputAt(wrapper, 'register-username').setValue('newuser')
    await inputAt(wrapper, 'register-email').setValue('test@example.com')
    await inputAt(wrapper, 'register-password').setValue('aaa')
    await inputAt(wrapper, 'register-confirm').setValue('bbb')
    await wrapper.find('[data-testid="register-form"]').trigger('submit')
    await flushPromises()
    expect(api.post).not.toHaveBeenCalled()
    expect(wrapper.find('[data-testid="register-error"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="register-error"]').text()).toContain(en.auth.register.passwordMismatch)
  })

  it('shows error on register failure', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'taken' } } })
    const wrapper = mountPage()
    await inputAt(wrapper, 'register-username').setValue('newuser')
    await inputAt(wrapper, 'register-email').setValue('test@example.com')
    await inputAt(wrapper, 'register-password').setValue('secretpass')
    await inputAt(wrapper, 'register-confirm').setValue('secretpass')
    await wrapper.find('[data-testid="register-form"]').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[data-testid="register-error"]').exists()).toBe(true)
  })

  it('disables submit button while loading', async () => {
    let resolve!: (v: unknown) => void
    vi.mocked(api.post).mockReturnValue(new Promise(r => { resolve = r }))
    const wrapper = mountPage()
    await inputAt(wrapper, 'register-username').setValue('newuser')
    await inputAt(wrapper, 'register-email').setValue('test@example.com')
    await inputAt(wrapper, 'register-password').setValue('secretpass')
    await inputAt(wrapper, 'register-confirm').setValue('secretpass')
    wrapper.find('[data-testid="register-form"]').trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="register-submit"]').attributes('disabled')).toBeDefined()
    resolve!({ data: { username: 'newuser' } })
  })

  it('login link points to /login', () => {
    const wrapper = mountPage()
    const link = wrapper.find('[data-testid="register-login-link"]')
    expect(link.attributes('href')).toBe('/login')
  })
})
