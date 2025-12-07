import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { loadConfig } from './config/api'

// Load configuration before creating the app
loadConfig().then(() => {
    let app = createApp(App)

    app.use(createPinia())
    app.use(router)

    app.mount('#app')
})
