import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'

// Lazy load components to improve performance
const MyCalendarsPage = () => import('@/components/MyCalendarsPage.vue')
const CalendarPage = () => import('@/components/CalendarPage.vue')
const LoginPage = () => import('@/components/LoginPage.vue')
const UserDetailsPage = () => import('@/components/UserDetailsPage.vue')
const UserSettingsPage = () => import('@/components/UserSettingsPage.vue')

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
      component: LoginPage,
      meta: { public: true }
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
    },
    {
      path: '/user/settings',
      name: 'UserSettings',
      component: UserSettingsPage,
      meta: { requiresAuth: true }
    },
    {
      path: '/forgot-password',
      name: 'ForgotPassword',
      component: () => import('@/components/ForgotPasswordPage.vue'),
      meta: { public: true }
    },
    {
      path: '/reset-password',
      name: 'ResetPassword',
      component: () => import('@/components/ResetPasswordPage.vue'),
      meta: { public: true }
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'NotFound',
      component: () => import('@/components/NotFoundPage.vue')
    }
  ]
})

// Navigation guard to protect routes
router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore()
  const userSettingsStore = useUserSettingsStore()

  // Rehydrate auth state from the server cookie on first navigation.
  // initAuth() is idempotent — subsequent navigations return immediately.
  await authStore.initAuth()

  // Fetch settings on authenticated routes only
  if (!to.meta.public) {
    const settings = await userSettingsStore.fetchSettings()
    if (settings) {
      applySettings(settings)
    }
  }

  if (to.path === '/login' && authStore.isAuthenticated()) {
    // If user is already logged in and tries to access /login, redirect to /my-calendars
    next('/my-calendars')
  } else if (to.meta.requiresAuth && !authStore.isAuthenticated()) {
    // If route requires auth and user is not authenticated, redirect to /login
    next('/login')
  } else {
    next()
  }
})

export default router
