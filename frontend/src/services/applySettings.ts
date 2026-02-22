import type { UserSettings } from "@/types/userSettings"

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
}
