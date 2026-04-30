import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createWebHistory } from 'vue-router'
import UserSettingsPage from '@/components/UserSettingsPage.vue'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import en from '@/locales/en.json'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), put: vi.fn(), post: vi.fn(), delete: vi.fn() }
}))

vi.mock('@/services/applySettings', () => ({
  applySettings: vi.fn()
}))

import { applySettings } from '@/services/applySettings'

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
const router = createRouter({ history: createWebHistory(), routes: [{ path: '/:p*', component: UserSettingsPage }] })

const mockSettings = {
  theme_preference: 'dark' as const,
  language_preference: 'en' as const,
  title_language_preference: 'English' as const,
  timezone: 'UTC'
}

// Shared pinia so store spies survive into mountPage
let pinia: ReturnType<typeof createPinia>
let settingsStore: ReturnType<typeof useUserSettingsStore>

function mountPage() {
  // Seed auth state so onMounted's isAuthenticated() check returns true
  pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }
  return mount(UserSettingsPage, {
    global: { plugins: [i18n, router, pinia] }
  })
}

describe('UserSettingsPage', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    pinia.state.value['auth'] = { user: 'mock-user', name: '', user_avatar: '' }
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue(null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    })
    // Initialize store on the shared pinia so spies target the right instance
    settingsStore = useUserSettingsStore()
    vi.spyOn(settingsStore, 'fetchSettings').mockResolvedValue(mockSettings)
    vi.spyOn(settingsStore, 'updateSettings').mockResolvedValue(undefined)
  })

  it('renders the settings page title', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain(en.userSettings.title)
  })

  it('calls fetchSettings on mount when authenticated', async () => {
    mountPage()
    await flushPromises()
    expect(settingsStore.fetchSettings).toHaveBeenCalled()
  })

  it('renders theme radio buttons after settings load', async () => {
    const wrapper = mountPage()
    await flushPromises()
    const radios = wrapper.findAll('input[type="radio"][name="theme"]')
    expect(radios.length).toBe(2)
    expect(radios.map(r => (r.element as HTMLInputElement).value)).toContain('light')
    expect(radios.map(r => (r.element as HTMLInputElement).value)).toContain('dark')
  })

  it('renders language select with en and pt options', async () => {
    const wrapper = mountPage()
    await flushPromises()
    const selects = wrapper.findAll('select')
    const langSelect = selects[0]
    const options = langSelect?.findAll('option') ?? []
    expect(options.map(o => o.element.value)).toContain('en')
    expect(options.map(o => o.element.value)).toContain('pt')
  })

  it('calls updateSettings and applySettings when save button clicked', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await wrapper.find('.btn-primary').trigger('click')
    await flushPromises()
    expect(settingsStore.updateSettings).toHaveBeenCalled()
    expect(applySettings).toHaveBeenCalled()
  })

  it('shows error when fetchSettings fails', async () => {
    vi.spyOn(settingsStore, 'fetchSettings').mockRejectedValue(new Error('fail'))
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.find('.alert-error').exists()).toBe(true)
    expect(wrapper.text()).toContain(en.userSettings.fetchFailed)
  })
})
