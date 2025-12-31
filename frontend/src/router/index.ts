import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

// Lazy load components to improve performance
const MyCalendarsPage = () => import('@/components/MyCalendarsPage.vue')
const CalendarPage = () => import('@/components/CalendarPage.vue')
const LoginPage = () => import('@/components/LoginPage.vue')
const UserDetailsPage = () => import('@/components/UserDetailsPage.vue')

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/my-calendars'
    },
    {
      path: '/login',
      name: 'Login',
      component: LoginPage
    },
    {
      path: '/my-calendars',
      name: 'MyCalendars',
      component: MyCalendarsPage,
      meta: { requiresAuth: true }
    },
    {
      path: '/calendar/:id',
      name: 'CalendarDetail',
      component: CalendarPage,
      meta: { requiresAuth: true }
    },
    {
      path: '/user/details',
      name: 'UserDetails',
      component: UserDetailsPage,
      meta: { requiresAuth: true }
    }
  ]
})

// Navigation guard to protect routes
router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore()
  
  if (to.meta.requiresAuth && !authStore.isAuthenticated()) {
    next('/login')
  } else {
    next()
  }
})

export default router
