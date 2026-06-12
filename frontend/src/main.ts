import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import './assets/main.css'
import { useAuthStore } from './stores/auth'
import { i18n, initI18n } from './plugins/i18n'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { useTheme } from './composables/useTheme'
import { loadPublicConfig } from './services/publicConfig'
import { logger } from './services/logger'

const app = createApp(App)

app.use(createPinia())

useTheme().init()

app.use(router)
app.use(i18n)

const authStore = useAuthStore()

// Bootstrap is split into two phases with different failure semantics:
//
// 1. Public config — hard requirement. Without `googleClientId` the Google
//    sign-in button is non-functional. A failure here renders a static
//    error shell instead of mounting a half-broken SPA, so misconfigured
//    deploys fail loudly rather than silently shipping a dead button.
//
// 2. Auth + settings — soft. The app already tolerates these failing
//    (unauth users have no /user response and no settings) and mounts with
//    English defaults. Preserves the existing degraded-mode behavior.
loadPublicConfig()
    .then(() => authStore.initAuth())
    .then(() => {
        const userSettingsStore = useUserSettingsStore()
        return userSettingsStore.fetchSettings().then(settings => {
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
    .catch(err => {
        // Only reachable if loadPublicConfig() rejected — auth/settings
        // failures are caught above and resolve to a degraded mount.
        logger.error('Public config bootstrap failed; refusing to mount.', err)
        renderBootstrapErrorShell()
    })

// Static error shell rendered via safe DOM APIs (no innerHTML) so a future
// CSP tightening doesn't break this fallback path.
function renderBootstrapErrorShell(): void {
    const root = document.getElementById('app')
    if (!root) return
    root.replaceChildren()

    const wrapper = document.createElement('div')
    wrapper.setAttribute('role', 'alert')
    wrapper.style.cssText =
        'padding:2rem;font-family:system-ui;max-width:40rem;margin:4rem auto;'

    const heading = document.createElement('h1')
    heading.style.marginTop = '0'
    heading.textContent = 'Application configuration unavailable'

    const body = document.createElement('p')
    body.textContent =
        'The app could not load its startup configuration. Please refresh the page or try again shortly.'

    wrapper.append(heading, body)
    root.append(wrapper)
}
