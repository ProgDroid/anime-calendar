import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getUserSettings, updateUserSettings, getCachedSettings, invalidateSettingsCache } from '@/services/userSettingsService'
import type { UserSettings } from '@/types/userSettings'

export const useUserSettingsStore = defineStore('userSettings', () => {
  const settings = ref<UserSettings | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Get cached settings immediately
  const cachedSettings = computed(() => {
    return getCachedSettings()
  })

  const isAuthenticated = computed(() => {
    // This would need to be connected to auth store
    return true // Placeholder
  })

  const fetchSettings = async (): Promise<UserSettings | null> => {
    if (!isAuthenticated.value) {
      return null
    }

    try {
      loading.value = true
      error.value = null
      
      const fetchedSettings = await getUserSettings()
      settings.value = fetchedSettings
      return fetchedSettings
    } catch (err) {
      error.value = 'Failed to fetch user settings'
      console.error('Error fetching user settings:', err)
      return null
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
    cachedSettings,
    loading,
    error,
    fetchSettings,
    updateSettings,
    clearCache,
    isAuthenticated
  }
})