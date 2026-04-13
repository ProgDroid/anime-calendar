import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { getUserSettings, updateUserSettings, invalidateSettingsCache } from '@/services/userSettingsService'

vi.mock('@/config/api', () => ({
  default: { get: vi.fn(), put: vi.fn() }
}))

import api from '@/config/api'

const SETTINGS_KEY = 'user_settings'
const TIMESTAMP_KEY = 'user_settings_timestamp'

const mockSettings = {
  theme_preference: 'dark' as const,
  language_preference: 'en' as const,
  title_language_preference: 'English' as const,
  timezone: 'UTC'
}

describe('userSettingsService', () => {
  let store: Record<string, string> = {}

  beforeEach(() => {
    store = {}
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn((key: string) => store[key] ?? null),
      setItem: vi.fn((key: string, value: string) => { store[key] = value }),
      removeItem: vi.fn((key: string) => { delete store[key] }),
      clear: vi.fn(() => { store = {} }),
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  describe('getUserSettings', () => {
    it('fetches from API and caches when no cache exists', async () => {
      vi.mocked(api.get).mockResolvedValue({ data: mockSettings })
      const result = await getUserSettings()
      expect(api.get).toHaveBeenCalledWith('/user/settings')
      expect(result).toEqual(mockSettings)
      expect(localStorage.setItem).toHaveBeenCalledWith(SETTINGS_KEY, JSON.stringify(mockSettings))
    })

    it('returns cached settings when cache is still valid (< 30 min)', async () => {
      store[SETTINGS_KEY] = JSON.stringify(mockSettings)
      store[TIMESTAMP_KEY] = Date.now().toString()
      const result = await getUserSettings()
      expect(api.get).not.toHaveBeenCalled()
      expect(result).toEqual(mockSettings)
    })

    it('re-fetches from API when cache timestamp is older than 30 minutes', async () => {
      const thirtyOneMinutesAgo = Date.now() - 31 * 60 * 1000
      store[SETTINGS_KEY] = JSON.stringify(mockSettings)
      store[TIMESTAMP_KEY] = thirtyOneMinutesAgo.toString()
      vi.mocked(api.get).mockResolvedValue({ data: mockSettings })
      await getUserSettings()
      expect(api.get).toHaveBeenCalledWith('/user/settings')
    })

    it('re-fetches from API when no timestamp exists', async () => {
      store[SETTINGS_KEY] = JSON.stringify(mockSettings)
      // no timestamp set
      vi.mocked(api.get).mockResolvedValue({ data: mockSettings })
      await getUserSettings()
      expect(api.get).toHaveBeenCalledWith('/user/settings')
    })
  })

  describe('updateUserSettings', () => {
    it('calls PUT /user/settings with the new settings', async () => {
      vi.mocked(api.put).mockResolvedValue({})
      await updateUserSettings(mockSettings)
      expect(api.put).toHaveBeenCalledWith('/user/settings', mockSettings)
    })

    it('updates the localStorage cache after PUT', async () => {
      vi.mocked(api.put).mockResolvedValue({})
      await updateUserSettings(mockSettings)
      expect(localStorage.setItem).toHaveBeenCalledWith(SETTINGS_KEY, JSON.stringify(mockSettings))
      expect(localStorage.setItem).toHaveBeenCalledWith(TIMESTAMP_KEY, expect.any(String))
    })
  })

  describe('invalidateSettingsCache', () => {
    it('removes both cache keys from localStorage', () => {
      store[SETTINGS_KEY] = JSON.stringify(mockSettings)
      store[TIMESTAMP_KEY] = Date.now().toString()
      invalidateSettingsCache()
      expect(localStorage.removeItem).toHaveBeenCalledWith(SETTINGS_KEY)
      expect(localStorage.removeItem).toHaveBeenCalledWith(TIMESTAMP_KEY)
    })
  })
})
