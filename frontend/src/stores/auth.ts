import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import axios from 'axios'
import api from '../config/api'
import { invalidateSettingsCache } from '@/services/userSettingsService'

export const useAuthStore = defineStore('auth', () => {
  const user = ref('')
  const userId = ref<number | null>(null)

  let initPromise: Promise<void> | null = null
  let initialized = false

  const isAuthenticated = () => user.value !== ''

  /**
   * Rehydrate auth state from the server on page load.
   * Calls GET /user; on success populates user.value, on failure clears it.
   * Idempotent — subsequent calls return immediately.
   * Concurrent calls return the same in-flight Promise.
   */
  const initAuth = async (): Promise<void> => {
    if (initialized) return
    if (initPromise) return initPromise

    initPromise = (async () => {
      try {
        const response = await api.get('/user')
        user.value = response.data.username ?? ''
        userId.value = response.data.user_id ?? null
      } catch {
        user.value = ''
        userId.value = null
      } finally {
        initialized = true
        initPromise = null
      }
    })()

    return initPromise
  }

  const login = async (email: string, password: string) => {
    try {
      const response = await api.post('/login', { email, password })
      user.value = response.data.username
      userId.value = response.data.user_id
      // A previous session that ended without an explicit logout (cookie
      // expiry, 401 redirect) leaves the old user's settings cached in
      // localStorage for up to 30 minutes — never serve them to this login.
      invalidateSettingsCache()
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Login failed')
      }
      throw err
    }
  }

  const register = async (username: string, email: string, password: string) => {
    try {
      const response = await api.post('/register', { username, email, password })
      // No cookie is issued — user must verify email before logging in.
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Registration failed')
      }
      throw err
    }
  }

  const verifyEmail = async (token: string): Promise<string> => {
    try {
      const response = await api.post('/auth/verify-email', { token })
      // Backend issues the auth cookie; set local state
      user.value = response.data.username
      userId.value = response.data.user_id
      // See login(): never serve a previous session's cached settings.
      invalidateSettingsCache()
      return response.data.username as string
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Verification failed')
      }
      throw err
    }
  }

  const oauthLogin = async (provider: 'google', token_string: string) => {
    try {
      const response = await api.post(`/auth/${provider}`, { token: token_string })
      const { username: userData, avatar: avatarUrl, user_id: userIdValue } = response.data
      user.value = userData
      userId.value = userIdValue
      // Non-sensitive display data only — no auth token in localStorage.
      // ProfileTab reads these directly from localStorage for OAuth users
      // (whose /user/details username is empty), so they are not mirrored
      // into store refs.
      localStorage.setItem('name', userData)
      localStorage.setItem('avatar', avatarUrl)
      // See login(): never serve a previous session's cached settings.
      invalidateSettingsCache()
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || `${provider} login failed`)
      }
      throw err
    }
  }

  const logout = async (push?: (path: string) => unknown) => {
    try {
      await api.post('/auth/logout')
    } catch {
      // Clear local state regardless of server response
    }
    user.value = ''
    userId.value = null
    initialized = false
    localStorage.removeItem('name')
    localStorage.removeItem('avatar')
    invalidateSettingsCache()
    // Optional `push` injection lets tests pass a spy without mocking
    // vue-router (which leaks across vitest workers — see memory
    // feedback_vue_router_mock_leaks_across_workers). Production callers
    // pass no argument and fall through to useRouter().push exactly as
    // before. Resolved lazily so seed-then-mount test flows don't warn.
    const navigate = push ?? useRouter().push
    navigate('/login')
  }

  return {
    user,
    userId,
    isAuthenticated,
    login,
    register,
    verifyEmail,
    oauthLogin,
    logout,
    initAuth,
  }
})
