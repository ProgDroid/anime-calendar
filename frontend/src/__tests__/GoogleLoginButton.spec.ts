import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createI18n } from 'vue-i18n'
import GoogleLoginButton from '@/components/GoogleLoginButton.vue'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { post: vi.fn(), get: vi.fn(), delete: vi.fn(), put: vi.fn() }
}))

import api from '@/config/api'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:p*', component: { template: '<div/>' } }] })

function mountBtn() {
  return mount(GoogleLoginButton, {
    global: {
      plugins: [i18n, router, createPinia()],
    }
  })
}

describe('GoogleLoginButton', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    delete window.handleGoogleLogin
  })

  it('registers window.handleGoogleLogin on mount', () => {
    mountBtn()
    expect(typeof window.handleGoogleLogin).toBe('function')
  })

  it('removes window.handleGoogleLogin on unmount', () => {
    const wrapper = mountBtn()
    wrapper.unmount()
    expect(window.handleGoogleLogin).toBeUndefined()
  })

  it('renders Google Sign-In container elements', () => {
    const wrapper = mountBtn()
    expect(wrapper.find('#g_id_onload').exists()).toBe(true)
    expect(wrapper.find('.g_id_signin').exists()).toBe(true)
  })

  it('calls oauthLogin with google token on callback', async () => {
    vi.mocked(api.post).mockResolvedValue({
      data: { token: 'jwt', username: 'user', avatar: 'https://avatar.url' }
    })
    mountBtn()
    await window.handleGoogleLogin!({ client_id: 'cid', credential: 'google-credential' })
    expect(api.post).toHaveBeenCalledWith('/auth/google', { token: 'google-credential' })
  })

  it('handles oauthLogin failure gracefully (no throw)', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { data: { error: 'oauth fail' } } })
    mountBtn()
    await expect(
      window.handleGoogleLogin!({ client_id: 'cid', credential: 'bad-cred' })
    ).resolves.toBeUndefined()
  })
})
