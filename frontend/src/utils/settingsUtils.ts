import { getCachedSettings } from '@/services/userSettingsService'

/**
 * Get the current theme preference from cached user settings
 * @returns 'light' or 'dark' theme preference, defaults to 'dark'
 */
export const getCurrentTheme = (): 'light' | 'dark' => {
  const settings = getCachedSettings()
  return settings?.theme_preference || 'dark'
}

/**
 * Get the current language preference from cached user settings
 * @returns 'en' or 'pt' language preference, defaults to 'en'
 */
export const getCurrentLanguage = (): 'en' | 'pt' => {
  const settings = getCachedSettings()
  return settings?.language_preference || 'en'
}

/**
 * Check if user has dark theme enabled
 * @returns true if dark theme, false if light theme
 */
export const isDarkTheme = (): boolean => {
  return getCurrentTheme() === 'dark'
}