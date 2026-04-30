import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useTheme } from '@/composables/useTheme'

vi.mock('@/services/userSettingsService', () => ({
  updateUserSettings: vi.fn().mockResolvedValue(undefined),
  getUserSettings: vi.fn(),
  invalidateSettingsCache: vi.fn(),
}))

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({ isAuthenticated: () => true }),
}))

describe('useTheme', () => {
  let store: Record<string, string> = {}

  beforeEach(() => {
    setActivePinia(createPinia())
    store = {}
    vi.stubGlobal('localStorage', {
      getItem: vi.fn((key: string) => store[key] ?? null),
      setItem: vi.fn((key: string, value: string) => { store[key] = value }),
      removeItem: vi.fn((key: string) => { delete store[key] }),
      clear: vi.fn(() => { store = {} }),
    })
    document.documentElement.setAttribute('data-theme', 'dark')
    document.documentElement.setAttribute('data-accent', 'coral')
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.useRealTimers()
    vi.clearAllMocks()
  })

  it('init() reads data attributes from <html>', () => {
    document.documentElement.setAttribute('data-theme', 'light')
    document.documentElement.setAttribute('data-accent', 'matcha')
    const t = useTheme()
    t.init()
    expect(t.theme.value).toBe('light')
    expect(t.accent.value).toBe('matcha')
  })

  it('setTheme writes data-theme attribute and localStorage', () => {
    const t = useTheme()
    t.init()
    t.setTheme('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(localStorage.getItem('theme')).toBe('light')
  })

  it('setAccent writes data-accent and localStorage', () => {
    const t = useTheme()
    t.init()
    t.setAccent('iris')
    expect(document.documentElement.getAttribute('data-accent')).toBe('iris')
    expect(localStorage.getItem('accent')).toBe('iris')
  })

  it('setTheme rejects unknown values', () => {
    const t = useTheme()
    t.init()
    // @ts-expect-error invalid value test
    t.setTheme('purple')
    expect(localStorage.getItem('theme')).toBeNull()
  })

  it('setAccent rejects unknown values', () => {
    const t = useTheme()
    t.init()
    // @ts-expect-error invalid value test
    t.setAccent('rainbow')
    expect(localStorage.getItem('accent')).toBeNull()
  })

  it('reconcileFromServer overrides local state when server differs', () => {
    const t = useTheme()
    t.init()
    t.setTheme('dark')
    t.reconcileFromServer({
      theme_preference: 'light',
      accent_preference: 'sakura',
    })
    expect(t.theme.value).toBe('light')
    expect(t.accent.value).toBe('sakura')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(localStorage.getItem('accent')).toBe('sakura')
  })

  it('debounces PATCH calls on rapid setAccent toggles', async () => {
    const { updateUserSettings } = await import('@/services/userSettingsService')
    const t = useTheme()
    t.init()
    t.setAccent('iris')
    t.setAccent('matcha')
    t.setAccent('sakura')
    expect(updateUserSettings).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(500)
    expect(updateUserSettings).toHaveBeenCalledTimes(1)
  })
})
