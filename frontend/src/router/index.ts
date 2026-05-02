import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useUserSettingsStore } from '@/stores/userSettingsStore'
import { applySettings } from '@/services/applySettings'

// Lazy load components to improve performance
const MyCalendarsPage = () => import('@/components/MyCalendarsPage.vue')
const CalendarPage = () => import('@/components/CalendarPage.vue')
const LoginPage = () => import('@/components/LoginPage.vue')

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
      component: CalendarPage,
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'calendar.editor',
          component: () => import('@/components/calendar/CalendarEditorView.vue')
        },
        {
          path: 'schedule',
          name: 'calendar.schedule',
          component: () => import('@/components/calendar/CalendarScheduleView.vue')
        }
      ]
    },
    {
      path: '/account',
      component: () => import('@/components/AccountPage.vue'),
      meta: { requiresAuth: true },
      children: [
        { path: '', redirect: { name: 'account.profile' } },
        { path: 'profile', name: 'account.profile', component: () => import('@/components/account/ProfileTab.vue') },
        { path: 'preferences', name: 'account.preferences', component: () => import('@/components/account/PreferencesTab.vue') },
        { path: 'password', name: 'account.password', component: () => import('@/components/account/PasswordTab.vue') },
        { path: 'danger', name: 'account.danger', component: () => import('@/components/account/DangerZoneTab.vue') }
      ]
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
      path: '/register',
      name: 'Register',
      component: () => import('@/components/Register.vue'),
      meta: { public: true }
    },
    {
      path: '/verify-email/pending',
      name: 'VerifyEmailPending',
      component: () => import('@/components/VerifyEmailPendingPage.vue'),
      meta: { public: true }
    },
    {
      path: '/verify-email',
      name: 'VerifyEmailConfirm',
      component: () => import('@/components/VerifyEmailConfirmPage.vue'),
      meta: { public: true }
    },
    {
      path: '/upgrade',
      name: 'Upgrade',
      component: () => import('@/components/UpgradePage.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/upgrade/success',
      name: 'UpgradeSuccess',
      component: () => import('@/components/UpgradeSuccessPage.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/upgrade/canceled',
      name: 'UpgradeCanceled',
      component: () => import('@/components/UpgradeCanceledPage.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'NotFound',
      component: () => import('@/components/NotFoundPage.vue'),
      meta: { public: true }
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
