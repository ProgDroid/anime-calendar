import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { loadConfig } from './config/api'
import { useAuthStore } from './stores/auth'

// Load configuration before creating the app
loadConfig().then(() => {
    let app = createApp(App)

    app.use(createPinia())
    app.use(router)

    // Initialize authentication
    const authStore = useAuthStore()
    authStore.initAuth().then(() => {
        app.mount('#app')
    })
})
