<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
import HomePage from '@/components/HomePage.vue'
import CalendarPage from './components/CalendarPage.vue';
</script>

<template>
  <nav>
    <RouterLink to="/">Home</RouterLink>
    <RouterLink to="/calendar">Calendar</RouterLink>
  </nav>
  <main>
    <RouterView />
  </main>
</template>

<style scoped>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
}

nav {
  padding: 30px;
}

nav a {
  font-weight: bold;
  color: #2c3e50;
  margin-right: 10px;
  text-decoration: none;
}

nav a.router-link-exact-active {
  color: #42b983;
}
</style>
