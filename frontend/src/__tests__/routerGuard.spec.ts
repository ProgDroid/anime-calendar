import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent } from 'vue'

// ── Mocks ──────────────────────────────────────────────────────────────────

const reconcileFromServer = vi.fn()
const localeRef = { value: 'en' }

vi.mock('@/composables/useTheme', () => ({
  useTheme: () => ({ reconcileFromServer }),
}))
vi.mock('@/plugins/i18n', () => ({
  i18n: { global: { locale: localeRef } },
}))
vi.mock('@/stores/auth', () => ({ useAuthStore: vi.fn() }))
vi.mock('@/stores/userSettingsStore', () => ({ useUserSettingsStore: vi.fn() }))

import { useAuthStore } from '@/stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'

// ── Helpers ────────────────────────────────────────────────────────────────

const Dummy = defineComponent({ template: '<div />' })

/** Build a fresh in-memory router with the same beforeEach guard as the real one. */
function makeRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', redirect: '/my-calendars' },
      { path: '/login', name: 'Login', component: Dummy, meta: { public: true } },
      { path: '/my-calendars', name: 'MyCalendars', component: Dummy, meta: { requiresAuth: true } },
      { path: '/account/profile', name: 'account.profile', component: Dummy, meta: { requiresAuth: true } },
    ],
  })

  // Mirrors the guard in router/index.ts exactly
  router.beforeEach(async (to, _from, next) => {
    const authStore = useAuthStore()
    const userSettingsStore = useUserSettingsStore()

    await authStore.initAuth()

    if (!to.meta.public && authStore.isAuthenticated()) {
      const settings = await userSettingsStore.fetchSettings()
      reconcileFromServer({
        theme_preference: settings.theme_preference,
        accent_preference: settings.accent_preference,
      })
      localeRef.value = settings.language_preference
    }

    if (to.path === '/login' && authStore.isAuthenticated()) {
      next('/my-calendars')
    } else if (to.meta.requiresAuth && !authStore.isAuthenticated()) {
      next('/login')
    } else {
      next()
    }
  })

  return router
}

function mockAuth(authenticated: boolean) {
  vi.mocked(useAuthStore).mockReturnValue({
    isAuthenticated: () => authenticated,
    initAuth: vi.fn().mockResolvedValue(undefined),
  } as unknown as ReturnType<typeof useAuthStore>)
}

function mockSettings(result: object) {
  vi.mocked(useUserSettingsStore).mockReturnValue({
    fetchSettings: vi.fn().mockResolvedValue(result),
  } as unknown as ReturnType<typeof useUserSettingsStore>)
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('Router navigation guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    localeRef.value = 'en'
  })

  it('unauthenticated user navigating to a protected route is redirected to /login', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('authenticated user can navigate to a protected route', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', accent_preference: 'iris', language_preference: 'pt' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('authenticated user navigating to /login is redirected to /my-calendars', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', accent_preference: 'iris', language_preference: 'pt' })
    const router = makeRouter()
    await router.push('/login')
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('unauthenticated user can access public /login route', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/login')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('reconciles theme + locale from server on authenticated protected navigation', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', accent_preference: 'iris', language_preference: 'pt' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(reconcileFromServer).toHaveBeenCalledWith({ theme_preference: 'light', accent_preference: 'iris' })
    expect(localeRef.value).toBe('pt')
  })

  it('does NOT reconcile for an unauthenticated user (no clobber of localStorage)', async () => {
    mockAuth(false)
    mockSettings({ theme_preference: 'dark', accent_preference: 'coral', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(reconcileFromServer).not.toHaveBeenCalled()
    expect(localeRef.value).toBe('en')
  })

  it('does NOT reconcile when an unauthenticated user navigates to a public route', async () => {
    mockAuth(false)
    const fetchSettings = vi.fn().mockResolvedValue({})
    vi.mocked(useUserSettingsStore).mockReturnValue({
      fetchSettings,
    } as unknown as ReturnType<typeof useUserSettingsStore>)
    const router = makeRouter()
    await router.push('/login')
    expect(fetchSettings).not.toHaveBeenCalled()
    expect(reconcileFromServer).not.toHaveBeenCalled()
  })
})
