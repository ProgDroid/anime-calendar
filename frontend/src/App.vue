<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import { onMounted } from 'vue'
import { useAuthStore } from './stores/auth'

const authStore = useAuthStore()

onMounted(async () => {
  await authStore.initAuth()
})
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
            <RouterLink to="/my-calendars">My Calendars</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/user/details">My Account</RouterLink>
          </li>
          <li v-if="authStore.isAuthenticated()">
            <RouterLink to="/login" @click="authStore.logout">Logout</RouterLink>
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
</style>
