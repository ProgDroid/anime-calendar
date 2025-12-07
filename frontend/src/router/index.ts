import { createRouter, createWebHistory } from 'vue-router'
import HomePage from '@/components/HomePage.vue'
import CalendarPage from '@/components/CalendarPage.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'Home',
      component: HomePage
    },
    {
      path: '/calendar',
      name: 'Calendar',
      component: CalendarPage
    }
  ]
})

export default router
