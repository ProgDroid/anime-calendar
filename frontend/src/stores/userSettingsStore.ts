import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getUserSettings, updateUserSettings, invalidateSettingsCache } from '@/services/userSettingsService'
import type { UserSettings } from '@/types/userSettings'
import { useAuthStore } from './auth'

export const useUserSettingsStore = defineStore('userSettings', () => {
  const settings = ref<UserSettings | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const authStore = useAuthStore()

  const getDefaultSettings = (): UserSettings => {
    return {
      theme_preference: 'dark',
      language_preference: 'en',
      title_language_preference: 'English',
      timezone: 'UTC'
    }
  }

  const fetchSettings = async (): Promise<UserSettings> => {
    if (!authStore.isAuthenticated()) {
      return getDefaultSettings()
    }

    try {
      loading.value = true
      error.value = null

      // getUserSettings will use cache if available
      const fetchedSettings = await getUserSettings()
      settings.value = fetchedSettings
      return fetchedSettings
    } catch (err) {
      error.value = 'Failed to fetch user settings'
      console.error('Error fetching user settings:', err)
      return getDefaultSettings()
    } finally {
      loading.value = false
    }
  }

  const updateSettings = async (newSettings: UserSettings): Promise<void> => {
    try {
      await updateUserSettings(newSettings)
      settings.value = newSettings
    } catch (err) {
      error.value = 'Failed to update user settings'
      console.error('Error updating user settings:', err)
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
    fetchSettings,
    updateSettings,
    clearCache,
    getDefaultSettings,
  }
})
