import { createI18n } from 'vue-i18n'

// Load translations at build time
import enTranslations from '@/locales/en.json'
import ptTranslations from '@/locales/pt.json'

// Create i18n instance
export const i18n = createI18n({
  locale: 'en', // default locale
  fallbackLocale: 'en',
  messages: {
    en: enTranslations,
    pt: ptTranslations
  },
  legacy: false // Use Composition API mode
})

// Initialize language from user settings (delayed initialization)
export function initI18n(initialLanguage: 'en' | 'pt') {
  // Set the initial language for i18n
  i18n.global.locale.value = initialLanguage
}

export default i18n
// TODO make translation keys more sane
