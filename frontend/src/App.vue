<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { useAuthStore } from './stores/auth'
import { useUserSettingsStore } from './stores/userSettingsStore'
import { ref } from 'vue'
import { applySettings } from './services/applySettings'

const authStore = useAuthStore()
const drawerToggle = ref<HTMLInputElement | null>(null)
const userSettingsStore = useUserSettingsStore()

const handleLogout = () => {
  authStore.logout()
  userSettingsStore.clearCache()
  applySettings(userSettingsStore.getDefaultSettings())
  // Close the drawer after logout
  if (drawerToggle.value) {
    drawerToggle.value.checked = false
  }
}
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] bg-base-200">
    <div class="navbar bg-base-100 shadow">
      <div class="flex-1">
        <RouterLink to="/my-calendars" class="btn btn-ghost text-xl">{{ $t('app.title') }}</RouterLink>
      </div>
      <div class="flex-none">
        <!-- DaisyUI Drawer for mobile -->
        <div class="drawer md:hidden">
          <input id="my-drawer" ref="drawerToggle" type="checkbox" class="drawer-toggle" />
          <div class="drawer-content">
            <!-- Hamburger menu button -->
            <label v-if="authStore.isAuthenticated()" for="my-drawer" class="btn btn-ghost">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            </label>
          </div>
          <div class="drawer-side z-50">
            <label for="my-drawer" class="drawer-overlay"></label>
            <ul class="menu p-4 w-80 min-h-full bg-base-100 text-base-content">
              <li v-if="authStore.isAuthenticated()" class="py-1">
                <RouterLink to="/my-calendars" active-class="active">{{ $t('app.myCalendars') }}</RouterLink>
              </li>
              <li v-if="authStore.isAuthenticated()" class="py-1">
                <RouterLink to="/account" active-class="active">{{ $t('app.myAccount') }}</RouterLink>
              </li>
              <li v-if="authStore.isAuthenticated()" class="mt-auto py-1">
                <button @click="handleLogout" class="w-full text-left">{{ $t('app.logout') }}</button>
              </li>
            </ul>
          </div>
        </div>
        
        <!-- Desktop menu items -->
        <ul class="menu menu-horizontal px-1 md:flex hidden gap-1">
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/my-calendars" active-class="active">{{ $t('app.myCalendars') }}</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/account" active-class="active">{{ $t('app.myAccount') }}</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <button @click="handleLogout">{{ $t('app.logout') }}</button>
          </li>
        </ul>
      </div>
    </div>
    <main class="container mx-auto p-4">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
}

.active {
  background-color: var(--fallback-bc,oklch(70% 0 0));
  color: var(--fallback-bc,oklch(0% 0 0));
  border-radius: 0.5rem;
  font-weight: bold;
  width: 100%;
  box-sizing: border-box;
}

.menu a {
  border-radius: 0.5rem;
  width: 100%;
  box-sizing: border-box;
}
</style>
