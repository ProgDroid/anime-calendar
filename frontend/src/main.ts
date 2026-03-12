import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { loadConfig } from './config/api'
import { useAuthStore } from './stores/auth'
import { i18n, initI18n } from './plugins/i18n'
import { useUserSettingsStore } from './stores/userSettingsStore'

// Load configuration before creating the app
loadConfig().then(() => {
    let app = createApp(App)

    app.use(createPinia())
    app.use(router)
    app.use(i18n)

    // Initialize authentication
    const authStore = useAuthStore()
    authStore.initAuth().then(() => {
        // Get user settings to determine initial language
        const userSettingsStore = useUserSettingsStore()
        userSettingsStore.fetchSettings().then(settings => {
            // Initialize i18n with the user's language preference
            initI18n(settings.language_preference)
            app.mount('#app')
        }).catch(() => {
            // If we can't fetch settings, default to English
            initI18n('en')
            app.mount('#app')
        })
    })
})
