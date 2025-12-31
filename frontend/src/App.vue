<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { onMounted } from 'vue'
import { useAuthStore } from './stores/auth'

const authStore = useAuthStore()
</script>

<template>
  <div class="min-h-screen bg-base-200">
    <div class="navbar bg-base-100 shadow">
      <div class="flex-1">
        <RouterLink to="/my-calendars" class="btn btn-ghost text-xl">Anime Calendar</RouterLink>
      </div>
      <div class="flex-none">
        <ul class="menu menu-horizontal px-1">
          <li>
            <RouterLink to="/my-calendars" active-class="active">My Calendars</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/user/details" active-class="active">My Account</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/login" @click="authStore.logout" active-class="active">Logout</RouterLink>
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
}
</style>
