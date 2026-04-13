import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { useAuthStore } from './stores/auth'
import { i18n, initI18n } from './plugins/i18n'
import { useUserSettingsStore } from './stores/userSettingsStore'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(i18n)

const authStore = useAuthStore()
authStore.initAuth().then(() => {
    const userSettingsStore = useUserSettingsStore()
    userSettingsStore.fetchSettings().then(settings => {
        initI18n(settings.language_preference)
        app.mount('#app')
    }).catch(() => {
        initI18n('en')
        app.mount('#app')
    })
})
