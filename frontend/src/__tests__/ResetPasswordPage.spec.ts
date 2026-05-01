import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import axios from 'axios'
import ResetPasswordPage from '@/components/ResetPasswordPage.vue'
import en from '@/locales/en.json'
import { inputAt } from './helpers/uiInput'

vi.mock('axios', () => ({
  default: {
    post: vi.fn(),
    isAxiosError: (err: unknown): err is { response?: { status?: number } } =>
      typeof err === 'object' && err !== null && 'isAxiosError' in err,
  },
}))

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createWebHistory(),
    routes: [
      { path: '/reset-password', component: ResetPasswordPage },
      { path: '/forgot-password', name: 'ForgotPassword', component: { template: '<div />' } },
      { path: '/login', name: 'Login', component: { template: '<div />' } },
    ],
  })
}

async function mountPage(token: string | undefined = 'tok-abc') {
  const router = makeRouter()
  const path = token ? `/reset-password?token=${encodeURIComponent(token)}` : '/reset-password'
  router.push(path)
  await router.isReady()
  return mount(ResetPasswordPage, {
    global: {
      plugins: [i18n, router, createPinia()],
    },
  })
}

describe('ResetPasswordPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('renders all required testids', async () => {
    const wrapper = await mountPage()
    expect(wrapper.find('[data-testid="reset-form"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="reset-password"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="reset-confirm"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="reset-submit"]').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.auth.resetPassword.title)
  })

  it('calls axios.post /api/auth/reset-password with token + new_password on submit', async () => {
    vi.mocked(axios.post).mockResolvedValue({ data: {} })
    const wrapper = await mountPage('tok-xyz')
    await inputAt(wrapper, 'reset-password').setValue('a-very-long-password')
    await inputAt(wrapper, 'reset-confirm').setValue('a-very-long-password')
    await wrapper.find('[data-testid="reset-form"]').trigger('submit')
    await flushPromises()
    expect(axios.post).toHaveBeenCalledWith('/api/auth/reset-password', {
      token: 'tok-xyz',
      new_password: 'a-very-long-password',
    })
    expect(wrapper.find('[data-testid="reset-success"]').exists()).toBe(true)
  })

  it('shows mismatch error and does not call axios when passwords differ', async () => {
    const wrapper = await mountPage()
    await inputAt(wrapper, 'reset-password').setValue('aaaaaaaaaaaa')
    await inputAt(wrapper, 'reset-confirm').setValue('bbbbbbbbbbbb')
    await wrapper.find('[data-testid="reset-form"]').trigger('submit')
    await flushPromises()
    expect(axios.post).not.toHaveBeenCalled()
    const errEl = wrapper.find('[data-testid="reset-error"]')
    expect(errEl.exists()).toBe(true)
    expect(errEl.text()).toContain(en.auth.resetPassword.passwordMismatch)
  })

  it('shows invalid-token error on 400 response', async () => {
    vi.mocked(axios.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 400 },
    })
    const wrapper = await mountPage()
    await inputAt(wrapper, 'reset-password').setValue('a-very-long-password')
    await inputAt(wrapper, 'reset-confirm').setValue('a-very-long-password')
    await wrapper.find('[data-testid="reset-form"]').trigger('submit')
    await flushPromises()
    const errEl = wrapper.find('[data-testid="reset-error"]')
    expect(errEl.exists()).toBe(true)
    expect(errEl.text()).toContain(en.auth.resetPassword.invalidToken)
  })

  it('shows generic error on other failures', async () => {
    vi.mocked(axios.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 500 },
    })
    const wrapper = await mountPage()
    await inputAt(wrapper, 'reset-password').setValue('a-very-long-password')
    await inputAt(wrapper, 'reset-confirm').setValue('a-very-long-password')
    await wrapper.find('[data-testid="reset-form"]').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[data-testid="reset-error"]').exists()).toBe(true)
  })

  it('toggles visibility for both password fields independently', async () => {
    const wrapper = await mountPage()
    expect(inputAt(wrapper, 'reset-password').attributes('type')).toBe('password')
    expect(inputAt(wrapper, 'reset-confirm').attributes('type')).toBe('password')
    await wrapper.find('[data-testid="reset-password-toggle"]').trigger('click')
    expect(inputAt(wrapper, 'reset-password').attributes('type')).toBe('text')
    expect(inputAt(wrapper, 'reset-confirm').attributes('type')).toBe('password')
    await wrapper.find('[data-testid="reset-confirm-toggle"]').trigger('click')
    expect(inputAt(wrapper, 'reset-confirm').attributes('type')).toBe('text')
  })

  it('disables submit while loading', async () => {
    let resolve!: (v: unknown) => void
    vi.mocked(axios.post).mockReturnValue(new Promise(r => { resolve = r }))
    const wrapper = await mountPage()
    await inputAt(wrapper, 'reset-password').setValue('a-very-long-password')
    await inputAt(wrapper, 'reset-confirm').setValue('a-very-long-password')
    wrapper.find('[data-testid="reset-form"]').trigger('submit')
    await wrapper.vm.$nextTick()
    expect(wrapper.find('[data-testid="reset-submit"]').attributes('disabled')).toBeDefined()
    resolve!({ data: {} })
  })
})
