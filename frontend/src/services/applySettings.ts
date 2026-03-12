import type { UserSettings } from "@/types/userSettings"
import { i18n } from '@/plugins/i18n'

const applyTheme = (theme: 'light' | 'dark') => {
    const html = document.documentElement

    // Remove existing theme data attributes
    html.removeAttribute('data-theme')

    // Set the new theme data attribute
    html.setAttribute('data-theme', theme)
}

export async function applySettings(settings: UserSettings): Promise<void> {
    // Apply settings to the application
    // This could involve updating various parts of the app

    applyTheme(settings.theme_preference)

    // Apply language setting by updating i18n instance directly
    i18n.global.locale.value = settings.language_preference
}
