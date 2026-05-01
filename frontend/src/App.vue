<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { useAuthStore } from './stores/auth'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { ref, watch } from 'vue'
import { applySettings } from './services/applySettings'
import UiToastHost from './components/ui/UiToastHost.vue'
import UiButton from './components/ui/UiButton.vue'

const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const route = useRoute()
const mobileOpen = ref(false)

const handleLogout = () => {
  authStore.logout()
  userSettingsStore.clearCache()
  applySettings(userSettingsStore.getDefaultSettings())
  mobileOpen.value = false
}

watch(
  () => route.fullPath,
  () => {
    mobileOpen.value = false
  },
)
</script>

<template>
  <div class="min-h-screen bg-bg-0 text-fg-1">
    <header class="sticky top-0 z-40 bg-bg-1/80 backdrop-blur border-b border-line">
      <div class="container mx-auto px-4 h-14 flex items-center justify-between">
        <RouterLink
          to="/my-calendars"
          class="text-lg font-semibold tracking-tight hover:text-accent-1 transition-colors"
        >
          {{ $t('app.title') }}
        </RouterLink>

        <nav v-if="authStore.isAuthenticated()" class="hidden md:flex items-center gap-1">
          <RouterLink
            to="/my-calendars"
            class="px-3 py-1.5 rounded-md text-sm text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors"
            active-class="text-fg-1 bg-bg-2"
          >
            {{ $t('app.myCalendars') }}
          </RouterLink>
          <RouterLink
            to="/account"
            class="px-3 py-1.5 rounded-md text-sm text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors"
            active-class="text-fg-1 bg-bg-2"
          >
            {{ $t('app.myAccount') }}
          </RouterLink>
          <UiButton variant="ghost" size="sm" @click="handleLogout">
            {{ $t('app.logout') }}
          </UiButton>
        </nav>

        <button
          v-if="authStore.isAuthenticated()"
          type="button"
          class="md:hidden p-2 rounded-md text-fg-1 hover:bg-bg-2 transition-colors"
          :aria-label="$t('app.myAccount')"
          :aria-expanded="mobileOpen"
          data-testid="mobile-nav-toggle"
          @click="mobileOpen = !mobileOpen"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-6 w-6"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 6h16M4 12h16M4 18h16"
            />
          </svg>
        </button>
      </div>

      <div
        v-if="mobileOpen && authStore.isAuthenticated()"
        class="md:hidden border-t border-line bg-bg-1"
        data-testid="mobile-nav-panel"
      >
        <ul class="container mx-auto px-4 py-2 flex flex-col gap-1">
          <li>
            <RouterLink
              to="/my-calendars"
              class="block px-3 py-2 rounded-md text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors"
              active-class="text-fg-1 bg-bg-2"
              @click="mobileOpen = false"
            >
              {{ $t('app.myCalendars') }}
            </RouterLink>
          </li>
          <li>
            <RouterLink
              to="/account"
              class="block px-3 py-2 rounded-md text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors"
              active-class="text-fg-1 bg-bg-2"
              @click="mobileOpen = false"
            >
              {{ $t('app.myAccount') }}
            </RouterLink>
          </li>
          <li>
            <button
              type="button"
              class="w-full text-left px-3 py-2 rounded-md text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors"
              @click="handleLogout"
            >
              {{ $t('app.logout') }}
            </button>
          </li>
        </ul>
      </div>
    </header>

    <main class="container mx-auto p-4">
      <RouterView />
    </main>
    <UiToastHost />
  </div>
</template>
