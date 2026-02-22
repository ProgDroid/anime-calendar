import api from '@/config/api'
import type { UserSettings } from '@/types/userSettings'

// Define the storage keys
const SETTINGS_STORAGE_KEY = 'user_settings'
const SETTINGS_TIMESTAMP_KEY = 'user_settings_timestamp'

// Helper function to check if cache is still valid (30 minutes)
const isCacheValid = (): boolean => {
  const timestamp = localStorage.getItem(SETTINGS_TIMESTAMP_KEY)
  if (!timestamp) return false
  
  const now = Date.now()
  const cacheTime = parseInt(timestamp, 10)
  const thirtyMinutes = 30 * 60 * 1000 // 30 minutes in milliseconds
  
  return (now - cacheTime) < thirtyMinutes
}

// Get user settings with local storage caching
export const getUserSettings = async (): Promise<UserSettings> => {
  // Check if we have valid cached settings
  if (isCacheValid()) {
    const cachedSettings = localStorage.getItem(SETTINGS_STORAGE_KEY)
    if (cachedSettings) {
      console.log('Using cached user settings')
      return JSON.parse(cachedSettings)
    }
  }
  
  // Fetch from backend if no valid cache or no cache exists
  try {
    console.log('Fetching user settings from backend')
    const response = await api.get('/user/settings')
    const settings = response.data
    
    // Cache the settings
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
    localStorage.setItem(SETTINGS_TIMESTAMP_KEY, Date.now().toString())
    
    return settings
  } catch (error) {
    console.error('Failed to fetch user settings:', error)
    throw error
  }
}

// Update user settings and invalidate cache
export const updateUserSettings = async (settings: UserSettings): Promise<void> => {
  try {
    await api.put('/user/settings', settings)
    
    // Update cache with new settings
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
    localStorage.setItem(SETTINGS_TIMESTAMP_KEY, Date.now().toString())
    
    console.log('User settings updated and cached')
  } catch (error) {
    console.error('Failed to update user settings:', error)
    throw error
  }
}

// Invalidate the cache (useful when settings are updated from elsewhere)
export const invalidateSettingsCache = (): void => {
  localStorage.removeItem(SETTINGS_STORAGE_KEY)
  localStorage.removeItem(SETTINGS_TIMESTAMP_KEY)
  console.log('User settings cache invalidated')
}

// Initialize the service
export const initSettingsService = (): void => {
  // No initialization needed for this service
  console.log('User settings service initialized')
}

// Get cached settings without making API call (for immediate access)
export const getCachedSettings = (): UserSettings | null => {
  try {
    const cachedSettings = localStorage.getItem(SETTINGS_STORAGE_KEY)
    if (cachedSettings) {
      return JSON.parse(cachedSettings)
    }
    return null
  } catch (error) {
    console.error('Failed to get cached settings:', error)
    return null
  }
}
