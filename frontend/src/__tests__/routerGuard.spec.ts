import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createRouter, createMemoryHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent } from 'vue'

// ── Mocks ──────────────────────────────────────────────────────────────────

vi.mock('@/services/applySettings', () => ({ applySettings: vi.fn() }))
vi.mock('@/stores/auth', () => ({
  useAuthStore: vi.fn()
}))
vi.mock('@/stores/userSettingsStore', () => ({
  useUserSettingsStore: vi.fn()
}))

import { applySettings } from '@/services/applySettings'
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
      { path: '/account/preferences', name: 'account.preferences', component: Dummy, meta: { requiresAuth: true } },
    ],
  })

  // Mirrors the guard in router/index.ts exactly
  router.beforeEach(async (to, _from, next) => {
    const authStore = useAuthStore()
    const userSettingsStore = useUserSettingsStore()

    // Rehydrate auth state from the server cookie on first navigation.
    // initAuth() is idempotent — subsequent navigations return immediately.
    await authStore.initAuth()

    if (!to.meta.public) {
      const settings = await userSettingsStore.fetchSettings()
      if (settings) {
        applySettings(settings as Parameters<typeof applySettings>[0])
      }
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

function mockSettings(result: object | null) {
  vi.mocked(useUserSettingsStore).mockReturnValue({
    fetchSettings: vi.fn().mockResolvedValue(result),
    initAuth: vi.fn().mockResolvedValue(undefined),
  } as unknown as ReturnType<typeof useUserSettingsStore>)
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('Router navigation guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  // ── Auth redirection ─────────────────────────────────────────────────────

  it('unauthenticated user navigating to a protected route is redirected to /login', async () => {
    mockAuth(false)
    mockSettings(null)
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  it('authenticated user navigating to /login is redirected to /my-calendars', async () => {
    mockAuth(true)
    mockSettings({ theme_preference: 'light', language_preference: 'en' })
    const router = makeRouter()
    await router.push('/login')
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('authenticated user can navigate to a protected route', async () => {
    mockAuth(true)
    mockSettings(null)
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(router.currentRoute.value.path).toBe('/my-calendars')
  })

  it('unauthenticated user can access public /login route', async () => {
    mockAuth(false)
    mockSettings(null)
    const router = makeRouter()
    await router.push('/login')
    expect(router.currentRoute.value.path).toBe('/login')
  })

  // ── Settings fetch behaviour ─────────────────────────────────────────────

  it('fetchSettings is NOT called when navigating to a public route', async () => {
    mockAuth(false)
    const fetchSettings = vi.fn().mockResolvedValue(null)
    vi.mocked(useUserSettingsStore).mockReturnValue({
      fetchSettings,
      initAuth: vi.fn().mockResolvedValue(undefined),
    } as unknown as ReturnType<typeof useUserSettingsStore>)
    const router = makeRouter()
    await router.push('/login')
    expect(fetchSettings).not.toHaveBeenCalled()
  })

  it('fetchSettings IS called when navigating to a protected route', async () => {
    mockAuth(true)
    const fetchSettings = vi.fn().mockResolvedValue(null)
    vi.mocked(useUserSettingsStore).mockReturnValue({
      fetchSettings,
      initAuth: vi.fn().mockResolvedValue(undefined),
    } as unknown as ReturnType<typeof useUserSettingsStore>)
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(fetchSettings).toHaveBeenCalledOnce()
  })

  it('applySettings is called with fetched settings on protected navigation', async () => {
    mockAuth(true)
    const settings = { theme_preference: 'dark', language_preference: 'pt' }
    mockSettings(settings)
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(applySettings).toHaveBeenCalledWith(settings)
  })

  it('applySettings is NOT called when fetchSettings returns null', async () => {
    mockAuth(true)
    mockSettings(null)
    const router = makeRouter()
    await router.push('/my-calendars')
    expect(applySettings).not.toHaveBeenCalled()
  })

  // ── Settings on multiple navigations ────────────────────────────────────

  it('fetchSettings is called on each protected navigation', async () => {
    mockAuth(true)
    const fetchSettings = vi.fn().mockResolvedValue(null)
    vi.mocked(useUserSettingsStore).mockReturnValue({
      fetchSettings,
      initAuth: vi.fn().mockResolvedValue(undefined),
    } as unknown as ReturnType<typeof useUserSettingsStore>)
    const router = makeRouter()
    await router.push('/my-calendars')
    await router.push('/account/profile')
    expect(fetchSettings).toHaveBeenCalledTimes(2)
  })
})
