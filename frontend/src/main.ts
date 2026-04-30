import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { useAuthStore } from './stores/auth'
import { i18n, initI18n } from './plugins/i18n'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { useTheme } from './composables/useTheme'

const app = createApp(App)

app.use(createPinia())

useTheme().init()

app.use(router)
app.use(i18n)

const authStore = useAuthStore()
authStore.initAuth().then(() => {
    const userSettingsStore = useUserSettingsStore()
    userSettingsStore.fetchSettings().then(settings => {
        if (authStore.isAuthenticated()) {
            useTheme().reconcileFromServer({
                theme_preference: settings.theme_preference,
                accent_preference: settings.accent_preference,
            })
        }
        initI18n(settings.language_preference)
        app.mount('#app')
    }).catch(() => {
        initI18n('en')
        app.mount('#app')
    })
})
