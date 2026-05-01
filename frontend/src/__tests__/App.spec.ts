import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { createRouter, createMemoryHistory } from 'vue-router'
import App from '@/App.vue'
import en from '@/locales/en.json'
import { useAuthStore } from '@/stores/auth'
import { useTheme } from '@/composables/useTheme'

vi.mock('@/services/userSettingsService', () => ({
  invalidateSettingsCache: vi.fn(),
  updateUserSettings: vi.fn(async () => undefined),
}))

beforeEach(() => {
  const store: Record<string, string> = {}
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: (k: string) => store[k] ?? null,
      setItem: (k: string, v: string) => {
        store[k] = v
      },
      removeItem: (k: string) => {
        delete store[k]
      },
      clear: () => {
        for (const k of Object.keys(store)) delete store[k]
      },
      key: (i: number) => Object.keys(store)[i] ?? null,
      get length() {
        return Object.keys(store).length
      },
    },
  })
})

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
      { path: '/my-calendars', component: { template: '<div>cal</div>' } },
      { path: '/account', component: { template: '<div>acc</div>' } },
    ],
  })
}

async function mountApp() {
  const router = makeRouter()
  const pinia = createPinia()
  setActivePinia(pinia)
  const auth = useAuthStore()
  auth.user = 'Jane Doe'
  router.push('/my-calendars')
  await router.isReady()
  const wrapper = mount(App, {
    global: {
      plugins: [i18n, router, pinia],
    },
  })
  await flushPromises()
  return { wrapper, router, auth }
}

describe('App.vue topbar', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders avatar with initials when authenticated', async () => {
    const { wrapper } = await mountApp()
    const avatar = wrapper.find('[data-testid="topbar-avatar"]')
    expect(avatar.exists()).toBe(true)
    expect(avatar.text()).toBe('JD')
  })

  it('opens dropdown menu with logout button when avatar clicked', async () => {
    const { wrapper } = await mountApp()
    expect(wrapper.find('[data-testid="topbar-logout"]').exists()).toBe(false)
    await wrapper.find('[data-testid="topbar-avatar"]').trigger('click')
    expect(wrapper.find('[data-testid="topbar-logout"]').exists()).toBe(true)
  })

  it('toggles theme when topbar theme toggle is clicked', async () => {
    const { wrapper } = await mountApp()
    const { theme } = useTheme()
    const before = theme.value
    await wrapper.find('[data-testid="topbar-theme-toggle"]').trigger('click')
    expect(theme.value).not.toBe(before)
  })
})
