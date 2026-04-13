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

describe('LoginPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
  })

  it('renders in login mode by default', () => {
    const wrapper = mountPage()
    expect(wrapper.text()).toContain(en.auth.login.title)
    expect(wrapper.find('#username').exists()).toBe(false)
    expect(wrapper.find('#email').exists()).toBe(true)
    expect(wrapper.find('#password').exists()).toBe(true)
  })

  it('toggles to register mode when link clicked', async () => {
    const wrapper = mountPage()
    await wrapper.find('.link.link-primary').trigger('click')
    expect(wrapper.text()).toContain(en.auth.register.title)
    expect(wrapper.find('#username').exists()).toBe(true)
  })

  it('clears error when toggling mode', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'fail' } } })
    const wrapper = mountPage()
    await wrapper.find('#email').setValue('x@x.com')
    await wrapper.find('#password').setValue('pass')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(wrapper.find('.alert-error').exists()).toBe(true)

    await wrapper.find('.link.link-primary').trigger('click')
    expect(wrapper.find('.alert-error').exists()).toBe(false)
  })

  it('calls api.post /login with correct credentials on submit', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: { token: 'tok', username: 'user' } })
    const wrapper = mountPage()
    await wrapper.find('#email').setValue('test@example.com')
    await wrapper.find('#password').setValue('secret')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/login', { email: 'test@example.com', password: 'secret' })
  })

  it('calls api.post /register with credentials in register mode', async () => {
    vi.mocked(api.post).mockResolvedValue({ data: { token: 'tok', username: 'user' } })
    const wrapper = mountPage()
    await wrapper.find('.link.link-primary').trigger('click')
    await wrapper.find('#username').setValue('newuser')
    await wrapper.find('#email').setValue('test@example.com')
    await wrapper.find('#password').setValue('secret')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(api.post).toHaveBeenCalledWith('/register', {
      username: 'newuser',
      email: 'test@example.com',
      password: 'secret'
    })
  })

  it('shows error alert on login failure', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'bad creds' } } })
    const wrapper = mountPage()
    await wrapper.find('#email').setValue('test@example.com')
    await wrapper.find('#password').setValue('wrong')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
  })

  it('disables submit button while loading', async () => {
    let resolve: (v: any) => void
    vi.mocked(api.post).mockReturnValue(new Promise(r => { resolve = r }))
    const wrapper = mountPage()
    await wrapper.find('#email').setValue('test@example.com')
    await wrapper.find('#password').setValue('pass')
    wrapper.find('form').trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('button[type="submit"]').attributes('disabled')).toBeDefined()
    resolve!({ data: { token: 't', username: 'u' } })
  })
})
