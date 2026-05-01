<script setup lang="ts">
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { useAuthStore } from './stores/auth'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { computed, ref, watch } from 'vue'
import { applySettings } from './services/applySettings'
import { useTheme } from './composables/useTheme'
import UiToastHost from './components/ui/UiToastHost.vue'
import UiMenu from './components/ui/UiMenu.vue'
import IconLogo from './components/ui/icons/IconLogo.vue'
import IconSun from './components/ui/icons/IconSun.vue'
import IconMoon from './components/ui/icons/IconMoon.vue'

const authStore = useAuthStore()
const userSettingsStore = useUserSettingsStore()
const route = useRoute()
const mobileOpen = ref(false)
const { theme, setTheme } = useTheme()

const handleLogout = () => {
  authStore.logout()
  userSettingsStore.clearCache()
  applySettings(userSettingsStore.getDefaultSettings())
  mobileOpen.value = false
}

const toggleTheme = () => {
  setTheme(theme.value === 'dark' ? 'light' : 'dark')
}

const initials = computed(() => {
  const name = authStore.user ?? ''
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length >= 2) return (parts[0]![0]! + parts[1]![0]!).toUpperCase()
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase()
  return '?'
})

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
          class="flex items-center gap-2 text-lg font-semibold tracking-tight hover:text-accent-1 transition-colors"
          :aria-label="$t('app.title')"
        >
          <IconLogo class="w-[22px] h-[22px]" />
          <span>Anime <span class="italic font-normal" style="font-family: var(--font-display)">Calendar</span></span>
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

          <button
            type="button"
            data-testid="topbar-theme-toggle"
            class="ml-2 p-2 rounded-md text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors [&_svg]:w-3.5 [&_svg]:h-3.5"
            :aria-label="$t('app.toggleTheme')"
            @click="toggleTheme"
          >
            <IconSun v-if="theme === 'dark'" />
            <IconMoon v-else />
          </button>

          <UiMenu align="right">
            <template #trigger>
              <button
                type="button"
                data-testid="topbar-avatar"
                :aria-label="$t('app.userMenu')"
                class="w-[30px] h-[30px] rounded-full bg-accent-1/15 text-accent-1 font-semibold text-xs flex items-center justify-center hover:ring-2 hover:ring-accent-1/30 transition"
              >
                {{ initials }}
              </button>
            </template>
            <RouterLink
              to="/account"
              class="px-3 py-1.5 text-sm text-fg-1 hover:bg-bg-2 rounded-sm text-left"
              role="menuitem"
            >
              {{ $t('app.myAccount') }}
            </RouterLink>
            <button
              type="button"
              data-testid="topbar-logout"
              class="px-3 py-1.5 text-sm text-fg-1 hover:bg-bg-2 rounded-sm text-left"
              role="menuitem"
              @click="handleLogout"
            >
              {{ $t('app.logout') }}
            </button>
          </UiMenu>
        </nav>

        <button
          v-if="authStore.isAuthenticated()"
          type="button"
          class="md:hidden p-2 rounded-md text-fg-1 hover:bg-bg-2 transition-colors"
          :aria-label="$t('app.toggleNavigation')"
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
              data-testid="mobile-theme-toggle"
              class="w-full text-left px-3 py-2 rounded-md text-fg-2 hover:text-fg-1 hover:bg-bg-2 transition-colors flex items-center gap-2"
              :aria-label="$t('app.toggleTheme')"
              @click="toggleTheme"
            >
              <IconSun v-if="theme === 'dark'" class="w-4 h-4" />
              <IconMoon v-else class="w-4 h-4" />
              <span>{{ $t('app.toggleTheme') }}</span>
            </button>
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
