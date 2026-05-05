import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getUserSettings, updateUserSettings, invalidateSettingsCache } from '@/services/userSettingsService'
import type { UserSettings } from '@/types/userSettings'
import { useAuthStore } from './auth'

export const useUserSettingsStore = defineStore('userSettings', () => {
  const settings = ref<UserSettings | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const fetchPromise = ref<Promise<UserSettings> | null>(null)
  const authStore = useAuthStore()

  const getDefaultSettings = (): UserSettings => {
    return {
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      accent_preference: 'coral',
      timezone: 'UTC',
      reminder_offsets_minutes: [30],
    }
  }

  const fetchSettings = async (): Promise<UserSettings> => {
    if (!authStore.isAuthenticated()) {
      return getDefaultSettings()
    }

    // Return in-flight request if one exists
    if (fetchPromise.value) {
      return fetchPromise.value
    }

    const promise = (async () => {
      try {
        loading.value = true
        error.value = null

        const fetchedSettings = await getUserSettings()
        settings.value = fetchedSettings
        return fetchedSettings
      } catch {
        error.value = 'Failed to fetch user settings'
        return getDefaultSettings()
      } finally {
        loading.value = false
        fetchPromise.value = null
      }
    })()

    fetchPromise.value = promise
    return promise
  }

  const updateSettings = async (newSettings: UserSettings): Promise<void> => {
    try {
      await updateUserSettings(newSettings)
      settings.value = newSettings
    } catch (err) {
      error.value = 'Failed to update user settings'
      throw err
    }
  }

  const clearCache = () => {
    settings.value = null
    invalidateSettingsCache()
  }

  return {
    settings,
    loading,
    error,
    fetchPromise,
    fetchSettings,
    updateSettings,
    clearCache,
    getDefaultSettings,
  }
})
